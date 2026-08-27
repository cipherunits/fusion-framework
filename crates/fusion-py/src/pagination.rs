use std::collections::HashMap;

use fusion_core::{
    PageConfig, PageParams, paginated_body, parse_page_params, HttpError,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::json::{json_to_py, py_to_json};

fn http_error_to_py(py: Python<'_>, err: HttpError) -> PyResult<PyErr> {
    let http = py.import("fusion_framework.http")?;
    let exc = http.getattr("HTTPException")?;
    let detail = json_to_py(py, &err.detail)?;
    let inst = exc.call1((err.status, detail))?;
    Ok(PyErr::from_value(inst))
}

#[pyclass(name = "PaginationParams")]
#[derive(Clone, Copy)]
pub struct PyPaginationParams {
    inner: PageParams,
}

#[pymethods]
impl PyPaginationParams {
    #[getter]
    fn page(&self) -> u64 {
        self.inner.page
    }

    #[getter]
    fn page_size(&self) -> u64 {
        self.inner.page_size
    }

    #[getter]
    fn offset(&self) -> u64 {
        self.inner.offset
    }

    #[getter]
    fn limit(&self) -> u64 {
        self.inner.limit()
    }

    fn total_pages(&self, total: u64) -> u64 {
        PageParams::total_pages(total, self.inner.page_size)
    }

    fn has_next(&self, total: u64) -> bool {
        self.inner.has_next(total)
    }

    fn has_prev(&self) -> bool {
        self.inner.has_prev()
    }

    fn __repr__(&self) -> String {
        format!(
            "PaginationParams(page={}, page_size={}, offset={})",
            self.inner.page, self.inner.page_size, self.inner.offset
        )
    }
}

fn query_dict_to_map(query: &Bound<'_, PyDict>) -> PyResult<HashMap<String, String>> {
    let mut map = HashMap::new();
    for (key, value) in query.iter() {
        let k: String = key.extract()?;
        let v: String = if value.is_none() {
            continue;
        } else {
            value.extract()?
        };
        map.insert(k, v);
    }
    Ok(map)
}

#[pyfunction]
#[pyo3(name = "parse_pagination")]
#[pyo3(signature = (query, *, default_page_size=20, max_page_size=100))]
pub fn py_parse_pagination(
    py: Python<'_>,
    query: &Bound<'_, PyDict>,
    default_page_size: u64,
    max_page_size: u64,
) -> PyResult<PyPaginationParams> {
    let map = query_dict_to_map(query)?;
    let config = PageConfig {
        default_page_size,
        max_page_size,
    };
    let inner = match parse_page_params(&map, &config) {
        Ok(params) => params,
        Err(err) => return Err(http_error_to_py(py, err)?),
    };
    Ok(PyPaginationParams { inner })
}

#[pyfunction]
#[pyo3(name = "paginated_body")]
pub fn py_paginated_body(
    py: Python<'_>,
    items: Bound<'_, PyAny>,
    total: u64,
    params: &PyPaginationParams,
) -> PyResult<PyObject> {
    let items_json = py_to_json(py, &items)?;
    let body = paginated_body(items_json, total, &params.inner);
    json_to_py(py, &body)
}

pub fn register_pagination(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPaginationParams>()?;
    m.add_function(wrap_pyfunction!(py_parse_pagination, m)?)?;
    m.add_function(wrap_pyfunction!(py_paginated_body, m)?)?;
    Ok(())
}
