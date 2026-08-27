//! Shared pagination parsing and response envelope for list APIs.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::HttpError;

/// Defaults applied when query params are missing or invalid.
#[derive(Debug, Clone, Copy)]
pub struct PageConfig {
    pub default_page_size: u64,
    pub max_page_size: u64,
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            default_page_size: 20,
            max_page_size: 100,
        }
    }
}

/// Normalized pagination state for a list request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageParams {
    /// 1-based page index.
    pub page: u64,
    pub page_size: u64,
    /// Item offset (0-based). May differ from `(page - 1) * page_size` when `offset` query is set.
    pub offset: u64,
}

impl PageParams {
    pub fn limit(&self) -> u64 {
        self.page_size
    }

    pub fn total_pages(total: u64, page_size: u64) -> u64 {
        if page_size == 0 {
            return 0;
        }
        total.div_ceil(page_size)
    }

    pub fn has_next(&self, total: u64) -> bool {
        self.offset.saturating_add(self.page_size) < total
    }

    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
}

fn query_u64(query: &HashMap<String, String>, key: &str) -> Option<u64> {
    query
        .get(key)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
}

fn first_query_u64(query: &HashMap<String, String>, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| query_u64(query, key))
}

fn bad_request(message: impl Into<String>) -> HttpError {
    HttpError::new(400, Value::String(message.into()))
}

/// Parse `page`, `page_size` / `per_page` / `limit`, and optional `offset` from query params.
pub fn parse_page_params(
    query: &HashMap<String, String>,
    config: &PageConfig,
) -> Result<PageParams, HttpError> {
    let page = match query_u64(query, "page") {
        None => 1,
        Some(0) => return Err(bad_request("page must be >= 1")),
        Some(p) => p,
    };

    let raw_size = first_query_u64(query, &["page_size", "per_page", "limit"]);
    let page_size = match raw_size {
        None => config.default_page_size,
        Some(0) => return Err(bad_request("page_size must be >= 1")),
        Some(n) => n.min(config.max_page_size),
    };

    let offset = match query_u64(query, "offset") {
        None => page.saturating_sub(1).saturating_mul(page_size),
        Some(o) => o,
    };

    Ok(PageParams {
        page,
        page_size,
        offset,
    })
}

/// Build the standard paginated JSON body: `{ items, pagination: { ... } }`.
pub fn paginated_body(items: Value, total: u64, params: &PageParams) -> Value {
    let total_pages = PageParams::total_pages(total, params.page_size);
    let pagination = Map::from_iter([
        ("page".into(), Value::from(params.page)),
        ("page_size".into(), Value::from(params.page_size)),
        ("offset".into(), Value::from(params.offset)),
        ("limit".into(), Value::from(params.limit())),
        ("total".into(), Value::from(total)),
        ("total_pages".into(), Value::from(total_pages)),
        ("has_next".into(), Value::Bool(params.has_next(total))),
        ("has_prev".into(), Value::Bool(params.has_prev())),
    ]);
    Map::from_iter([
        ("items".into(), items),
        ("pagination".into(), Value::Object(pagination)),
    ])
    .into()
}

/// Slice an in-memory collection using normalized pagination bounds.
pub fn paginate_slice<T: Clone>(items: &[T], params: &PageParams) -> Vec<T> {
    let len = items.len() as u64;
    if len == 0 || params.offset >= len {
        return Vec::new();
    }
    let start = params.offset as usize;
    let end = params
        .offset
        .saturating_add(params.page_size)
        .min(len) as usize;
    items[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn defaults_page_and_size() {
        let params = parse_page_params(&q(&[]), &PageConfig::default()).unwrap();
        assert_eq!(
            params,
            PageParams {
                page: 1,
                page_size: 20,
                offset: 0,
            }
        );
    }

    #[test]
    fn parses_page_and_page_size() {
        let params =
            parse_page_params(&q(&[("page", "3"), ("page_size", "10")]), &PageConfig::default())
                .unwrap();
        assert_eq!(
            params,
            PageParams {
                page: 3,
                page_size: 10,
                offset: 20,
            }
        );
    }

    #[test]
    fn per_page_and_limit_aliases() {
        let from_per_page =
            parse_page_params(&q(&[("per_page", "5")]), &PageConfig::default()).unwrap();
        assert_eq!(from_per_page.page_size, 5);

        let from_limit = parse_page_params(&q(&[("limit", "15")]), &PageConfig::default()).unwrap();
        assert_eq!(from_limit.page_size, 15);
    }

    #[test]
    fn explicit_offset_overrides_page_math() {
        let params = parse_page_params(
            &q(&[("page", "2"), ("page_size", "10"), ("offset", "50")]),
            &PageConfig::default(),
        )
        .unwrap();
        assert_eq!(params.offset, 50);
        assert_eq!(params.page, 2);
    }

    #[test]
    fn clamps_to_max_page_size() {
        let params = parse_page_params(
            &q(&[("page_size", "500")]),
            &PageConfig {
                default_page_size: 20,
                max_page_size: 100,
            },
        )
        .unwrap();
        assert_eq!(params.page_size, 100);
    }

    #[test]
    fn rejects_invalid_page() {
        assert!(parse_page_params(&q(&[("page", "0")]), &PageConfig::default()).is_err());
    }

    #[test]
    fn paginated_body_shape() {
        let params = PageParams {
            page: 2,
            page_size: 10,
            offset: 10,
        };
        let body = paginated_body(Value::Array(vec![Value::from(1)]), 25, &params);
        let obj = body.as_object().unwrap();
        assert!(obj.contains_key("items"));
        let meta = obj.get("pagination").unwrap().as_object().unwrap();
        assert_eq!(meta.get("total").and_then(Value::as_u64), Some(25));
        assert_eq!(meta.get("total_pages").and_then(Value::as_u64), Some(3));
        assert_eq!(meta.get("has_next").and_then(Value::as_bool), Some(true));
        assert_eq!(meta.get("has_prev").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn paginate_slice_bounds() {
        let items: Vec<i32> = (0..25).collect();
        let params = PageParams {
            page: 3,
            page_size: 10,
            offset: 20,
        };
        let slice = paginate_slice(&items, &params);
        assert_eq!(slice, vec![20, 21, 22, 23, 24]);
    }
}
