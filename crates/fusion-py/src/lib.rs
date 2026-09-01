use std::path::PathBuf;
use std::sync::Mutex;

use bytes::Bytes;
use fusion_core::{
    App as CoreApp, HTTP_HEADER_CONSTANTS, HTTP_METHODS, HTTP_STATUS_CODES, Handler, HandlerFuture,
    Request, Response, Settings as CoreSettings, api_resource_name, attachment, cache_control,
    coerce_param, content_type, download, inline, location, param_kind_from_name, render_template,
    response_from_value,
};
use pyo3::exceptions::{PyKeyError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyType};
use serde_json::Value as JsonValue;

mod api_types;
mod json;
mod pagination;

use api_types::{PyFusionBaseApi, clear_registry, mount_routes, register_route};
use json::{json_to_py, py_to_json};
use pagination::register_pagination;

// ─── Settings (core) ─────────────────────────────────────────────────────────

#[pyclass(name = "Settings")]
struct PySettings {
    inner: Mutex<CoreSettings>,
}

impl PySettings {
    fn with_mut<R>(&self, f: impl FnOnce(&mut CoreSettings) -> R) -> PyResult<R> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("settings lock poisoned"))?;
        Ok(f(&mut guard))
    }

    fn with_ref<R>(&self, f: impl FnOnce(&CoreSettings) -> R) -> PyResult<R> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("settings lock poisoned"))?;
        Ok(f(&guard))
    }
}

#[pymethods]
impl PySettings {
    #[new]
    fn new() -> Self {
        Self {
            inner: Mutex::new(CoreSettings::new()),
        }
    }

    #[pyo3(signature = (path=None, env=None, extra_roots=None))]
    fn load_json(
        &self,
        path: Option<String>,
        env: Option<String>,
        extra_roots: Option<Vec<String>>,
    ) -> PyResult<()> {
        let roots: Vec<PathBuf> = extra_roots
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();
        let path_buf = path.map(PathBuf::from);
        self.with_mut(|s| {
            s.load_json(path_buf.as_deref(), env.as_deref(), &roots)
                .map(|_| ())
        })?
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    #[pyo3(signature = (extra_roots=None))]
    fn ensure_loaded(&self, extra_roots: Option<Vec<String>>) -> PyResult<()> {
        let roots: Vec<PathBuf> = extra_roots
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();
        self.with_mut(|s| s.ensure_loaded(&roots).map(|_| ()))?
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn merge(&self, py: Python<'_>, values: Bound<'_, PyDict>) -> PyResult<()> {
        let json = py_to_json(py, values.as_any())?;
        let JsonValue::Object(map) = json else {
            return Err(PyRuntimeError::new_err("merge expects a dict"));
        };
        self.with_mut(|s| s.merge_map(map))?;
        Ok(())
    }

    #[pyo3(signature = (**kwargs))]
    fn configure(&self, py: Python<'_>, kwargs: Option<Bound<'_, PyDict>>) -> PyResult<()> {
        if let Some(kwargs) = kwargs {
            self.merge(py, kwargs)?;
        }
        Ok(())
    }

    fn clear(&self) -> PyResult<()> {
        self.with_mut(|s| s.clear())?;
        Ok(())
    }

    #[pyo3(signature = (key, default=None, *, defualt=None))]
    fn get(
        &self,
        py: Python<'_>,
        key: &str,
        default: Option<PyObject>,
        defualt: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        let fallback = defualt.or(default).unwrap_or_else(|| py.None());
        match self.with_ref(|s| s.get(key))? {
            Some(v) => json_to_py(py, &v),
            None => Ok(fallback),
        }
    }

    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        match self.with_ref(|s| s.get(key))? {
            Some(v) => json_to_py(py, &v),
            None => Err(PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> PyResult<bool> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        self.with_ref(|s| s.contains(key))
    }

    #[getter]
    fn env(&self) -> PyResult<String> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        self.with_ref(|s| s.env().to_string())
    }

    #[getter]
    fn host(&self) -> PyResult<String> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        self.with_ref(|s| s.host())
    }

    #[getter]
    fn port(&self) -> PyResult<u16> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        self.with_ref(|s| s.port())
    }

