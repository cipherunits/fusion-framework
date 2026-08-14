use pyo3::prelude::*;

#[pyfunction]
#[pyo3(signature = (name="Fusion"))]
fn hello(name: &str) -> String {
    fusion_security_mod::hello(name)
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello, m)?)?;
    Ok(())
}
