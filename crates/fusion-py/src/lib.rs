use std::sync::Mutex;

use bytes::Bytes;
use fusion_core::{App as CoreApp, Handler, Request, Response};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

struct PyHandler {
    callback: PyObject,
}

impl Handler for PyHandler {
    fn call(&self, req: Request) -> Response {
        Python::with_gil(|py| match invoke_handler(py, &self.callback, req) {
            Ok(response) => response,
            Err(err) => {
                let message = err.to_string();
                Response::text(500, format!("handler error: {message}"))
            }
        })
    }
}

fn invoke_handler(py: Python<'_>, callback: &PyObject, req: Request) -> PyResult<Response> {
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
    response_from_py(py, result)
}

fn response_from_py(py: Python<'_>, value: PyObject) -> PyResult<Response> {
    if let Ok(dict) = value.downcast_bound::<PyDict>(py) {
        let status: u16 = match dict.get_item("status")? {
            Some(v) => v.extract()?,
            None => 200,
        };

        let body = match dict.get_item("body")? {
            Some(v) => {
                if let Ok(s) = v.extract::<String>() {
                    Bytes::from(s)
                } else if let Ok(b) = v.extract::<Vec<u8>>() {
                    Bytes::from(b)
                } else {
                    Bytes::from(v.str()?.to_string())
                }
            }
            None => Bytes::new(),
        };

        let mut response = Response::new(status, body);

        if let Some(headers) = dict.get_item("headers")? {
            let headers = headers.downcast::<PyDict>()?;
            for (key, val) in headers.iter() {
                let name: String = key.extract()?;
                let value: String = val.extract()?;
                response.headers.push((name, value));
            }
        }

        return Ok(response);
    }

    if let Ok(s) = value.extract::<String>(py) {
        return Ok(Response::text(200, s));
    }

    let as_any = value.bind(py);
    Ok(Response::text(200, as_any.str()?.to_string()))
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
        let app = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("app lock poisoned"))?;
            guard.clone()
        };

        let host = host.to_string();

        let result = py.allow_threads(|| {
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| format!("tokio runtime: {e}"))?;
            runtime
                .block_on(async move { app.listen_host_port(&host, port).await })
                .map_err(|e| e.to_string())
        });

        result.map_err(PyRuntimeError::new_err)
    }
}

#[pymodule]
fn _fusion(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyApp>()?;
    Ok(())
}