    #[getter]
    fn debug(&self) -> PyResult<bool> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        self.with_ref(|s| s.debug())
    }

    #[getter]
    fn config(&self, py: Python<'_>) -> PyResult<PyObject> {
        let _ = self.with_mut(|s| s.ensure_loaded(&[]).map(|_| ()))?;
        let map = self.with_ref(|s| JsonValue::Object(s.config().clone()))?;
        json_to_py(py, &map)
    }

    fn __repr__(&self) -> PyResult<String> {
        self.with_ref(|s| {
            let keys: Vec<_> = s.keys().into_iter().collect();
            Ok(format!("<Settings env={:?} keys={keys:?}>", s.env()))
        })?
    }
}

// ─── HTTP handlers ───────────────────────────────────────────────────────────

struct PyHandler {
    callback: PyObject,
}

impl Handler for PyHandler {
    fn call(&self, req: Request) -> HandlerFuture {
        let callback = Python::with_gil(|py| self.callback.clone_ref(py));
        Box::pin(async move {
            match invoke_handler_async(callback, req).await {
                Ok(response) => response,
                Err(err) => {
                    let message = err.to_string();
                    let status = if message.contains("missing path param")
                        || message.contains("missing query param")
                        || message.contains("missing body param")
                    {
                        400
                    } else {
                        500
                    };
                    Response::text(status, format!("handler error: {err}"))
                }
            }
        })
    }
}

enum PyOutcome {
    Ready(Response),
    Pending(PyObject),
}

fn invoke_handler_start(py: Python<'_>, callback: &PyObject, req: Request) -> PyResult<PyOutcome> {
    let body = req.body_str();
    let py_req = PyDict::new(py);
    py_req.set_item("method", req.method)?;
    py_req.set_item("path", req.path)?;
    py_req.set_item("body", body)?;

    let headers = PyDict::new(py);
    for (name, value) in req.headers {
        headers.set_item(name, value)?;
    }
    py_req.set_item("headers", headers)?;

    let params = PyDict::new(py);
    for (name, value) in req.params {
        params.set_item(name, value)?;
    }
    py_req.set_item("params", params)?;

    let query = PyDict::new(py);
    for (name, value) in req.query {
        query.set_item(name, value)?;
    }
    py_req.set_item("query", query)?;

    let state = PyDict::new(py);
    for (name, value) in req.state {
        state.set_item(name, json_to_py(py, &value)?)?;
    }
    py_req.set_item("state", state)?;

    let result = callback.call1(py, (py_req,))?;
    let bound = result.bind(py);

    // Real async: leave awaitables for the shared event loop.
    let inspect = py.import("inspect")?;
    let is_awaitable: bool = inspect.call_method1("isawaitable", (bound,))?.extract()?;
    if is_awaitable {
        return Ok(PyOutcome::Pending(result));
    }

    Ok(PyOutcome::Ready(py_value_to_response(py, bound)?))
}

