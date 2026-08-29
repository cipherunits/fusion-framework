use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    /// Header names that must not be re-injected by wire-level fingerprinting.
    pub suppress_headers: Vec<String>,
}

impl Response {
    pub fn new(status: u16, body: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
            suppress_headers: Vec::new(),
        }
    }

    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self::new(200, body)
    }

    pub fn not_found() -> Self {
        Self::new(404, "Not Found")
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        let mut res = Self::new(status, Bytes::from(body.into()));
        res.headers
            .push(("content-type".into(), "text/plain; charset=utf-8".into()));
        res
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}
