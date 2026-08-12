//! Shared handler argument binding for all language bindings.

use std::collections::HashMap;

use serde_json::{Map, Number, Value};

use crate::coerce::{ParamKind, coerce_param};
use crate::http_error::HttpError;
use crate::request::Request;

/// HTTP methods that read parameters from a JSON request body.
pub const BODY_METHODS: &[&str] = &["POST", "PUT", "PATCH"];

/// Metadata for one handler parameter, registered at mount time by bindings.
#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub name: String,
    pub kind: ParamKind,
    pub optional: bool,
    pub has_default: bool,
}

/// Parse a JSON object from a request body string.
pub fn parse_json_object(body: &str) -> Option<Map<String, Value>> {
    let text = body.trim();
    if text.is_empty() {
        return None;
    }
    let parsed: Value = serde_json::from_str(text).ok()?;
    match parsed {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

/// Coerce a raw value (path/query string or JSON scalar) to the target kind.
pub fn coerce_value(raw: &Value, kind: ParamKind) -> Result<Value, HttpError> {
    match kind {
        ParamKind::Int => {
            if raw.as_bool().is_some() {
                return Err(HttpError::new(
                    400,
                    Value::Object(Map::from_iter([(
                        "detail".into(),
                        Value::String("expected int, got bool".into()),
                    )])),
                ));
            }
            if let Some(n) = raw.as_i64() {
                return Ok(Value::Number(Number::from(n)));
            }
            if let Some(n) = raw.as_u64() {
                return Ok(Value::Number(Number::from(n)));
            }
            if let Some(s) = raw.as_str() {
                return Ok(coerce_param(s, ParamKind::Int));
            }
            Ok(coerce_param(&raw.to_string(), ParamKind::Int))
        }
        ParamKind::Float => {
            if raw.as_bool().is_some() {
                return Err(HttpError::new(
                    400,
                    Value::Object(Map::from_iter([(
                        "detail".into(),
                        Value::String("expected float, got bool".into()),
                    )])),
                ));
            }
            if let Some(n) = raw.as_f64() {
                return Ok(Value::Number(
                    Number::from_f64(n).unwrap_or_else(|| Number::from(0)),
                ));
            }
            if let Some(n) = raw.as_i64() {
                return Ok(Value::Number(Number::from(n)));
            }
            if let Some(s) = raw.as_str() {
                return Ok(coerce_param(s, ParamKind::Float));
            }
            Ok(coerce_param(&raw.to_string(), ParamKind::Float))
        }
        ParamKind::Bool => {
            if let Some(b) = raw.as_bool() {
                return Ok(Value::Bool(b));
            }
            if let Some(s) = raw.as_str() {
                return Ok(coerce_param(s, ParamKind::Bool));
            }
            Ok(coerce_param(&raw.to_string(), ParamKind::Bool))
        }
        ParamKind::String | ParamKind::Auto => {
            if raw.is_null() {
                return Ok(Value::String(String::new()));
            }
            if let Some(s) = raw.as_str() {
                return Ok(Value::String(s.to_string()));
            }
            Ok(Value::String(raw.to_string()))
        }
    }
}

/// Bind handler arguments: path first, then query (read) or JSON body (write).
pub fn bind_args(
    specs: &[ParamSpec],
    req: &Request,
) -> Result<HashMap<String, Value>, HttpError> {
    let http_method = req.method.to_ascii_uppercase();
    let body_fields = if BODY_METHODS.contains(&http_method.as_str()) {
        parse_json_object(&req.body_str())
    } else {
        None
    };

    let mut out = HashMap::new();

    for spec in specs {
        let mut source: Option<&str> = None;
        let mut raw: Option<Value> = None;

        if let Some(value) = req.params.get(&spec.name) {
            source = Some("path");
            raw = Some(Value::String(value.clone()));
        } else if BODY_METHODS.contains(&http_method.as_str()) {
            if let Some(ref fields) = body_fields {
                if let Some(value) = fields.get(&spec.name) {
                    source = Some("body");
                    raw = Some(value.clone());
                }
            }
        } else if let Some(value) = req.query.get(&spec.name) {
            source = Some("query");
            raw = Some(Value::String(value.clone()));
        }

        if source.is_none() {
            if spec.has_default {
                continue;
            }
            // Missing query/body: pass null so the handler can raise HttpError.
            out.insert(spec.name.clone(), Value::Null);
            continue;
        }

        let raw = raw.unwrap_or(Value::Null);
        if raw.is_null() && spec.optional {
            out.insert(spec.name.clone(), Value::Null);
            continue;
        }

        out.insert(spec.name.clone(), coerce_value(&raw, spec.kind)?);
    }

    Ok(out)
}

/// Build a framework response envelope from parts.
pub fn build_response(
    body: Value,
    status: u16,
    headers: HashMap<String, String>,
) -> Value {
    let mut hdrs = headers;
    if !body.is_string() && !body.is_null() {
        hdrs.entry("content-type".into())
            .or_insert_with(|| "application/json".into());
    }

    let mut map = Map::new();
    map.insert("status".into(), Value::from(status as u64));
    map.insert("body".into(), body);
    if !hdrs.is_empty() {
        let header_map: Map<String, Value> = hdrs
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        map.insert("headers".into(), Value::Object(header_map));
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn req(method: &str, params: &[(&str, &str)], query: &[(&str, &str)], body: &str) -> Request {
        Request {
            method: method.into(),
            path: "/".into(),
            headers: vec![],
            body: Bytes::from(body.to_string()),
            params: params
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            query: query
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn binds_path_param() {
        let specs = vec![ParamSpec {
            name: "id".into(),
            kind: ParamKind::Int,
            optional: false,
            has_default: false,
        }];
        let request = req("GET", &[("id", "42")], &[], "");
        let args = bind_args(&specs, &request).unwrap();
        assert_eq!(args.get("id"), Some(&Value::from(42)));
    }

    #[test]
    fn binds_query_param() {
        let specs = vec![ParamSpec {
            name: "q".into(),
            kind: ParamKind::String,
            optional: false,
            has_default: false,
        }];
        let request = req("GET", &[], &[("q", "hello")], "");
        let args = bind_args(&specs, &request).unwrap();
        assert_eq!(
            args.get("q"),
            Some(&Value::String("hello".into()))
        );
    }

    #[test]
    fn binds_body_param() {
        let specs = vec![ParamSpec {
            name: "name".into(),
            kind: ParamKind::String,
            optional: false,
            has_default: false,
        }];
        let request = req("POST", &[], &[], r#"{"name":"alice"}"#);
        let args = bind_args(&specs, &request).unwrap();
        assert_eq!(
            args.get("name"),
            Some(&Value::String("alice".into()))
        );
    }

    #[test]
    fn missing_param_passes_null() {
        let specs = vec![ParamSpec {
            name: "id".into(),
            kind: ParamKind::Int,
            optional: false,
            has_default: false,
        }];
        let request = req("GET", &[], &[], "");
        let args = bind_args(&specs, &request).unwrap();
        assert_eq!(args.get("id"), Some(&Value::Null));
    }

    #[test]
    fn rejects_bool_for_int() {
        let err = coerce_value(&Value::Bool(true), ParamKind::Int).unwrap_err();
        assert_eq!(err.status, 400);
    }
}
