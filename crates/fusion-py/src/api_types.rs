use std::collections::HashMap;
use std::sync::Mutex;

use fusion_core::{
    HttpError, ParamKind, ParamSpec, Request, bind_args, build_response, resolve_route_path,
    HTTP_METHODS,
};
use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyString, PyType};
use pyo3::PyObject;
use serde_json::Value as JsonValue;

use crate::json::{json_to_py, py_to_json};

#[derive(Debug, Clone, Default)]
struct SwaggerMeta {
    tags: Vec<String>,
    description: Option<String>,
    title: Option<String>,
    deprecated: bool,
}

fn extract_path_params_from_pattern(path: &str) -> Vec<String> {
    // Match fusion-core router pattern tokens: `{id}` or `[module]` style.
    // Note: `[module]` should already be resolved to a static segment by resolve_route_path.
    path.trim_matches('/')
        .split('/')
        .filter_map(|seg| {
            if seg.starts_with('{') && seg.ends_with('}') && seg.len() > 2 {
                Some(seg[1..seg.len() - 1].to_string())
            } else if seg.starts_with('[') && seg.ends_with(']') && seg.len() > 2 {
                Some(seg[1..seg.len() - 1].to_string())
            } else {
                None
            }
        })
        .collect()
}

fn schema_for_param(kind: ParamKind, nullable: bool) -> serde_json::Value {
    use serde_json::json;
    let mut schema = match kind {
        ParamKind::String => json!({ "type": "string" }),
        ParamKind::Int => json!({ "type": "integer", "format": "int64" }),
        ParamKind::Float => json!({ "type": "number" }),
        ParamKind::Bool => json!({ "type": "boolean" }),
        ParamKind::Auto => json!({
            "oneOf": [
                { "type": "integer", "format": "int64" },
                { "type": "number" },
                { "type": "boolean" },
                { "type": "string" }
            ]
        }),
    };
    if nullable {
        if let Some(obj) = schema.as_object_mut() {
            obj.insert("nullable".to_string(), serde_json::Value::Bool(true));
        }
    }
    schema
}

// ─── FusionBaseApi ───────────────────────────────────────────────────────────

#[pyclass(name = "FusionBaseApi", module = "fusion_framework._fusion")]
pub struct PyFusionBaseApi {
    request: Py<PyDict>,
}

#[pymethods]
impl PyFusionBaseApi {
    #[new]
    fn new(request: Py<PyDict>) -> Self {
        Self { request }
    }

    #[getter]
    fn request(slf: PyRef<Self>, py: Python<'_>) -> Py<PyDict> {
        slf.request.clone_ref(py)
    }

    #[getter]
    fn method(&self, py: Python<'_>) -> PyResult<String> {
        let req = self.request.bind(py);
        let raw = req
            .get_item("method")?
            .map(|v| v.extract::<String>())
            .transpose()?
            .unwrap_or_default();
        Ok(raw.to_ascii_uppercase())
    }

    #[getter]
    fn path(&self, py: Python<'_>) -> PyResult<String> {
        let req = self.request.bind(py);
        Ok(req
            .get_item("path")?
            .map(|v| v.extract::<String>())
            .transpose()?
            .unwrap_or_default())
    }

    #[getter]
    fn body(&self, py: Python<'_>) -> PyResult<String> {
        let req = self.request.bind(py);
        Ok(req
            .get_item("body")?
            .map(|v| v.extract::<String>())
            .transpose()?
            .unwrap_or_default())
    }

    #[getter]
    fn headers(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let req = self.request.bind(py);
        match req.get_item("headers")? {
            Some(dict) if dict.is_instance_of::<PyDict>() => {
                Ok(dict.downcast::<PyDict>()?.clone().unbind())
            }
            _ => Ok(PyDict::new(py).unbind()),
        }
    }

    #[getter]
    fn params(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let req = self.request.bind(py);
        match req.get_item("params")? {
            Some(dict) if dict.is_instance_of::<PyDict>() => {
                Ok(dict.downcast::<PyDict>()?.clone().unbind())
            }
            _ => Ok(PyDict::new(py).unbind()),
        }
    }

