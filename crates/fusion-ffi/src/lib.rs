//! Thin C ABI over [`fusion_core`] for managed bindings (C#, etc.).
//!
//! Strings crossing the boundary are UTF-8, NUL-terminated, and owned by
//! whoever allocated them. Caller-owned returns from this library must be
//! freed with [`fusion_string_free`]. Handler callbacks must return strings
//! allocated with [`fusion_string_dup`] (or any pointer freeable the same way).

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::ptr;

use fusion_core::{
    App, HTTP_HEADER_CONSTANTS, HTTP_METHODS, HTTP_STATUS_CODES, Handler, Request, Response,
    Settings, api_resource_name, attachment, cache_control, content_type, download, inline,
    location, render_template, resolve_route_path, response_from_value,
};
use serde_json::{Map, Value};
use std::time::Duration;

/// Opaque application handle.
pub struct FusionAppHandle {
    app: Option<App>,
}

/// Opaque settings handle.
pub struct FusionSettingsHandle {
    settings: Settings,
}

/// Handler callback: returns a newly allocated UTF-8 response JSON string
/// (envelope or plain JSON). Free with [`fusion_string_free`].
pub type FusionHandlerFn = extern "C" fn(
    user_data: *mut c_void,
    method: *const c_char,
    path: *const c_char,
    headers_json: *const c_char,
    body: *const c_char,
    params_json: *const c_char,
    query_json: *const c_char,
    state_json: *const c_char,
) -> *mut c_char;

fn cstr_to_str<'a>(p: *const c_char) -> &'a str {
    if p.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("")
}

fn to_cstring(s: &str) -> *mut c_char {
    CString::new(s.replace('\0', ""))
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

fn parse_json_object(raw: &str) -> Map<String, Value> {
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

fn headers_to_json(headers: &[(String, String)]) -> String {
    let mut map = Map::new();
    for (k, v) in headers {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(map).to_string()
}

fn map_to_json_string(map: &std::collections::HashMap<String, String>) -> String {
    let mut obj = Map::new();
    for (k, v) in map {
        obj.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(obj).to_string()
}

fn state_to_json(state: &std::collections::HashMap<String, Value>) -> String {
    let mut obj = Map::new();
    for (k, v) in state {
        obj.insert(k.clone(), v.clone());
    }
    Value::Object(obj).to_string()
}

struct FfiHandler {
    cb: FusionHandlerFn,
    user_data: usize, // pointer as usize for Send+Sync
}

unsafe impl Send for FfiHandler {}
unsafe impl Sync for FfiHandler {}

impl Handler for FfiHandler {
    fn call(&self, req: Request) -> fusion_core::HandlerFuture {
        let cb = self.cb;
        let user_data = self.user_data;
        Box::pin(async move {
            let method = CString::new(req.method.replace('\0', "")).unwrap_or_default();
            let path = CString::new(req.path.replace('\0', "")).unwrap_or_default();
            let headers = CString::new(headers_to_json(&req.headers)).unwrap_or_default();
            let body = CString::new(req.body_str().replace('\0', "")).unwrap_or_default();
            let params = CString::new(map_to_json_string(&req.params)).unwrap_or_default();
            let query = CString::new(map_to_json_string(&req.query)).unwrap_or_default();
            let state = CString::new(state_to_json(&req.state)).unwrap_or_default();

            let raw = cb(
                user_data as *mut c_void,
                method.as_ptr(),
                path.as_ptr(),
                headers.as_ptr(),
                body.as_ptr(),
                params.as_ptr(),
                query.as_ptr(),
                state.as_ptr(),
            );

            if raw.is_null() {
                return Response::text(500, "null handler response");
            }
            let json = unsafe { CStr::from_ptr(raw) }
                .to_str()
                .unwrap_or("")
                .to_owned();
            fusion_string_free(raw);

            match serde_json::from_str::<Value>(&json) {
                Ok(value) => response_from_value(value),
                Err(_) => Response::text(200, json),
            }
        })
    }
}

// ─── Strings ─────────────────────────────────────────────────────────────────

/// Duplicate a UTF-8 C string. Result must be freed with [`fusion_string_free`].
#[unsafe(no_mangle)]
pub extern "C" fn fusion_string_dup(s: *const c_char) -> *mut c_char {
    to_cstring(cstr_to_str(s))
}

/// Free a string allocated by this library ([`fusion_string_dup`] or returned APIs).
#[unsafe(no_mangle)]
pub extern "C" fn fusion_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(s));
    }
}

// ─── App ─────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn fusion_app_new() -> *mut FusionAppHandle {
    Box::into_raw(Box::new(FusionAppHandle {
        app: Some(App::new()),
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_app_free(app: *mut FusionAppHandle) {
    if app.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(app));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_app_set_settings(
    app: *mut FusionAppHandle,
    settings: *const FusionSettingsHandle,
) -> c_int {
    if app.is_null() || settings.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *app };
    let Some(inner) = handle.app.as_mut() else {
        return -1;
    };
    let settings = unsafe { &*settings };
    *inner.settings_mut() = settings.settings.clone();
    0
}

/// Register a route. `user_data` is passed to `handler` on each request.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_app_route(
    app: *mut FusionAppHandle,
    method: *const c_char,
    path: *const c_char,
    handler: Option<FusionHandlerFn>,
    user_data: *mut c_void,
) -> c_int {
    let Some(handler) = handler else {
        return -1;
    };
    if app.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *app };
    let Some(inner) = handle.app.as_mut() else {
        return -1;
    };
    let method = cstr_to_str(method);
    let path = cstr_to_str(path);
    if method.is_empty() || path.is_empty() {
        return -1;
    }
    inner.route(
        method,
        path,
        FfiHandler {
            cb: handler,
            user_data: user_data as usize,
        },
    );
    0
}

/// Blocking listen. Consumes the app; further use is invalid (returns error).
#[unsafe(no_mangle)]
pub extern "C" fn fusion_app_listen(
    app: *mut FusionAppHandle,
    host: *const c_char,
    port: u16,
) -> c_int {
    if app.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *app };
    let Some(inner) = handle.app.take() else {
        return -1;
    };
    let host = {
        let h = cstr_to_str(host);
        if h.is_empty() {
            inner.settings().host()
        } else {
            h.to_string()
        }
    };
    let port = if port == 0 {
        inner.settings().port()
    } else {
        port
    };

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return -1,
    };
    match rt.block_on(inner.listen_host_port(&host, port)) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_app_listen_from_settings(app: *mut FusionAppHandle) -> c_int {
    fusion_app_listen(app, ptr::null(), 0)
}