fn py_value_to_response(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Response> {
    if let Ok(dict) = value.downcast::<PyDict>() {
        if let Some(body) = dict.get_item("body")? {
            if let Ok(bytes) = body.extract::<Vec<u8>>() {
                let status: u16 = match dict.get_item("status")? {
                    Some(v) => v.extract().unwrap_or(200),
                    None => 200,
                };
                let mut response = Response::new(status, Bytes::from(bytes));
                if let Some(headers) = dict.get_item("headers")? {
                    let headers = headers.downcast::<PyDict>()?;
                    for (key, val) in headers.iter() {
                        response.headers.push((key.extract()?, val.extract()?));
                    }
                }
                return Ok(response);
            }
        }
    }

    let json = py_to_json(py, value)?;
    Ok(response_from_value(json))
}

async fn invoke_handler_async(callback: PyObject, req: Request) -> PyResult<Response> {
    let outcome = Python::with_gil(|py| invoke_handler_start(py, &callback, req))?;
    match outcome {
        PyOutcome::Ready(response) => Ok(response),
        PyOutcome::Pending(coro) => {
            // Schedule on the shared asyncio loop (concurrent with other requests),
            // then wait without blocking the tokio worker via spawn_blocking.
            let concurrent = Python::with_gil(|py| -> PyResult<PyObject> {
                let runtime = py.import("fusion_framework.async_runtime")?;
                Ok(runtime.call_method1("submit", (coro,))?.unbind())
            })?;

            let result = tokio::task::spawn_blocking(move || {
                Python::with_gil(|py| -> PyResult<PyObject> {
                    let fut = concurrent.bind(py);
                    match fut.call_method0("result") {
                        Ok(v) => Ok(v.unbind()),
                        Err(err) => {
                            if let Some(resp) = api_types::is_http_exception(py, &err.value(py)) {
                                Ok(resp.into_any())
                            } else {
                                Err(err)
                            }
                        }
                    }
                })
            })
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("async join: {e}")))??;

            Python::with_gil(|py| py_value_to_response(py, result.bind(py)))
        }
    }
}

#[pyclass(name = "App")]
struct PyApp {
    inner: Mutex<CoreApp>,
}

#[pymethods]
impl PyApp {
    #[new]
    fn new() -> Self {
        Self {
            inner: Mutex::new(CoreApp::new()),
        }
    }

    fn route(&self, method: String, path: String, handler: PyObject) -> PyResult<()> {
        let mut app = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("app lock poisoned"))?;
        app.route(&method, &path, PyHandler { callback: handler });
        Ok(())
    }

    fn mount_routes(&self) -> PyResult<()> {
        mount_routes(self)
    }

    fn listen(&self, py: Python<'_>, host: &str, port: u16) -> PyResult<()> {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let app = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("app lock poisoned"))?;
            guard.clone()
        };

        let host = host.to_string();
        let interrupted = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&interrupted))
            .map_err(|e| PyRuntimeError::new_err(format!("signal hook: {e}")))?;
        #[cfg(unix)]
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&interrupted))
            .map_err(|e| PyRuntimeError::new_err(format!("signal hook: {e}")))?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| PyRuntimeError::new_err(format!("tokio runtime: {e}")))?;

        let server = runtime.spawn(async move { app.listen_host_port(&host, port).await });

        loop {
            if interrupted.load(Ordering::SeqCst) {
                server.abort();
                drop(runtime);
                return Err(pyo3::exceptions::PyKeyboardInterrupt::new_err(
                    "Interrupted",
                ));
            }
            if server.is_finished() {
                break;
            }
            py.allow_threads(|| std::thread::sleep(std::time::Duration::from_millis(50)));
            if let Err(err) = py.check_signals() {
                server.abort();
                drop(runtime);
                return Err(err);
            }
        }

        match runtime.block_on(server) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(PyRuntimeError::new_err(e.to_string())),
            Err(e) if e.is_cancelled() => Ok(()),
            Err(e) => Err(PyRuntimeError::new_err(format!("server task: {e}"))),
        }
    }
}

// ─── Pure helpers from core ──────────────────────────────────────────────────

#[pyfunction(name = "api_resource_name")]
fn py_api_resource_name(class_name: &str) -> String {
    api_resource_name(class_name)
}

#[pyfunction(name = "resolve_route_path")]
fn py_resolve_route_path(template: &str, class_name: &str) -> String {
    fusion_core::resolve_route_path(template, class_name)
}

