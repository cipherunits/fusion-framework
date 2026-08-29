//! Language-neutral handler return values → [`Response`].
//!
//! Bindings (Python, Node, C#, …) only convert host values into
//! [`serde_json::Value`], then call [`response_from_value`]. All envelope
//! detection and JSON serialization lives here.

use bytes::Bytes;
use serde_json::{Map, Value};

use crate::response::Response;

const CONTENT_TYPE_JSON: &str = "application/json";

/// Build an HTTP response from a JSON-like handler return value.
///
/// Rules (shared across all language bindings):
/// - string → `200` text/plain body
/// - object with `body`, or only `status`/`headers` envelope keys → HTTP envelope
/// - object / array / number / bool / null → `200` application/json
pub fn response_from_value(value: Value) -> Response {
    match value {
        Value::String(s) => Response::text(200, s),
        Value::Object(map) if is_response_envelope(&map) => response_from_envelope(map),
        other => Response::json(200, &other),
    }
}

/// True for framework envelopes like `{"status": 200, "body": "..."}`.
/// Plain data objects (e.g. `{"message": "hello"}`) are not envelopes.
pub fn is_response_envelope(map: &Map<String, Value>) -> bool {
    if map.contains_key("body") {
        return true;
    }

    let mut has_status = false;
    let mut has_headers = false;

    for (key, value) in map {
        match key.as_str() {
            "status" => {
                if !value.is_u64() && !value.is_i64() {
                    return false;
                }
                has_status = true;
            }
            "headers" => {
                if !value.is_object() && !value.is_null() {
                    return false;
                }
                has_headers = true;
            }
            "suppress_headers" => {
                if !value.is_array() && !value.is_null() {
                    return false;
                }
            }
            _ => return false,
        }
    }

    has_status || has_headers
}

fn response_from_envelope(map: Map<String, Value>) -> Response {
    let status = map
        .get("status")
        .and_then(Value::as_u64)
        .or_else(|| map.get("status").and_then(Value::as_i64).map(|n| n as u64))
        .unwrap_or(200)
        .min(u16::MAX as u64) as u16;

    let body = match map.get("body") {
        Some(Value::String(s)) => Bytes::from(s.clone()),
        Some(Value::Null) | None => Bytes::new(),
        Some(other) => Bytes::from(serde_json::to_vec(other).unwrap_or_default()),
    };

    let mut response = Response::new(status, body);

    if let Some(Value::Object(headers)) = map.get("headers") {
        for (name, value) in headers {
            let header_value = match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            response.headers.push((name.clone(), header_value));
        }
    }

    // Optional envelope key used by @delete_header / [DeleteHeader] so wire
    // fingerprint does not re-add intentionally removed identity headers.
    if let Some(Value::Array(names)) = map.get("suppress_headers") {
        for name in names {
            if let Some(s) = name.as_str() {
                if !s.is_empty() {
                    response.suppress_headers.push(s.to_string());
                }
            }
        }
    }

    // Convenience: if the handler returned a framework envelope with a non-string body
    // but did not explicitly set `content-type`, assume JSON.
    //
    // This keeps host-language helpers (Python/Node) thin.
    let has_content_type = response
        .headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
    if !has_content_type {
        match map.get("body") {
            Some(Value::String(_)) | None | Some(Value::Null) => {}
            Some(_) => {
                response
                    .headers
                    .push(("content-type".into(), CONTENT_TYPE_JSON.into()));
            }
        }
    }

    response
}

impl Response {
    /// JSON response with `application/json` content type.
    pub fn json(status: u16, value: &Value) -> Self {
        let body = serde_json::to_vec(value).unwrap_or_else(|_| b"null".to_vec());
        Self::new(status, body).with_header("content-type", CONTENT_TYPE_JSON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_object_becomes_json() {
        let res = response_from_value(json!({"status": 200, "message": "hello"}));
        assert_eq!(res.status, 200);
        assert!(
            res.headers
                .iter()
                .any(|(k, v)| k == "content-type" && v == CONTENT_TYPE_JSON)
        );
        let body = std::str::from_utf8(&res.body).unwrap();
        assert!(body.contains("hello"));
        assert!(body.contains("message"));
    }

    #[test]
    fn envelope_with_body() {
        let res = response_from_value(json!({
            "status": 201,
            "body": "{\"ok\":true}",
            "headers": {"content-type": "application/json"}
        }));
        assert_eq!(res.status, 201);
        assert_eq!(res.body.as_ref(), br#"{"ok":true}"#);
    }

    #[test]
    fn envelope_status_only_empty_body() {
        let res = response_from_value(json!({"status": 204}));
        assert_eq!(res.status, 204);
        assert!(res.body.is_empty());
        assert!(
            !res.headers
                .iter()
                .any(|(k, _)| k == "content-type")
        );
    }

    #[test]
    fn string_is_text() {
        let res = response_from_value(Value::String("hi".into()));
        assert_eq!(res.status, 200);
        assert_eq!(res.body.as_ref(), b"hi");
    }

    #[test]
    fn array_is_json() {
        let res = response_from_value(json!([1, 2, 3]));
        assert_eq!(res.status, 200);
        assert_eq!(res.body.as_ref(), b"[1,2,3]");
    }
}