// ─── Settings ────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_new() -> *mut FusionSettingsHandle {
    Box::into_raw(Box::new(FusionSettingsHandle {
        settings: Settings::new(),
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_free(settings: *mut FusionSettingsHandle) {
    if settings.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(settings));
    }
}

/// `extra_roots_json` is a JSON array of path strings, or null/empty.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_ensure_loaded(
    settings: *mut FusionSettingsHandle,
    extra_roots_json: *const c_char,
) -> c_int {
    if settings.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *settings };
    let roots = parse_path_list(cstr_to_str(extra_roots_json));
    match handle.settings.ensure_loaded(&roots) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_load_json(
    settings: *mut FusionSettingsHandle,
    path: *const c_char,
    env: *const c_char,
    extra_roots_json: *const c_char,
) -> c_int {
    if settings.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *settings };
    let path_opt = {
        let p = cstr_to_str(path);
        if p.is_empty() {
            None
        } else {
            Some(PathBuf::from(p))
        }
    };
    let env_opt = {
        let e = cstr_to_str(env);
        if e.is_empty() { None } else { Some(e) }
    };
    let roots = parse_path_list(cstr_to_str(extra_roots_json));
    match handle
        .settings
        .load_json(path_opt.as_deref(), env_opt, &roots)
    {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Merge a JSON object into settings.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_merge(
    settings: *mut FusionSettingsHandle,
    json_object: *const c_char,
) -> c_int {
    if settings.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *settings };
    let map = parse_json_object(cstr_to_str(json_object));
    handle.settings.merge_map(map);
    0
}

