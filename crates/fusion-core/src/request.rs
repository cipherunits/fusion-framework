use bytes::Bytes;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    pub params: HashMap<String, String>,
}

impl Request {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        headers: Vec<(String, String)>,
        body: Bytes,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            headers,
            body,
            params: HashMap::new(),
        }
    }

    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = params;
        self
    }

    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}