    #[getter]
    fn query(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let req = self.request.bind(py);
        match req.get_item("query")? {
            Some(dict) if dict.is_instance_of::<PyDict>() => {
                Ok(dict.downcast::<PyDict>()?.clone().unbind())
            }
            _ => Ok(PyDict::new(py).unbind()),
        }
    }

    #[pyo3(signature = (body=None, status=200, **headers))]
    fn response(
        &self,
        py: Python<'_>,
        body: Option<PyObject>,
        status: u16,
        headers: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyDict>> {
        let body_obj = body.unwrap_or_else(|| py.None());
        let json_body = py_to_json(py, body_obj.bind(py))?;

        let mut hdr_map = HashMap::new();
        if let Some(hdrs) = headers {
            for (key, val) in hdrs.iter() {
                hdr_map.insert(key.extract()?, val.extract()?);
            }
        }

        let envelope = build_response(json_body, status, hdr_map);
        envelope_to_py_dict(py, envelope).map(Bound::unbind)
    }
}

// ─── HTTP error helpers ──────────────────────────────────────────────────────

#[pyfunction(name = "http_error_to_response")]
#[pyo3(signature = (status, detail=None, **headers))]
pub fn http_error_to_response(
    py: Python<'_>,
    status: u16,
    detail: Option<PyObject>,
    headers: Option<Bound<'_, PyDict>>,
) -> PyResult<Py<PyDict>> {
    let detail_obj = detail.unwrap_or_else(|| py.None());
    let json_detail = py_to_json(py, detail_obj.bind(py))?;
    let mut hdr_map = HashMap::new();
    if let Some(hdrs) = headers {
        for (k, v) in hdrs.iter() {
            hdr_map.insert(k.extract()?, v.extract()?);
        }
    }
    let envelope = HttpError {
        status,
        detail: json_detail,
        headers: hdr_map,
    }
    .to_envelope();
    envelope_to_py_dict(py, envelope).map(Bound::unbind)
}

pub fn http_exception_to_response(py: Python<'_>, err: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
    let status: u16 = err.getattr("status")?.extract()?;
    let detail = err.getattr("detail")?;
    let headers = err.getattr("headers")?;
    http_error_to_response(
        py,
        status,
        Some(detail.unbind()),
        Some(headers.downcast()?.clone()),
    )
}

pub fn is_http_exception(py: Python<'_>, err: &Bound<'_, PyAny>) -> Option<Py<PyDict>> {
    let http_exc = match py.import("fusion_framework.http") {
        Ok(m) => m.getattr("HTTPException").ok()?,
        Err(_) => return None,
    };
    let is_exc = err.is_instance(&http_exc).ok()?;
    if !is_exc {
        return None;
    }
    http_exception_to_response(py, err).ok()
}

// ─── Route registry ──────────────────────────────────────────────────────────

struct RegisteredRoute {
    path: String,
    api_name: String,
    api_cls: Py<PyType>,
    method_specs: HashMap<String, Vec<ParamSpec>>,
    swagger: SwaggerMeta,
}

static REGISTRY: Mutex<Vec<RegisteredRoute>> = Mutex::new(Vec::new());

pub fn clear_registry() {
    if let Ok(mut guard) = REGISTRY.lock() {
        guard.clear();
    }
}

pub fn register_route(
    template: &str,
    api_cls: Bound<'_, PyType>,
    tags: Vec<String>,
    description: Option<String>,
    title: Option<String>,
    version: Option<String>,
    deprecated: bool,
) -> PyResult<String> {
    let py = api_cls.py();

    let class_name: String = api_cls.name()?.extract()?;
    let mut resolved = resolve_route_path(template, &class_name);
    if let Some(version) = version {
        let v = version.trim().trim_matches('/');
        if !v.is_empty() {
            // Prefix the resolved api path with version: `v1/api/` style.
            let trimmed = resolved.trim_start_matches('/');
            resolved = format!("{}/{}", v, trimmed);
        }
    }

    let mut method_specs = HashMap::new();
    for method_name in HTTP_METHODS {
        if !defines_method(&api_cls, method_name)? {
            continue;
        }
        let method = api_cls.getattr(method_name)?;
        method_specs.insert(
            method_name.to_string(),
            extract_param_specs(py, &method)?,
        );
    }

    api_cls.setattr("__fusion_path__", &resolved)?;
    api_cls.setattr("__fusion_path_template__", template)?;

    REGISTRY
        .lock()
        .map_err(|_| PyRuntimeError::new_err("registry lock poisoned"))?
        .push(RegisteredRoute {
            path: resolved.clone(),
            api_name: class_name.clone(),
            api_cls: api_cls.unbind(),
            method_specs,
            swagger: SwaggerMeta {
                tags,
                description,
                title,
                deprecated,
            },
        });

    Ok(resolved)
}

pub fn mount_routes(app: &super::PyApp) -> PyResult<()> {
    let routes: Vec<(String, Py<PyType>, HashMap<String, Vec<ParamSpec>>)> = Python::with_gil(|py| {
        let guard = REGISTRY
            .lock()
            .map_err(|_| PyRuntimeError::new_err("registry lock poisoned"))?;
        Ok::<_, PyErr>(guard
            .iter()
            .map(|r| {
                (
                    r.path.clone(),
                    r.api_cls.clone_ref(py),
                    r.method_specs.clone(),
                )
            })
            .collect())
    })?;

    for (path, api_cls, method_specs) in routes {
        for method_name in HTTP_METHODS {
            let Some(specs) = method_specs.get(*method_name) else {
                continue;
            };
            let handler = Python::with_gil(|py| -> PyResult<PyObject> {
                let route_handler = RouteHandler {
                    api_cls: api_cls.clone_ref(py),
                    method_name: (*method_name).to_string(),
                    specs: specs.clone(),
                };
                Ok(Py::new(py, route_handler)?.into_any().into())
            })?;

            app.route((*method_name).to_uppercase(), path.clone(), handler)?;
        }
    }
    Ok(())
}

#[pyclass]
struct RouteHandler {
    api_cls: Py<PyType>,
    method_name: String,
    specs: Vec<ParamSpec>,
}

#[pymethods]
impl RouteHandler {
    #[pyo3(name = "__call__")]
    fn call(&self, py: Python<'_>, request: Py<PyDict>) -> PyResult<PyObject> {
        invoke_api_method(py, &self.api_cls, &self.method_name, &self.specs, request)
    }
}

pub fn invoke_api_method(
    py: Python<'_>,
    api_cls: &Py<PyType>,
    method_name: &str,
    specs: &[ParamSpec],
    request: Py<PyDict>,
) -> PyResult<PyObject> {
    let req = build_core_request(py, &request)?;
    let bound_args = match bind_args(specs, &req) {
        Ok(args) => args,
        Err(err) => {
            let dict = envelope_to_py_dict(py, err.to_envelope())?;
            return Ok(dict.into_any().unbind());
        }
    };

    let instance = api_cls.bind(py).call1((request,))?;
    let method = instance.getattr(method_name)?;

    let kwargs = PyDict::new(py);
    for (name, value) in bound_args {
        kwargs.set_item(name, json_to_py(py, &value)?)?;
    }

    let result = match method.call((), Some(&kwargs)) {
        Ok(v) => v.unbind(),
        Err(err) => {
            if let Some(resp) = is_http_exception(py, &err.value(py)) {
                return Ok(resp.into_any());
            }
            return Err(err);
        }
    };

    Ok(result)
}

fn build_core_request(py: Python<'_>, request: &Py<PyDict>) -> PyResult<Request> {
    let dict = request.bind(py);
    let method: String = dict
        .get_item("method")?
        .map(|v| v.extract())
        .transpose()?
        .unwrap_or_default();
    let path: String = dict
        .get_item("path")?
        .map(|v| v.extract())
        .transpose()?
        .unwrap_or_default();
    let body: String = dict
        .get_item("body")?
        .map(|v| v.extract())
        .transpose()?
        .unwrap_or_default();

    let mut headers = Vec::new();
    if let Some(hdrs) = dict.get_item("headers")? {
        if let Ok(map) = hdrs.downcast::<PyDict>() {
            for (k, v) in map.iter() {
                headers.push((k.extract()?, v.extract()?));
            }
        }
    }

    let mut params = HashMap::new();
    if let Some(p) = dict.get_item("params")? {
        if let Ok(map) = p.downcast::<PyDict>() {
            for (k, v) in map.iter() {
                params.insert(k.extract()?, v.extract()?);
            }
        }
    }

    let mut query = HashMap::new();
    if let Some(q) = dict.get_item("query")? {
        if let Ok(map) = q.downcast::<PyDict>() {
            for (k, v) in map.iter() {
                query.insert(k.extract()?, v.extract()?);
            }
        }
    }

    Ok(Request {
        method,
        path,
        headers,
        body: body.into_bytes().into(),
        params,
        query,
    })
}

fn envelope_to_py_dict(py: Python<'_>, envelope: JsonValue) -> PyResult<Bound<'_, PyDict>> {
    let JsonValue::Object(map) = envelope else {
        return Err(PyRuntimeError::new_err("expected object envelope"));
    };
    let dict = PyDict::new(py);
    for (k, v) in map {
        dict.set_item(k, json_to_py(py, &v)?)?;
    }
    Ok(dict)
}

fn defines_method(api_cls: &Bound<'_, PyType>, method_name: &str) -> PyResult<bool> {
    let mro = api_cls.getattr("__mro__")?;
    for base in mro.try_iter()? {
        let base = base?;
        let base_name: Option<String> = base.getattr("__name__")?.extract().ok();
        if base_name.as_deref() == Some("object") {
            break;
        }
        let dict = base.getattr("__dict__")?;
        if dict.contains(method_name)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn extract_param_specs(py: Python<'_>, method: &Bound<'_, PyAny>) -> PyResult<Vec<ParamSpec>> {
    let inspect = py.import("inspect")?;
    let empty = inspect.getattr("Parameter")?.getattr("empty")?;
    let sig = inspect.call_method1("signature", (method,))?;
    let params = sig.getattr("parameters")?;

    let typing = py.import("typing")?;
    let get_origin = typing.getattr("get_origin")?;
    let get_args = typing.getattr("get_args")?;

    // Prefer resolved hints so `from __future__ import annotations` / string
    // annotations like `"Optional[int]"` still detect optional correctly.
    let resolved_hints = typing
        .call_method1("get_type_hints", (method,))
        .ok()
        .and_then(|h| h.downcast::<PyDict>().ok().map(|d| d.to_owned()));

    let mut specs = Vec::new();
    // `inspect.Signature.parameters` is typically a `mappingproxy`,
    // so convert to a real dict for easier iteration.
    let builtins = py.import("builtins")?;
    let dict_ctor = builtins.getattr("dict")?;
    let params_any = dict_ctor.call1((params,))?;
    let params_dict = params_any.downcast::<PyDict>()?;

    for (name, param) in params_dict.iter() {
        let name: String = name.extract()?;
        if name == "self" {
            continue;
        }

        let annotation = if let Some(ref hints) = resolved_hints {
            match hints.get_item(&name)? {
                Some(hint) => hint,
                None => param.getattr("annotation")?,
            }
        } else {
            param.getattr("annotation")?
        };
        let default = param.getattr("default")?;
        let has_default = !default.eq(&empty)?;

        let (optional, kind) = classify_annotation(py, &annotation, &get_origin, &get_args)?;

        specs.push(ParamSpec {
            name,
            kind,
            optional,
            has_default,
        });
    }
    Ok(specs)
}

fn is_none_type(py: Python<'_>, value: &Bound<'_, PyAny>) -> bool {
    // `typing.get_args(Optional[T])` yields `(T, type(None))`, not `None`.
    let none_type = py.None().bind(py).get_type();
    value.is(none_type.as_any()) || value.eq(py.None()).unwrap_or(false)
}

fn is_union_origin(py: Python<'_>, origin: &Bound<'_, PyAny>) -> PyResult<bool> {
    let typing = py.import("typing")?;
    let union = typing.getattr("Union")?;
    if origin.eq(&union)? {
        return Ok(true);
    }
    // PEP 604 unions: `int | None` use `types.UnionType`.
    if let Ok(types_mod) = py.import("types") {
        if let Ok(union_type) = types_mod.getattr("UnionType") {
            if origin.eq(&union_type)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn classify_annotation(
    py: Python<'_>,
    annotation: &Bound<'_, PyAny>,
    get_origin: &Bound<'_, PyAny>,
    get_args: &Bound<'_, PyAny>,
) -> PyResult<(bool, ParamKind)> {
    let inspect = py.import("inspect")?;
    let empty = inspect.getattr("Parameter")?.getattr("empty")?;
    if annotation.eq(&empty)? {
        return Ok((false, ParamKind::String));
    }

    // Bare `None` / `NoneType` annotation → optional string.
    if is_none_type(py, annotation) {
        return Ok((true, ParamKind::String));
    }

    let origin = get_origin.call1((annotation,))?;
    if !origin.is_none() && is_union_origin(py, &origin)? {
        let args = get_args.call1((annotation,))?;
        let args_list: Vec<Bound<'_, PyAny>> = args.try_iter()?.collect::<PyResult<_>>()?;
        let has_none = args_list.iter().any(|a| is_none_type(py, a));
        let non_none: Vec<_> = args_list
            .into_iter()
            .filter(|a| !is_none_type(py, a))
            .collect();
        if has_none && non_none.len() == 1 {
            let kind = annotation_kind(py, &non_none[0])?;
            return Ok((true, kind));
        }
        if has_none && non_none.is_empty() {
            return Ok((true, ParamKind::String));
        }
    }

    Ok((false, annotation_kind(py, annotation)?))
}

fn annotation_kind(py: Python<'_>, annotation: &Bound<'_, PyAny>) -> PyResult<ParamKind> {
    let typing = py.import("typing")?;
    if annotation.eq(typing.getattr("Any")?)? {
        return Ok(ParamKind::String);
    }
    if annotation.is_instance_of::<PyType>() {
        if annotation.is(py.get_type::<PyInt>()) {
            return Ok(ParamKind::Int);
        }
        if annotation.is(py.get_type::<PyFloat>()) {
            return Ok(ParamKind::Float);
        }
        if annotation.is(py.get_type::<PyBool>()) {
            return Ok(ParamKind::Bool);
        }
        if annotation.is(py.get_type::<PyString>()) {
            return Ok(ParamKind::String);
        }
    }

    let name: Option<String> = annotation.getattr("__name__").ok().and_then(|n| n.extract().ok());
    match name.as_deref() {
        Some("int") => Ok(ParamKind::Int),
        Some("float") => Ok(ParamKind::Float),
        Some("bool") => Ok(ParamKind::Bool),
        Some("str") => Ok(ParamKind::String),
        _ => Ok(ParamKind::String),
    }
}

pub fn openapi_spec() -> serde_json::Value {
    use serde_json::{json, Map, Value};
    const OPENAPI_VERSION: &str = "3.0.3";

    let routes_guard = match REGISTRY.lock() {
        Ok(g) => g,
        Err(_) => return json!({ "openapi": OPENAPI_VERSION, "paths": {} }),
    };

    let mut tags_set: std::collections::BTreeSet<String> = Default::default();
    let mut paths: Map<String, Value> = Map::new();

    for r in routes_guard.iter() {
        let resolved_path = if r.path.starts_with('/') {
            r.path.clone()
        } else {
            format!("/{}", r.path)
        };

        let path_params = extract_path_params_from_pattern(&resolved_path);

        let mut methods_obj: Map<String, Value> = Map::new();

        for (method, specs) in &r.method_specs {
            let method_upper = method.to_ascii_uppercase();
            let method_lower = method.to_string();

            let mut operation_params: Vec<Value> = Vec::new();
            let mut body_properties: Map<String, Value> = Map::new();
            let mut body_required: Vec<String> = Vec::new();

            for spec in specs {
                let nullable = spec.optional || spec.has_default;
                let required = !nullable;
                if path_params.iter().any(|p| p == &spec.name) {
                    // OpenAPI normally requires path params, but if the Python
                    // signature marks them Optional / defaulted, expose that in Swagger.
                    operation_params.push(json!({
                        "name": spec.name,
                        "in": "path",
                        "required": required,
                        "schema": schema_for_param(spec.kind, nullable),
                    }));
                } else if method_upper == "POST" || method_upper == "PUT" || method_upper == "PATCH" {
                    body_properties.insert(spec.name.clone(), schema_for_param(spec.kind, nullable));
                    if required {
                        body_required.push(spec.name.clone());
                    }
                } else {
                    operation_params.push(json!({
                        "name": spec.name,
                        "in": "query",
                        "required": required,
                        "schema": schema_for_param(spec.kind, nullable),
                    }));
                }
            }

            let title = r
                .swagger
                .title
                .clone()
                .unwrap_or_else(|| format!("{} {}", r.api_name, method_upper));

            let description = r.swagger.description.clone().unwrap_or_default();
            let deprecated = r.swagger.deprecated;

            let mut op = json!({
                "tags": r.swagger.tags.clone(),
                "summary": title,
                "description": description,
                "operationId": format!("{}_{}", r.api_name, method_lower),
                "deprecated": deprecated,
                "responses": {
                    "200": { "description": "OK" }
                }
            });

            if !operation_params.is_empty() {
                if let Some(obj) = op.as_object_mut() {
                    obj.insert("parameters".to_string(), Value::Array(operation_params));
                }
            }

            if !body_properties.is_empty() {
                let required = if body_required.is_empty() {
                    Vec::<String>::new()
                } else {
                    body_required
                };

                let request_body = json!({
                    "required": required.is_empty() == false,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": body_properties,
                                "required": required,
                            }
                        }
                    }
                });

                if let Some(obj) = op.as_object_mut() {
                    obj.insert("requestBody".to_string(), request_body);
                }

                // Make a best-effort response schema.
                if let Some(obj) = op.as_object_mut() {
                    if let Some(resp) = obj.get_mut("responses").and_then(|v| v.as_object_mut()) {
                        resp.insert("200".to_string(), json!({
                            "description": "OK",
                            "content": { "application/json": { "schema": { "type": "object" } } }
                        }));
                    }
                }
            } else {
                // If we only have query/path parameters, default to JSON response.
                if let Some(obj) = op.as_object_mut() {
                    if let Some(resp) = obj.get_mut("responses").and_then(|v| v.as_object_mut()) {
                        resp.insert("200".to_string(), json!({
                            "description": "OK",
                            "content": { "application/json": { "schema": { "type": "object" } } }
                        }));
                    }
                }
            }

            methods_obj.insert(method_lower, op);
        }

        paths.insert(resolved_path, Value::Object(methods_obj));
        // Collect tag names.
        for t in &r.swagger.tags {
            tags_set.insert(t.clone());
        }
    }

    // Keep it simple: put tags list only if non-empty.
    let tags = if tags_set.is_empty() {
        None
    } else {
        Some(tags_set.into_iter().map(|t| json!({ "name": t })).collect::<Vec<_>>())
    };

    let mut spec = json!({
        "openapi": OPENAPI_VERSION,
        "info": { "title": "fusion-framework", "version": "0.1.0" },
        "paths": paths,
    });
    if let (Some(t), Some(spec_obj)) = (tags, spec.as_object_mut()) {
        spec_obj.insert("tags".to_string(), Value::Array(t));
    }

    spec
}