#[pyfunction(name = "register_route")]
#[pyo3(signature = (template, api_cls, tags=Vec::new(), desc=None, title=None, version=None, deprecated=false, middleware=Vec::new()))]
fn py_register_route(
    template: &str,
    api_cls: Bound<'_, PyType>,
    tags: Vec<String>,
    desc: Option<String>,
    title: Option<String>,
    version: Option<String>,
    deprecated: bool,
    middleware: Vec<PyObject>,
) -> PyResult<String> {
    let py = api_cls.py();
    let middleware: Vec<Py<PyAny>> = middleware
        .into_iter()
        .map(|m| m.into_bound(py).into_any().unbind())
        .collect();
    register_route(
        template, api_cls, tags, desc, title, version, deprecated, middleware,
    )
}

#[pyfunction(name = "openapi_spec")]
#[pyo3(signature = (version=None))]
fn py_openapi_spec(py: Python<'_>, version: Option<String>) -> PyResult<PyObject> {
    let spec = match version.as_deref() {
        None => api_types::openapi_spec(),
        Some(v) => api_types::openapi_spec_for(Some(v)),
    };
    json_to_py(py, &spec)
}

#[pyfunction(name = "route_versions")]
fn py_route_versions() -> Vec<String> {
    api_types::route_versions()
}

#[pyfunction(name = "has_unversioned_routes")]
fn py_has_unversioned_routes() -> bool {
    api_types::has_unversioned_routes()
}

#[pyfunction]
fn clear_routes() {
    clear_registry();
}

// Host-language HTTPException response building.
#[pyfunction]
fn http_error_to_response(
    py: Python<'_>,
    status: u16,
    detail: Option<PyObject>,
    headers: Option<Bound<'_, PyDict>>,
) -> PyResult<Py<PyDict>> {
    api_types::http_error_to_response(py, status, detail, headers)
}

#[pyfunction(name = "coerce_param")]
#[pyo3(signature = (raw, kind="auto"))]
fn py_coerce_param(py: Python<'_>, raw: &str, kind: &str) -> PyResult<PyObject> {
    let value = coerce_param(raw, param_kind_from_name(kind));
    json_to_py(py, &value)
}

#[pyfunction(name = "render_template")]
#[pyo3(signature = (template_name, context=None, templates_root=None))]
fn py_render_template(
    py: Python<'_>,
    template_name: &str,
    context: Option<Bound<'_, PyDict>>,
    templates_root: Option<&str>,
) -> PyResult<String> {
    use std::path::PathBuf;

    let ctx = match context {
        Some(d) => py_to_json(py, d.as_any())?,
        None => serde_json::Value::Object(serde_json::Map::new()),
    };
    let root = PathBuf::from(templates_root.unwrap_or("templates"));
    render_template(template_name, &ctx, &root)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
}

#[pyfunction(name = "prefers_json")]
#[pyo3(signature = (accept=None, format_query=None))]
fn py_prefers_json(accept: Option<&str>, format_query: Option<&str>) -> bool {
    fusion_core::prefers_json(accept, format_query)
}

#[pymodule]
fn _fusion(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyApp>()?;
    m.add_class::<PySettings>()?;
    m.add_class::<PyFusionBaseApi>()?;
    m.add_function(wrap_pyfunction!(py_api_resource_name, m)?)?;
    m.add_function(wrap_pyfunction!(py_resolve_route_path, m)?)?;
    m.add_function(wrap_pyfunction!(py_coerce_param, m)?)?;
    m.add_function(wrap_pyfunction!(py_render_template, m)?)?;
    m.add_function(wrap_pyfunction!(py_prefers_json, m)?)?;
    m.add_function(wrap_pyfunction!(py_register_route, m)?)?;
    m.add_function(wrap_pyfunction!(clear_routes, m)?)?;
    m.add_function(wrap_pyfunction!(http_error_to_response, m)?)?;
    m.add_function(wrap_pyfunction!(py_openapi_spec, m)?)?;
    m.add_function(wrap_pyfunction!(py_route_versions, m)?)?;
    m.add_function(wrap_pyfunction!(py_has_unversioned_routes, m)?)?;
    register_pagination(m)?;
    m.add("HTTP_METHODS", HTTP_METHODS)?;
    add_status_module(m)?;
    add_header_module(m)?;

    // Global settings singleton — shared JSON/env logic from fusion-core.
    m.add(
        "settings",
        Py::new(
            m.py(),
            PySettings {
                inner: Mutex::new(CoreSettings::new()),
            },
        )?,
    )?;
    Ok(())
}

