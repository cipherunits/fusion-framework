//! HTTP error type raised by handlers and dispatch logic.

use std::collections::HashMap;

use serde_json::{Map, Value};

/// Structured HTTP error (equivalent to ``HTTPException`` in host languages).
#[derive(Debug, Clone)]
pub struct HttpError {
    pub status: u16,
    pub detail: Value,
    pub headers: HashMap<String, String>,
}

impl HttpError {
    pub fn new(status: u16, detail: Value) -> Self {
        Self {
            status,
            detail,
            headers: HashMap::new(),
        }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Build the framework response envelope consumed by [`crate::response_from_value`].
    pub fn to_envelope(&self) -> Value {
        let mut headers = self.headers.clone();
        let body = self.detail.clone();

        if !body.is_string() && !body.is_null() {
            headers
                .entry("content-type".into())
                .or_insert_with(|| "application/json".into());
        }

        let mut map = Map::new();
        map.insert("status".into(), Value::from(self.status as u64));
        map.insert("body".into(), body);
        if !headers.is_empty() {
            let header_map: Map<String, Value> = headers
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect();
            map.insert("headers".into(), Value::Object(header_map));
        }
        Value::Object(map)
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Value::String(s) if !s.is_empty() => write!(f, "{s}"),
            _ => write!(f, "HTTP {}", self.status),
        }
    }
}

impl std::error::Error for HttpError {}
