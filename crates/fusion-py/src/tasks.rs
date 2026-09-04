//! Python bindings for Tokio background tasks.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyAny;

use fusion_core::{reset_tasks, spawn_after_ms, spawn_fn, task_cancel, task_snapshot, task_status};

use crate::json::json_to_py;

#[pyfunction(name = "task_spawn")]
fn py_task_spawn(callback: Bound<'_, PyAny>) -> PyResult<String> {
    if !callback.is_callable() {
        return Err(PyTypeError::new_err(
            "callback must be callable; use tasks.spawn(lambda: work(arg)) not tasks.spawn(work(arg))",
        ));
    }
    let cb = callback.unbind();
    Ok(spawn_fn(move || {
        Python::with_gil(|py| {
            if let Err(e) = cb.bind(py).call0() {
                e.print_and_set_sys_last_vars(py);
            }
        });
    }))
}

/// Spawn a callable after `delay_ms` milliseconds. Returns task id.
#[pyfunction(name = "task_spawn_after")]
fn py_task_spawn_after(
    py: Python<'_>,
    delay_ms: u64,
    callback: Bound<'_, PyAny>,
) -> PyResult<String> {
    if !callback.is_callable() {
        return Err(PyTypeError::new_err(
            "callback must be callable; use tasks.spawn(lambda: work(arg)) not tasks.spawn(work(arg))",
        ));
    }
    let _ = py;
    let cb = callback.unbind();
    Ok(spawn_after_ms(delay_ms, move || {
        Python::with_gil(|py| {
            if let Err(e) = cb.bind(py).call0() {
                e.print_and_set_sys_last_vars(py);
            }
        });
    }))
}

/// Cancel a background task by id.
#[pyfunction(name = "task_cancel")]
fn py_task_cancel(id: &str) -> bool {
    task_cancel(id)
}

/// Return status string for a task id, or None if unknown.
#[pyfunction(name = "task_status")]
fn py_task_status(id: &str) -> Option<String> {
    task_status(id).map(|s| s.as_str().to_string())
}

/// JSON snapshot of tracked background tasks.
#[pyfunction(name = "task_snapshot")]
fn py_task_snapshot(py: Python<'_>) -> PyResult<PyObject> {
    json_to_py(py, &task_snapshot())
}

/// Reset the task registry (tests).
#[pyfunction(name = "task_reset")]
fn py_task_reset() {
    reset_tasks();
}

/// Register task helpers on the `_fusion` native module.
pub fn register_tasks(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_task_spawn, m)?)?;
    m.add_function(wrap_pyfunction!(py_task_spawn_after, m)?)?;
    m.add_function(wrap_pyfunction!(py_task_cancel, m)?)?;
    m.add_function(wrap_pyfunction!(py_task_status, m)?)?;
    m.add_function(wrap_pyfunction!(py_task_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(py_task_reset, m)?)?;
    Ok(())
}