/// Return JSON encoding of the value, or default JSON, or "null". Free with fusion_string_free.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_get(
    settings: *const FusionSettingsHandle,
    key: *const c_char,
    default_json: *const c_char,
) -> *mut c_char {
    if settings.is_null() {
        return to_cstring("null");
    }
    let handle = unsafe { &*settings };
    let key = cstr_to_str(key);
    match handle.settings.get(key) {
        Some(v) => to_cstring(&v.to_string()),
        None => {
            let d = cstr_to_str(default_json);
            if d.is_empty() {
                to_cstring("null")
            } else {
                to_cstring(d)
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_host(settings: *const FusionSettingsHandle) -> *mut c_char {
    if settings.is_null() {
        return to_cstring("127.0.0.1");
    }
    to_cstring(&unsafe { &*settings }.settings.host())
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_port(settings: *const FusionSettingsHandle) -> u16 {
    if settings.is_null() {
        return 3000;
    }
    unsafe { &*settings }.settings.port()
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_debug(settings: *const FusionSettingsHandle) -> c_int {
    if settings.is_null() {
        return 0;
    }
    if unsafe { &*settings }.settings.debug() {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_settings_env(settings: *const FusionSettingsHandle) -> *mut c_char {
    if settings.is_null() {
        return to_cstring("dev");
    }
    to_cstring(unsafe { &*settings }.settings.env())
}

fn parse_path_list(raw: &str) -> Vec<PathBuf> {
    if raw.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(PathBuf::from))
            .collect(),
        _ => Vec::new(),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn fusion_resolve_route_path(
    template: *const c_char,
    class_name: *const c_char,
) -> *mut c_char {
    to_cstring(&resolve_route_path(
        cstr_to_str(template),
        cstr_to_str(class_name),
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_api_resource_name(class_name: *const c_char) -> *mut c_char {
    to_cstring(&api_resource_name(cstr_to_str(class_name)))
}

/// JSON array of HTTP method names.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_http_methods() -> *mut c_char {
    to_cstring(&serde_json::to_string(HTTP_METHODS).unwrap_or_else(|_| "[]".into()))
}

/// JSON object `{ "HTTP_SUCCESS": 200, ... }`.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_http_status_codes() -> *mut c_char {
    let mut map = Map::new();
    for (name, code) in HTTP_STATUS_CODES {
        map.insert((*name).to_string(), Value::from(*code));
    }
    to_cstring(&Value::Object(map).to_string())
}

/// JSON object `{ "CONTENT_TYPE": "Content-Type", "APPLICATION_JSON": "application/json", ... }`.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_http_header_constants() -> *mut c_char {
    let mut map = Map::new();
    for (name, value) in HTTP_HEADER_CONSTANTS {
        map.insert((*name).to_string(), Value::String((*value).to_string()));
    }
    to_cstring(&Value::Object(map).to_string())
}

fn headers_map_json(map: &std::collections::BTreeMap<String, String>) -> *mut c_char {
    let mut obj = Map::new();
    for (k, v) in map {
        obj.insert(k.clone(), Value::String(v.clone()));
    }
    to_cstring(&Value::Object(obj).to_string())
}

/// JSON map for `Content-Disposition: attachment; filename=...`.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_header_attachment(filename: *const c_char) -> *mut c_char {
    headers_map_json(&attachment(cstr_to_str(filename)))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_header_inline(filename: *const c_char) -> *mut c_char {
    let raw = cstr_to_str(filename);
    let opt = if raw.is_empty() { None } else { Some(raw) };
    headers_map_json(&inline(opt))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_header_content_type(
    media_type: *const c_char,
    charset: *const c_char,
) -> *mut c_char {
    let cs = cstr_to_str(charset);
    let charset = if cs.is_empty() { None } else { Some(cs) };
    headers_map_json(&content_type(cstr_to_str(media_type), charset))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_header_location(url: *const c_char) -> *mut c_char {
    headers_map_json(&location(cstr_to_str(url)))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_header_cache_control(value: *const c_char) -> *mut c_char {
    headers_map_json(&cache_control(cstr_to_str(value)))
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_header_download(
    filename: *const c_char,
    media_type: *const c_char,
) -> *mut c_char {
    let mt = cstr_to_str(media_type);
    let media = if mt.is_empty() { None } else { Some(mt) };
    headers_map_json(&download(cstr_to_str(filename), media))
}

/// JSON map of default Fusion identity headers.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_fingerprint_headers() -> *mut c_char {
    headers_map_json(&fusion_core::fingerprint_headers())
}

/// Render a Tera template. `context_json` is a JSON object; `templates_root` may be empty for default `templates`.
/// Returns HTML or null on error (check with fusion_last_error). Free with fusion_string_free.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_render_template(
    template_name: *const c_char,
    context_json: *const c_char,
    templates_root: *const c_char,
) -> *mut c_char {
    let name = cstr_to_str(template_name);
    if name.is_empty() {
        return ptr::null_mut();
    }
    let ctx_raw = cstr_to_str(context_json);
    let context = if ctx_raw.is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(ctx_raw).unwrap_or(Value::Object(Map::new()))
    };
    let root_raw = cstr_to_str(templates_root);
    let root = if root_raw.is_empty() {
        PathBuf::from("templates")
    } else {
        PathBuf::from(root_raw)
    };
    match render_template(name, &context, &root) {
        Ok(html) => to_cstring(&html),
        Err(e) => {
            eprintln!("fusion_render_template: {e}");
            ptr::null_mut()
        }
    }
}

fn parse_json_value(raw: &str) -> Value {
    if raw.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(raw).unwrap_or(Value::Null)
    }
}

fn ttl_opt(ttl_secs: f64) -> Option<Duration> {
    // Use -1.0 to mean "no explicit TTL" (fall back to cache default).
    if ttl_secs < 0.0 {
        None
    } else {
        Some(Duration::from_secs_f64(ttl_secs))
    }
}

fn value_to_cstring(v: &Value) -> *mut c_char {
    to_cstring(&serde_json::to_string(v).unwrap_or_else(|_| "null".into()))
}

/// Configure process-wide cache from a settings handle (`cache.*`).
#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_configure(settings: *const FusionSettingsHandle) -> c_int {
    if settings.is_null() {
        return -1;
    }
    let settings = unsafe { &*settings };
    match fusion_core::cache::configure_from_settings(&settings.settings) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("fusion_cache_configure: {e}");
            -1
        }
    }
}

/// Ensure a default moka cache exists.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_ensure() -> c_int {
    match fusion_core::cache::ensure_configured() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("fusion_cache_ensure: {e}");
            -1
        }
    }
}

/// Store JSON value. `ttl_secs < 0` uses the configured default TTL.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_set(
    key: *const c_char,
    value_json: *const c_char,
    ttl_secs: f64,
) -> c_int {
    let key = cstr_to_str(key);
    if key.is_empty() {
        return -1;
    }
    let value = parse_json_value(cstr_to_str(value_json));
    match fusion_core::cache::set(key, value, ttl_opt(ttl_secs)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("fusion_cache_set: {e}");
            -1
        }
    }
}

