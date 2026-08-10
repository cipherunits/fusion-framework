use bytes::Bytes;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
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
            query: HashMap::new(),
        }
    }

    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = params;
        self
    }

    pub fn with_query(mut self, query: HashMap<String, String>) -> Self {
        self.query = query;
        self
    }

    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Parse an HTTP query string into key/value pairs.
/// Duplicate keys keep the first value; values are percent-decoded best-effort.
pub fn parse_query(raw: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if raw.is_empty() {
        return out;
    }
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        let key = percent_decode(key);
        if key.is_empty() || out.contains_key(&key) {
            continue;
        }
        out.insert(key, percent_decode(value));
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let h = hex_nibble(bytes[i + 1]);
                let l = hex_nibble(bytes[i + 2]);
                if let (Some(h), Some(l)) = (h, l) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_query() {
        let q = parse_query("id=12&name=hello");
        assert_eq!(q.get("id").map(String::as_str), Some("12"));
        assert_eq!(q.get("name").map(String::as_str), Some("hello"));
    }

    #[test]
    fn keeps_first_duplicate_key() {
        let q = parse_query("id=1&id=2");
        assert_eq!(q.get("id").map(String::as_str), Some("1"));
    }

    #[test]
    fn percent_decodes_values() {
        let q = parse_query("q=hello%20world&plus=a+b");
        assert_eq!(q.get("q").map(String::as_str), Some("hello world"));
        assert_eq!(q.get("plus").map(String::as_str), Some("a b"));
    }

    #[test]
    fn empty_and_flag_keys() {
        let q = parse_query("flag&empty=");
        assert_eq!(q.get("flag").map(String::as_str), Some(""));
        assert_eq!(q.get("empty").map(String::as_str), Some(""));
    }
}