fn add_status_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let status = PyModule::new(py, "status")?;
    let mut names = Vec::with_capacity(HTTP_STATUS_CODES.len());
    for &(name, code) in HTTP_STATUS_CODES {
        status.add(name, code)?;
        names.push(name);
    }
    status.setattr("__all__", names)?;
    parent.add_submodule(&status)?;
    parent.setattr("status", &status)?;

    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item("fusion_framework._fusion.status", &status)?;
    Ok(())
}

fn btree_to_pydict(
    py: Python<'_>,
    map: &std::collections::BTreeMap<String, String>,
) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    for (k, v) in map {
        dict.set_item(k, v)?;
    }
    Ok(dict.unbind())
}

#[pyfunction(name = "attachment")]
#[pyo3(signature = (filename))]
fn header_attachment(py: Python<'_>, filename: &str) -> PyResult<Py<PyDict>> {
    btree_to_pydict(py, &attachment(filename))
}

#[pyfunction(name = "inline")]
#[pyo3(signature = (filename=None))]
fn header_inline(py: Python<'_>, filename: Option<&str>) -> PyResult<Py<PyDict>> {
    btree_to_pydict(py, &inline(filename))
}

#[pyfunction(name = "content_type")]
#[pyo3(signature = (media_type, charset=None))]
fn header_content_type(
    py: Python<'_>,
    media_type: &str,
    charset: Option<&str>,
) -> PyResult<Py<PyDict>> {
    btree_to_pydict(py, &content_type(media_type, charset))
}

#[pyfunction(name = "location")]
#[pyo3(signature = (url))]
fn header_location(py: Python<'_>, url: &str) -> PyResult<Py<PyDict>> {
    btree_to_pydict(py, &location(url))
}

#[pyfunction(name = "cache_control")]
#[pyo3(signature = (value))]
fn header_cache_control(py: Python<'_>, value: &str) -> PyResult<Py<PyDict>> {
    btree_to_pydict(py, &cache_control(value))
}

#[pyfunction(name = "download")]
#[pyo3(signature = (filename, media_type=None))]
fn header_download(
    py: Python<'_>,
    filename: &str,
    media_type: Option<&str>,
) -> PyResult<Py<PyDict>> {
    btree_to_pydict(py, &download(filename, media_type))
}

#[pyfunction(name = "fingerprint_headers")]
fn py_fingerprint_headers(py: Python<'_>) -> PyResult<Py<PyDict>> {
    btree_to_pydict(py, &fusion_core::fingerprint_headers())
}

fn add_header_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let header = PyModule::new(py, "header")?;
    let mut names = Vec::with_capacity(HTTP_HEADER_CONSTANTS.len() + 6);
    for &(name, value) in HTTP_HEADER_CONSTANTS {
        header.add(name, value)?;
        names.push(name);
    }
    header.add_function(wrap_pyfunction!(header_attachment, &header)?)?;
    header.add_function(wrap_pyfunction!(header_inline, &header)?)?;
    header.add_function(wrap_pyfunction!(header_content_type, &header)?)?;
    header.add_function(wrap_pyfunction!(header_location, &header)?)?;
    header.add_function(wrap_pyfunction!(header_cache_control, &header)?)?;
    header.add_function(wrap_pyfunction!(header_download, &header)?)?;
    names.extend([
        "attachment",
        "inline",
        "content_type",
        "location",
        "cache_control",
        "download",
    ]);
    header.setattr("__all__", names)?;
    parent.add_submodule(&header)?;
    parent.setattr("header", &header)?;

    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item("fusion_framework._fusion.header", &header)?;

    // Top-level helper used by default middleware
    parent.add_function(wrap_pyfunction!(py_fingerprint_headers, parent)?)?;
    Ok(())
}
