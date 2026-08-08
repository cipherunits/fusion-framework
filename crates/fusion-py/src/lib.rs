use std::path::PathBuf;
use std::sync::Mutex;

use bytes::Bytes;
use fusion_core::{
    App as CoreApp, Handler, HandlerFuture, Request, Response, Settings as CoreSettings,
    api_resource_name, coerce_param, param_kind_from_name, resolve_route_path,
    response_from_value, HTTP_METHODS, HTTP_STATUS_CODES,
};
use pyo3::exceptions::{PyKeyError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyDict, PyFloat, PyInt, PyList, PyModule, PyNone, PyString,
};
use serde_json::{Map, Number, Value as JsonValue};

// ─── JSON bridge ────────────────────────────────────────────────────────────

fn py_to_json(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    if value.is_instance_of::<PyNone>() {
        return Ok(JsonValue::Null);
    }
    if value.is_instance_of::<PyBool>() {
        return Ok(JsonValue::Bool(value.extract::<bool>()?));
    }
    if value.is_instance_of::<PyInt>() {
        return Ok(JsonValue::Number(Number::from(value.extract::<i64>()?)));
    }
    if value.is_instance_of::<PyFloat>() {
        let f = value.extract::<f64>()?;
        return Ok(match Number::from_f64(f) {
            Some(n) => JsonValue::Number(n),
            None => JsonValue::Null,
        });
    }
    if value.is_instance_of::<PyString>() {
        return Ok(JsonValue::String(value.extract::<String>()?));
    }
    if let Ok(list) = value.downcast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(py_to_json(py, &item)?);
        }
        return Ok(JsonValue::Array(items));
    }
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut map = Map::new();
        for (key, val) in dict.iter() {
            map.insert(key.extract()?, py_to_json(py, &val)?);
        }
        return Ok(JsonValue::Object(map));
    }
    Ok(JsonValue::String(value.str()?.to_string()))
}

fn json_to_py(py: Python<'_>, value: &JsonValue) -> PyResult<PyObject> {
    Ok(match value {
        JsonValue::Null => py.None(),
        JsonValue::Bool(b) => PyBool::new(py, *b).as_any().clone().unbind(),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py)?.into_any().unbind()
            } else if let Some(u) = n.as_u64() {
                u.into_pyobject(py)?.into_any().unbind()
            } else if let Some(f) = n.as_f64() {
                f.into_pyobject(py)?.into_any().unbind()
            } else {
                py.None()
            }
        }
        JsonValue::String(s) => s.into_pyobject(py)?.into_any().unbind(),
        JsonValue::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            list.into_any().unbind()
        }
        JsonValue::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            dict.into_any().unbind()
        }
    })
}

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
                Err(err) => Response::text(500, format!("handler error: {err}")),
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

    let result = callback.call1(py, (py_req,))?;
    let bound = result.bind(py);

    // Real async: leave awaitables for the shared event loop.
    let inspect = py.import("inspect")?;
    let is_awaitable: bool = inspect
        .call_method1("isawaitable", (bound,))?
        .extract()?;
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
                    Ok(fut.call_method0("result")?.unbind())
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

    fn route(&self, method: &str, path: &str, handler: PyObject) -> PyResult<()> {
        let mut app = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("app lock poisoned"))?;
        app.route(method, path, PyHandler { callback: handler });
        Ok(())
    }

    fn listen(&self, py: Python<'_>, host: &str, port: u16) -> PyResult<()> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

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
                return Err(pyo3::exceptions::PyKeyboardInterrupt::new_err("Interrupted"));
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
    resolve_route_path(template, class_name)
}

#[pyfunction(name = "coerce_param")]
#[pyo3(signature = (raw, kind="auto"))]
fn py_coerce_param(py: Python<'_>, raw: &str, kind: &str) -> PyResult<PyObject> {
    let value = coerce_param(raw, param_kind_from_name(kind));
    json_to_py(py, &value)
}

#[pymodule]
fn _fusion(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyApp>()?;
    m.add_class::<PySettings>()?;
    m.add_function(wrap_pyfunction!(py_api_resource_name, m)?)?;
    m.add_function(wrap_pyfunction!(py_resolve_route_path, m)?)?;
    m.add_function(wrap_pyfunction!(py_coerce_param, m)?)?;
    m.add("HTTP_METHODS", HTTP_METHODS)?;
    add_status_module(m)?;

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
