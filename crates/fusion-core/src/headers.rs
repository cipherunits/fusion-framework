//! HTTP header name constants and value helpers shared by all language bindings.
//!
//! Constants are canonical header *names* (e.g. `CONTENT_TYPE` → `"Content-Type"`).
//! Helpers build values (or name/value maps) that need arguments — downloads,
//! content types, redirects, etc.

use std::collections::BTreeMap;

// ─── Common header names ─────────────────────────────────────────────────────

pub const ACCEPT: &str = "Accept";
pub const ACCEPT_ENCODING: &str = "Accept-Encoding";
pub const ACCEPT_LANGUAGE: &str = "Accept-Language";
pub const AUTHORIZATION: &str = "Authorization";
pub const CACHE_CONTROL: &str = "Cache-Control";
pub const CONNECTION: &str = "Connection";
pub const CONTENT_DISPOSITION: &str = "Content-Disposition";
pub const CONTENT_ENCODING: &str = "Content-Encoding";
pub const CONTENT_LENGTH: &str = "Content-Length";
pub const CONTENT_RANGE: &str = "Content-Range";
pub const CONTENT_TYPE: &str = "Content-Type";
pub const COOKIE: &str = "Cookie";
pub const DATE: &str = "Date";
pub const ETAG: &str = "ETag";
pub const EXPECT: &str = "Expect";
pub const EXPIRES: &str = "Expires";
pub const HOST: &str = "Host";
pub const IF_MATCH: &str = "If-Match";
pub const IF_MODIFIED_SINCE: &str = "If-Modified-Since";
pub const IF_NONE_MATCH: &str = "If-None-Match";
pub const IF_RANGE: &str = "If-Range";
pub const IF_UNMODIFIED_SINCE: &str = "If-Unmodified-Since";
pub const LAST_MODIFIED: &str = "Last-Modified";
pub const LOCATION: &str = "Location";
pub const ORIGIN: &str = "Origin";
pub const PRAGMA: &str = "Pragma";
pub const RANGE: &str = "Range";
pub const REFERER: &str = "Referer";
pub const RETRY_AFTER: &str = "Retry-After";
pub const SERVER: &str = "Server";
pub const SET_COOKIE: &str = "Set-Cookie";
pub const TE: &str = "TE";
pub const TRAILER: &str = "Trailer";
pub const TRANSFER_ENCODING: &str = "Transfer-Encoding";
pub const UPGRADE: &str = "Upgrade";
pub const USER_AGENT: &str = "User-Agent";
pub const VARY: &str = "Vary";
pub const VIA: &str = "Via";
pub const WARNING: &str = "Warning";
pub const WWW_AUTHENTICATE: &str = "WWW-Authenticate";
pub const X_CONTENT_TYPE_OPTIONS: &str = "X-Content-Type-Options";
pub const X_FRAME_OPTIONS: &str = "X-Frame-Options";
pub const X_XSS_PROTECTION: &str = "X-XSS-Protection";
pub const X_REQUESTED_WITH: &str = "X-Requested-With";
pub const X_FORWARDED_FOR: &str = "X-Forwarded-For";
pub const X_FORWARDED_PROTO: &str = "X-Forwarded-Proto";
pub const X_REAL_IP: &str = "X-Real-IP";
pub const X_REQUEST_ID: &str = "X-Request-Id";
pub const X_CORRELATION_ID: &str = "X-Correlation-Id";
pub const STRICT_TRANSPORT_SECURITY: &str = "Strict-Transport-Security";
pub const REFERRER_POLICY: &str = "Referrer-Policy";
pub const PERMISSIONS_POLICY: &str = "Permissions-Policy";
pub const CONTENT_SECURITY_POLICY: &str = "Content-Security-Policy";
pub const CROSS_ORIGIN_OPENER_POLICY: &str = "Cross-Origin-Opener-Policy";
pub const CROSS_ORIGIN_RESOURCE_POLICY: &str = "Cross-Origin-Resource-Policy";
pub const CROSS_ORIGIN_EMBEDDER_POLICY: &str = "Cross-Origin-Embedder-Policy";
pub const ACCESS_CONTROL_ALLOW_ORIGIN: &str = "Access-Control-Allow-Origin";
pub const ACCESS_CONTROL_ALLOW_METHODS: &str = "Access-Control-Allow-Methods";
pub const ACCESS_CONTROL_ALLOW_HEADERS: &str = "Access-Control-Allow-Headers";
pub const ACCESS_CONTROL_ALLOW_CREDENTIALS: &str = "Access-Control-Allow-Credentials";
pub const ACCESS_CONTROL_EXPOSE_HEADERS: &str = "Access-Control-Expose-Headers";
pub const ACCESS_CONTROL_MAX_AGE: &str = "Access-Control-Max-Age";
pub const ACCESS_CONTROL_REQUEST_METHOD: &str = "Access-Control-Request-Method";
pub const ACCESS_CONTROL_REQUEST_HEADERS: &str = "Access-Control-Request-Headers";

