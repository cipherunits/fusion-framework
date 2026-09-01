//! Request view passed to user-defined API handlers.

use std::collections::HashMap;

use serde_json::Value;

use crate::dispatch::build_response;
use crate::request::Request;

/// Language-neutral request context for class-based API handlers.
#[derive(Debug, Clone)]
pub struct ApiContext {
    pub method: String,
    pub path: String,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub state: HashMap<String, Value>,
}

impl ApiContext {
    pub fn from_request(req: &Request) -> Self {
        Self {
            method: req.method.to_ascii_uppercase(),
            path: req.path.clone(),
            body: req.body_str(),
            headers: req.headers.iter().cloned().collect(),
            params: req.params.clone(),
            query: req.query.clone(),
            state: req.state.clone(),
        }
    }

    pub fn response(body: Value, status: u16, headers: HashMap<String, String>) -> Value {
        build_response(body, status, headers)
    }
}
