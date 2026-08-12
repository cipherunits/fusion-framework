use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyNone, PyString};
use serde_json::{Map, Number, Value as JsonValue};

pub fn py_to_json(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
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

pub fn json_to_py(py: Python<'_>, value: &JsonValue) -> PyResult<PyObject> {
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