/// Classic stack fingerprint header (Wappalyzer / DevTools / similar).
pub const X_POWERED_BY: &str = "X-Powered-By";
/// Short framework id for detectors.
pub const X_FRAMEWORK: &str = "X-Framework";
/// Framework release version.
pub const X_FUSION_VERSION: &str = "X-Fusion-Version";

/// Display name used in ``X-Powered-By``.
pub const FRAMEWORK_POWERED_BY: &str = "Fusion Framework";
/// Short id used in ``X-Framework``.
pub const FRAMEWORK_ID: &str = "Fusion";

/// Current fusion-core package version (bindings share this via helpers).
pub fn framework_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Default identity headers applied to every HTTP response (unless disabled).
pub fn fingerprint_header_pairs() -> [(&'static str, String); 3] {
    [
        (X_POWERED_BY, FRAMEWORK_POWERED_BY.to_string()),
        (X_FRAMEWORK, FRAMEWORK_ID.to_string()),
        (X_FUSION_VERSION, framework_version().to_string()),
    ]
}

/// Insert fingerprint headers if the response does not already set them.
///
/// Names listed in ``suppress`` (case-insensitive) are never re-added — used by
/// ``@delete_header`` / ``[DeleteHeader]`` so wire-level fingerprint does not
/// undo intentional removals.
pub fn apply_fingerprint_headers(headers: &mut Vec<(String, String)>) {
    apply_fingerprint_headers_except(headers, &[]);
}

/// Like [`apply_fingerprint_headers`], but skips names present in ``suppress``.
pub fn apply_fingerprint_headers_except(
    headers: &mut Vec<(String, String)>,
    suppress: &[String],
) {
    for (name, value) in fingerprint_header_pairs() {
        if suppress
            .iter()
            .any(|s| s.eq_ignore_ascii_case(name))
        {
            continue;
        }
        let exists = headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case(name));
        if !exists {
            headers.push((name.to_string(), value));
        }
    }
}

/// Map form of [`fingerprint_header_pairs`] for language bindings / middleware.
pub fn fingerprint_headers() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (k, v) in fingerprint_header_pairs() {
        map.insert(k.to_string(), v);
    }
    map
}

// ─── Common Content-Type values ──────────────────────────────────────────────