/// Get JSON value or null pointer when missing. Free with fusion_string_free.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_get(key: *const c_char) -> *mut c_char {
    let key = cstr_to_str(key);
    match fusion_core::cache::get(key) {
        Ok(Some(v)) => value_to_cstring(&v),
        Ok(None) => ptr::null_mut(),
        Err(e) => {
            eprintln!("fusion_cache_get: {e}");
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_delete(key: *const c_char) -> c_int {
    match fusion_core::cache::delete(cstr_to_str(key)) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(e) => {
            eprintln!("fusion_cache_delete: {e}");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_exists(key: *const c_char) -> c_int {
    match fusion_core::cache::exists(cstr_to_str(key)) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(e) => {
            eprintln!("fusion_cache_exists: {e}");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_get_or_set(
    key: *const c_char,
    default_json: *const c_char,
    ttl_secs: f64,
) -> *mut c_char {
    let key = cstr_to_str(key);
    let default = parse_json_value(cstr_to_str(default_json));
    match fusion_core::cache::get_or_set(key, default, ttl_opt(ttl_secs)) {
        Ok(v) => value_to_cstring(&v),
        Err(e) => {
            eprintln!("fusion_cache_get_or_set: {e}");
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_delete_or_set(
    key: *const c_char,
    value_json: *const c_char,
    ttl_secs: f64,
) -> *mut c_char {
    let key = cstr_to_str(key);
    let value = parse_json_value(cstr_to_str(value_json));
    match fusion_core::cache::delete_or_set(key, value, ttl_opt(ttl_secs)) {
        Ok(v) => value_to_cstring(&v),
        Err(e) => {
            eprintln!("fusion_cache_delete_or_set: {e}");
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_exists_or_set(
    key: *const c_char,
    value_json: *const c_char,
    ttl_secs: f64,
) -> c_int {
    let key = cstr_to_str(key);
    let value = parse_json_value(cstr_to_str(value_json));
    match fusion_core::cache::exists_or_set(key, value, ttl_opt(ttl_secs)) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(e) => {
            eprintln!("fusion_cache_exists_or_set: {e}");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_driver() -> *mut c_char {
    match fusion_core::cache::driver() {
        Ok(d) => to_cstring(&d),
        Err(e) => {
            eprintln!("fusion_cache_driver: {e}");
            ptr::null_mut()
        }
    }
}

/// Clear all entries from the process-wide cache.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_clear() -> c_int {
    match fusion_core::cache::clear() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("fusion_cache_clear: {e}");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_reset() {
    fusion_core::cache::reset_global();
}

/// JSON snapshot of cache entries + recent events. Free with fusion_string_free.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_snapshot() -> *mut c_char {
    match fusion_core::cache::snapshot() {
        Ok(v) => value_to_cstring(&v),
        Err(e) => {
            eprintln!("fusion_cache_snapshot: {e}");
            ptr::null_mut()
        }
    }
}

/// JSON template context for the built-in monitor panel. Free with fusion_string_free.
#[unsafe(no_mangle)]
pub extern "C" fn fusion_cache_panel_context() -> *mut c_char {
    match fusion_core::cache::panel_context() {
        Ok(v) => value_to_cstring(&v),
        Err(e) => {
            eprintln!("fusion_cache_panel_context: {e}");
            ptr::null_mut()
        }
    }
}
