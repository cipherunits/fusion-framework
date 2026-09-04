//! Python bindings for the process-wide Fusion cache (default: moka).

use std::time::Duration;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyAny;

use fusion_core::cache::{self, CacheConfig};

use crate::json::{json_to_py, py_to_json};
use crate::PySettings;

fn map_err(e: String) -> PyErr {
    PyRuntimeError::new_err(e)
}

fn ttl_from_secs(ttl: Option<f64>) -> PyResult<Option<Duration>> {
    match ttl {
        None => Ok(None),
        Some(s) if s < 0.0 => Err(PyValueError::new_err("ttl must be >= 0")),
        Some(s) => Ok(Some(Duration::from_secs_f64(s))),
    }
}

/// Configure the global cache from a Settings object (`cache.*` keys).
#[pyfunction(name = "cache_configure")]
fn py_cache_configure(settings: &Bound<'_, PySettings>) -> PyResult<()> {
    let borrowed = settings.borrow();
    let guard = borrowed
        .inner
        .lock()
        .map_err(|_| PyRuntimeError::new_err("settings lock poisoned"))?;
    cache::configure_from_settings(&guard).map_err(map_err)
}
/// Configure the global cache from an explicit driver name (mainly for tests).
#[pyfunction(name = "cache_configure_driver")]
#[pyo3(signature = (driver="moka", max_capacity=None, default_ttl=None))]
fn py_cache_configure_driver(
    driver: &str,
    max_capacity: Option<u64>,
    default_ttl: Option<f64>,
) -> PyResult<()> {
    let mut cfg = CacheConfig::default();
    cfg.driver = driver.to_string();
    if let Some(cap) = max_capacity {
        cfg.max_capacity = cap.max(1);
    }
    if let Some(secs) = default_ttl {
        cfg.default_ttl = Some(Duration::from_secs_f64(secs));
    }
    let instance = cache::Cache::open(cfg).map_err(map_err)?;
    cache::configure(instance);
    Ok(())
}

#[pyfunction(name = "cache_set")]
#[pyo3(signature = (key, value, ttl=None))]
fn py_cache_set(py: Python<'_>, key: &str, value: Bound<'_, PyAny>, ttl: Option<f64>) -> PyResult<()> {
    let json = py_to_json(py, &value)?;
    cache::set(key, json, ttl_from_secs(ttl)?).map_err(map_err)
}

#[pyfunction(name = "cache_get")]
fn py_cache_get(py: Python<'_>, key: &str) -> PyResult<PyObject> {
    match cache::get(key).map_err(map_err)? {
        Some(v) => json_to_py(py, &v),
        None => Ok(py.None()),
    }
}

#[pyfunction(name = "cache_delete")]
fn py_cache_delete(key: &str) -> PyResult<bool> {
    cache::delete(key).map_err(map_err)
}

#[pyfunction(name = "cache_exists")]
fn py_cache_exists(key: &str) -> PyResult<bool> {
    cache::exists(key).map_err(map_err)
}

#[pyfunction(name = "cache_get_or_set")]
#[pyo3(signature = (key, default, ttl=None))]
fn py_cache_get_or_set(
    py: Python<'_>,
    key: &str,
    default: Bound<'_, PyAny>,
    ttl: Option<f64>,
) -> PyResult<PyObject> {
    if cache::exists(key).map_err(map_err)? {
        return py_cache_get(py, key);
    }
    let value = if default.is_callable() {
        default.call0()?
    } else {
        default
    };
    let json = py_to_json(py, &value)?;
    let stored = cache::get_or_set(key, json, ttl_from_secs(ttl)?).map_err(map_err)?;
    json_to_py(py, &stored)
}

#[pyfunction(name = "cache_delete_or_set")]
#[pyo3(signature = (key, value, ttl=None))]
fn py_cache_delete_or_set(
    py: Python<'_>,
    key: &str,
    value: Bound<'_, PyAny>,
    ttl: Option<f64>,
) -> PyResult<PyObject> {
    let json = py_to_json(py, &value)?;
    let stored = cache::delete_or_set(key, json, ttl_from_secs(ttl)?).map_err(map_err)?;
    json_to_py(py, &stored)
}

#[pyfunction(name = "cache_exists_or_set")]
#[pyo3(signature = (key, value, ttl=None))]
fn py_cache_exists_or_set(
    py: Python<'_>,
    key: &str,
    value: Bound<'_, PyAny>,
    ttl: Option<f64>,
) -> PyResult<bool> {
    let json = py_to_json(py, &value)?;
    cache::exists_or_set(key, json, ttl_from_secs(ttl)?).map_err(map_err)
}

#[pyfunction(name = "cache_driver")]
fn py_cache_driver() -> PyResult<String> {
    cache::driver().map_err(map_err)
}

#[pyfunction(name = "cache_clear")]
fn py_cache_clear() -> PyResult<()> {
    cache::clear().map_err(map_err)
}

#[pyfunction(name = "cache_reset")]
fn py_cache_reset() {
    cache::reset_global();
}

#[pyfunction(name = "cache_snapshot")]
fn py_cache_snapshot(py: Python<'_>) -> PyResult<PyObject> {
    let value = cache::snapshot().map_err(map_err)?;
    json_to_py(py, &value)
}

#[pyfunction(name = "cache_panel_context")]
fn py_cache_panel_context(py: Python<'_>) -> PyResult<PyObject> {
    let value = cache::panel_context().map_err(map_err)?;
    json_to_py(py, &value)
}

/// Register cache helpers on the `_fusion` native module.
pub fn register_cache(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_cache_configure, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_configure_driver, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_set, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_get, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_delete, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_exists, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_get_or_set, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_delete_or_set, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_exists_or_set, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_driver, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_clear, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_reset, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(py_cache_panel_context, m)?)?;
    Ok(())
}