pub const APPLICATION_JSON: &str = "application/json";
pub const APPLICATION_JSON_UTF8: &str = "application/json; charset=utf-8";
pub const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";
pub const APPLICATION_PDF: &str = "application/pdf";
pub const APPLICATION_XML: &str = "application/xml";
pub const APPLICATION_FORM_URLENCODED: &str = "application/x-www-form-urlencoded";
pub const MULTIPART_FORM_DATA: &str = "multipart/form-data";
pub const TEXT_PLAIN: &str = "text/plain";
pub const TEXT_PLAIN_UTF8: &str = "text/plain; charset=utf-8";
pub const TEXT_HTML: &str = "text/html";
pub const TEXT_HTML_UTF8: &str = "text/html; charset=utf-8";
pub const TEXT_CSS: &str = "text/css";
pub const TEXT_CSV: &str = "text/csv";
pub const TEXT_JAVASCRIPT: &str = "text/javascript";
pub const IMAGE_PNG: &str = "image/png";
pub const IMAGE_JPEG: &str = "image/jpeg";
pub const IMAGE_GIF: &str = "image/gif";
pub const IMAGE_WEBP: &str = "image/webp";
pub const IMAGE_SVG: &str = "image/svg+xml";

/// All `(CONST_NAME, header_name_or_value)` pairs for language bindings.
/// Includes header names and common media-type values.
pub const HTTP_HEADER_CONSTANTS: &[(&str, &str)] = &[
    ("ACCEPT", ACCEPT),
    ("ACCEPT_ENCODING", ACCEPT_ENCODING),
    ("ACCEPT_LANGUAGE", ACCEPT_LANGUAGE),
    ("AUTHORIZATION", AUTHORIZATION),
    ("CACHE_CONTROL", CACHE_CONTROL),
    ("CONNECTION", CONNECTION),
    ("CONTENT_DISPOSITION", CONTENT_DISPOSITION),
    ("CONTENT_ENCODING", CONTENT_ENCODING),
    ("CONTENT_LENGTH", CONTENT_LENGTH),
    ("CONTENT_RANGE", CONTENT_RANGE),
    ("CONTENT_TYPE", CONTENT_TYPE),
    ("COOKIE", COOKIE),
    ("DATE", DATE),
    ("ETAG", ETAG),
    ("EXPECT", EXPECT),
    ("EXPIRES", EXPIRES),
    ("HOST", HOST),
    ("IF_MATCH", IF_MATCH),
    ("IF_MODIFIED_SINCE", IF_MODIFIED_SINCE),
    ("IF_NONE_MATCH", IF_NONE_MATCH),
    ("IF_RANGE", IF_RANGE),
    ("IF_UNMODIFIED_SINCE", IF_UNMODIFIED_SINCE),
    ("LAST_MODIFIED", LAST_MODIFIED),
    ("LOCATION", LOCATION),
    ("ORIGIN", ORIGIN),
    ("PRAGMA", PRAGMA),
    ("RANGE", RANGE),
    ("REFERER", REFERER),
    ("RETRY_AFTER", RETRY_AFTER),
    ("SERVER", SERVER),
    ("SET_COOKIE", SET_COOKIE),
    ("TE", TE),
    ("TRAILER", TRAILER),
    ("TRANSFER_ENCODING", TRANSFER_ENCODING),
    ("UPGRADE", UPGRADE),
    ("USER_AGENT", USER_AGENT),
    ("VARY", VARY),
    ("VIA", VIA),
    ("WARNING", WARNING),
    ("WWW_AUTHENTICATE", WWW_AUTHENTICATE),
    ("X_CONTENT_TYPE_OPTIONS", X_CONTENT_TYPE_OPTIONS),
    ("X_FRAME_OPTIONS", X_FRAME_OPTIONS),
    ("X_XSS_PROTECTION", X_XSS_PROTECTION),
    ("X_REQUESTED_WITH", X_REQUESTED_WITH),
    ("X_FORWARDED_FOR", X_FORWARDED_FOR),
    ("X_FORWARDED_PROTO", X_FORWARDED_PROTO),
    ("X_REAL_IP", X_REAL_IP),
    ("X_REQUEST_ID", X_REQUEST_ID),
    ("X_CORRELATION_ID", X_CORRELATION_ID),
    ("STRICT_TRANSPORT_SECURITY", STRICT_TRANSPORT_SECURITY),
    ("REFERRER_POLICY", REFERRER_POLICY),
    ("PERMISSIONS_POLICY", PERMISSIONS_POLICY),
    ("CONTENT_SECURITY_POLICY", CONTENT_SECURITY_POLICY),
    ("CROSS_ORIGIN_OPENER_POLICY", CROSS_ORIGIN_OPENER_POLICY),
    ("CROSS_ORIGIN_RESOURCE_POLICY", CROSS_ORIGIN_RESOURCE_POLICY),
    ("CROSS_ORIGIN_EMBEDDER_POLICY", CROSS_ORIGIN_EMBEDDER_POLICY),
    ("ACCESS_CONTROL_ALLOW_ORIGIN", ACCESS_CONTROL_ALLOW_ORIGIN),
    ("ACCESS_CONTROL_ALLOW_METHODS", ACCESS_CONTROL_ALLOW_METHODS),
    ("ACCESS_CONTROL_ALLOW_HEADERS", ACCESS_CONTROL_ALLOW_HEADERS),
    ("ACCESS_CONTROL_ALLOW_CREDENTIALS", ACCESS_CONTROL_ALLOW_CREDENTIALS),
    ("ACCESS_CONTROL_EXPOSE_HEADERS", ACCESS_CONTROL_EXPOSE_HEADERS),
    ("ACCESS_CONTROL_MAX_AGE", ACCESS_CONTROL_MAX_AGE),
    ("ACCESS_CONTROL_REQUEST_METHOD", ACCESS_CONTROL_REQUEST_METHOD),
    ("ACCESS_CONTROL_REQUEST_HEADERS", ACCESS_CONTROL_REQUEST_HEADERS),
    ("X_POWERED_BY", X_POWERED_BY),
    ("X_FRAMEWORK", X_FRAMEWORK),
    ("X_FUSION_VERSION", X_FUSION_VERSION),
    ("FRAMEWORK_POWERED_BY", FRAMEWORK_POWERED_BY),
    ("FRAMEWORK_ID", FRAMEWORK_ID),
    ("APPLICATION_JSON", APPLICATION_JSON),
    ("APPLICATION_JSON_UTF8", APPLICATION_JSON_UTF8),
    ("APPLICATION_OCTET_STREAM", APPLICATION_OCTET_STREAM),
    ("APPLICATION_PDF", APPLICATION_PDF),
    ("APPLICATION_XML", APPLICATION_XML),
    ("APPLICATION_FORM_URLENCODED", APPLICATION_FORM_URLENCODED),
    ("MULTIPART_FORM_DATA", MULTIPART_FORM_DATA),
    ("TEXT_PLAIN", TEXT_PLAIN),
    ("TEXT_PLAIN_UTF8", TEXT_PLAIN_UTF8),
    ("TEXT_HTML", TEXT_HTML),
    ("TEXT_HTML_UTF8", TEXT_HTML_UTF8),
    ("TEXT_CSS", TEXT_CSS),
    ("TEXT_CSV", TEXT_CSV),
    ("TEXT_JAVASCRIPT", TEXT_JAVASCRIPT),
    ("IMAGE_PNG", IMAGE_PNG),
    ("IMAGE_JPEG", IMAGE_JPEG),
    ("IMAGE_GIF", IMAGE_GIF),
    ("IMAGE_WEBP", IMAGE_WEBP),
    ("IMAGE_SVG", IMAGE_SVG),
];

