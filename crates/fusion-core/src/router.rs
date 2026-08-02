use std::collections::HashMap;
use std::sync::Arc;

use crate::handler::Handler;
use crate::request::Request;
use crate::response::Response;

#[derive(Debug, Clone)]
enum Segment {
    Static(String),
    Param(String),
}

#[derive(Clone)]
struct Route {
    method: String,
    segments: Vec<Segment>,
    handler: Arc<dyn Handler>,
}

#[derive(Clone, Default)]
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn route(&mut self, method: &str, path: &str, handler: impl Handler + 'static) {
        self.routes.push(Route {
            method: method.to_uppercase(),
            segments: parse_pattern(path),
            handler: Arc::new(handler),
        });
    }

    pub fn dispatch(&self, mut req: Request) -> Response {
        let method = req.method.to_uppercase();
        let path_segments = split_path(&req.path);

        for route in &self.routes {
            if route.method != method {
                continue;
            }
            if let Some(params) = match_segments(&route.segments, &path_segments) {
                req.params = params;
                return route.handler.call(req);
            }
        }

        Response::not_found()
    }
}

fn split_path(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

/// Supports both `{id}` and `[name]` parameter segments.
fn parse_pattern(path: &str) -> Vec<Segment> {
    split_path(path)
        .into_iter()
        .map(|seg| {
            if let Some(name) = param_name(seg) {
                Segment::Param(name.to_string())
            } else {
                Segment::Static(seg.to_string())
            }
        })
        .collect()
}

fn param_name(seg: &str) -> Option<&str> {
    if seg.starts_with('{') && seg.ends_with('}') && seg.len() > 2 {
        Some(&seg[1..seg.len() - 1])
    } else if seg.starts_with('[') && seg.ends_with(']') && seg.len() > 2 {
        Some(&seg[1..seg.len() - 1])
    } else {
        None
    }
}

fn match_segments(pattern: &[Segment], path: &[&str]) -> Option<HashMap<String, String>> {
    if pattern.len() != path.len() {
        return None;
    }

    let mut params = HashMap::new();
    for (seg, value) in pattern.iter().zip(path.iter()) {
        match seg {
            Segment::Static(expected) => {
                if expected != value {
                    return None;
                }
            }
            Segment::Param(name) => {
                params.insert(name.clone(), (*value).to_string());
            }
        }
    }
    Some(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::Response;

    #[test]
    fn matches_mixed_param_styles() {
        let mut router = Router::new();
        router.route("GET", "/api/[name]/{id}", |req: Request| {
            let name = req.params.get("name").cloned().unwrap_or_default();
            let id = req.params.get("id").cloned().unwrap_or_default();
            Response::text(200, format!("{name}:{id}"))
        });

        let req = Request::new("GET", "/api/alice/42", vec![], bytes::Bytes::new());
        let res = router.dispatch(req);
        assert_eq!(res.status, 200);
        assert_eq!(res.body.as_ref(), b"alice:42");
    }
}