// ─── Value / map helpers ─────────────────────────────────────────────────────

/// Escape a filename for the quoted `filename="..."` parameter.
fn escape_quoted_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '\\' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            // Strip CR/LF to avoid header injection
            '\r' | '\n' => {}
            c => out.push(c),
        }
    }
    out
}

/// RFC 5987 `filename*` encoding (UTF-8 percent-encoding).
fn encode_filename_star(name: &str) -> String {
    let mut out = String::with_capacity(name.len() * 3);
    for b in name.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(*b >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(*b & 0xf) as usize]));
            }
        }
    }
    out
}

fn basename(filename: &str) -> &str {
    filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .trim()
}

/// `Content-Disposition` value for file download: `attachment; filename=...`.
pub fn content_disposition_attachment(filename: &str) -> String {
    let name = basename(filename);
    let quoted = escape_quoted_filename(name);
    let star = encode_filename_star(name);
    format!("attachment; filename=\"{quoted}\"; filename*=UTF-8''{star}")
}

/// `Content-Disposition` value for inline display (optional filename).
pub fn content_disposition_inline(filename: Option<&str>) -> String {
    match filename.map(basename).filter(|s| !s.is_empty()) {
        Some(name) => {
            let quoted = escape_quoted_filename(name);
            let star = encode_filename_star(name);
            format!("inline; filename=\"{quoted}\"; filename*=UTF-8''{star}")
        }
        None => "inline".into(),
    }
}

/// `Content-Type` value, optionally with charset.
pub fn content_type_value(media_type: &str, charset: Option<&str>) -> String {
    match charset.map(str::trim).filter(|s| !s.is_empty()) {
        Some(_) if media_type.to_ascii_lowercase().contains("charset=") => media_type.into(),
        Some(cs) => format!("{media_type}; charset={cs}"),
        None => media_type.into(),
    }
}

/// Single-entry header map helper.
pub fn headers_map(pairs: &[(&str, String)]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (k, v) in pairs {
        map.insert((*k).to_string(), v.clone());
    }
    map
}

/// `{ Content-Disposition: attachment; filename="..." }`
pub fn attachment(filename: &str) -> BTreeMap<String, String> {
    headers_map(&[(CONTENT_DISPOSITION, content_disposition_attachment(filename))])
}

/// `{ Content-Disposition: inline; filename="..." }` (filename optional).
pub fn inline(filename: Option<&str>) -> BTreeMap<String, String> {
    headers_map(&[(CONTENT_DISPOSITION, content_disposition_inline(filename))])
}

/// `{ Content-Type: ... }`
pub fn content_type(media_type: &str, charset: Option<&str>) -> BTreeMap<String, String> {
    headers_map(&[(CONTENT_TYPE, content_type_value(media_type, charset))])
}

/// `{ Location: url }`
pub fn location(url: &str) -> BTreeMap<String, String> {
    headers_map(&[(LOCATION, url.to_string())])
}

/// `{ Cache-Control: value }`
pub fn cache_control(value: &str) -> BTreeMap<String, String> {
    headers_map(&[(CACHE_CONTROL, value.to_string())])
}

/// Download-friendly pair: Content-Type + Content-Disposition attachment.
pub fn download(filename: &str, media_type: Option<&str>) -> BTreeMap<String, String> {
    let mut map = attachment(filename);
    let mt = media_type.unwrap_or(APPLICATION_OCTET_STREAM);
    map.insert(CONTENT_TYPE.to_string(), content_type_value(mt, None));
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_includes_filename() {
        let v = content_disposition_attachment("report.pdf");
        assert!(v.starts_with("attachment;"));
        assert!(v.contains("filename=\"report.pdf\""));
        assert!(v.contains("filename*=UTF-8''report.pdf"));
    }

    #[test]
    fn attachment_strips_path() {
        let v = content_disposition_attachment("/tmp/docs/my file.pdf");
        assert!(v.contains("filename=\"my file.pdf\""));
        assert!(v.contains("filename*=UTF-8''my%20file.pdf"));
    }

    #[test]
    fn download_sets_both_headers() {
        let h = download("a.csv", Some(TEXT_CSV));
        assert_eq!(h.get(CONTENT_TYPE).map(String::as_str), Some(TEXT_CSV));
        assert!(h.get(CONTENT_DISPOSITION).unwrap().contains("attachment"));
    }
}
