mod settings;

use std::sync::Mutex;

use fusion_core::{
    App as CoreApp, HTTP_HEADER_CONSTANTS, HTTP_METHODS, HTTP_STATUS_CODES, Handler, HandlerFuture,
    PageConfig, PageParams, Request, Response, api_resource_name, attachment, cache_control,
    coerce_param, content_type, download, fingerprint_headers, inline, location,
    paginated_body as core_paginated_body, param_kind_from_name, parse_page_params, prefers_json,
    render_template, resolve_route_path, response_from_value,
};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction,
};
use napi::{JsFunction, ValueType};
use napi_derive::napi;
use serde_json::{Map, Number, Value as JsonValue};

pub use settings::Settings;

/// JSON extracted on the Node thread so the async result is `Send`.
pub struct JsJson(pub JsonValue);

impl FromNapiValue for JsJson {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> Result<Self> {
        let unknown = unsafe { Unknown::from_napi_value(env, value)? };
        Ok(JsJson(js_to_json(unknown)?))
    }
}

// ─── JS ↔ JSON bridge ────────────────────────────────────────────────────────

pub(crate) fn js_to_json(value: Unknown) -> Result<JsonValue> {
    match value.get_type()? {
        ValueType::Undefined | ValueType::Null => Ok(JsonValue::Null),
        ValueType::Boolean => Ok(JsonValue::Bool(value.coerce_to_bool()?.get_value()?)),
        ValueType::Number => {
            let n = value.coerce_to_number()?.get_double()?;
            if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                Ok(JsonValue::Number(Number::from(n as i64)))
            } else {
                Ok(match Number::from_f64(n) {
                    Some(num) => JsonValue::Number(num),
                    None => JsonValue::Null,
                })
            }
        }
        ValueType::String => {
            let s = value.coerce_to_string()?.into_utf8()?.into_owned()?;
            Ok(JsonValue::String(s))
        }
        ValueType::Object => {
            let obj = value.coerce_to_object()?;
            if obj.is_array()? {
                let len = obj.get_array_length()?;
                let mut items = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let element: Unknown = obj.get_element(i)?;
                    items.push(js_to_json(element)?);
                }
                return Ok(JsonValue::Array(items));
            }

            let names = obj.get_property_names()?;
            let len = names.get_array_length()?;
            let mut map = Map::new();
            for i in 0..len {
                let name_val: Unknown = names.get_element(i)?;
                let name = name_val.coerce_to_string()?.into_utf8()?.into_owned()?;
                let prop: Unknown = obj.get_named_property(&name)?;
                map.insert(name, js_to_json(prop)?);
            }
            Ok(JsonValue::Object(map))
        }
        _ => {
            let s = value.coerce_to_string()?.into_utf8()?.into_owned()?;
            Ok(JsonValue::String(s))
        }
    }
}

pub(crate) fn json_to_js(env: &Env, value: &JsonValue) -> Result<Unknown> {
    match value {
        JsonValue::Null => Ok(env.get_null()?.into_unknown()),
        JsonValue::Bool(b) => Ok(env.get_boolean(*b)?.into_unknown()),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(env.create_int64(i)?.into_unknown())
            } else if let Some(f) = n.as_f64() {
                Ok(env.create_double(f)?.into_unknown())
            } else {
                Ok(env.get_null()?.into_unknown())
            }
        }
        JsonValue::String(s) => Ok(env.create_string(s)?.into_unknown()),
        JsonValue::Array(items) => {
            let mut arr = env.create_array(items.len() as u32)?;
            for (i, item) in items.iter().enumerate() {
                arr.set(i as u32, json_to_js(env, item)?)?;
            }
            Ok(arr.coerce_to_object()?.into_unknown())
        }
        JsonValue::Object(map) => {
            let mut obj = env.create_object()?;
            for (k, v) in map {
                obj.set_named_property(k, json_to_js(env, v)?)?;
            }
            Ok(obj.into_unknown())
        }
    }
}

// ─── App / handlers ──────────────────────────────────────────────────────────

#[derive(Clone)]
struct PlainRequest {
    method: String,
    path: String,
    body: String,
    headers: Vec<(String, String)>,
    params: Vec<(String, String)>,
    query: Vec<(String, String)>,
}

struct ReturnJsHandler {
    // Fatal: JS is `(request) => ...`, not Node's `(err, request) => ...`.
    tsfn: ThreadsafeFunction<PlainRequest, ErrorStrategy::Fatal>,
}

impl Handler for ReturnJsHandler {
    fn call(&self, req: Request) -> HandlerFuture {
        let tsfn = self.tsfn.clone();
        Box::pin(async move {
            let body = req.body_str();
            let plain = PlainRequest {
                method: req.method,
                path: req.path,
                body,
                headers: req.headers,
                params: req.params.into_iter().collect(),
                query: req.query.into_iter().collect(),
            };

            // call_async awaits JS Promises; JsJson converts on the Node thread (Send).
            match tsfn.call_async::<JsJson>(plain).await {
                Ok(JsJson(json)) => response_from_value(json),
                Err(err) => {
                    let message = err.to_string();
                    let status = if message.contains("missing path param")
                        || message.contains("missing query param")
                        || message.contains("missing body param")
                    {
                        400
                    } else {
                        500
                    };
                    Response::text(status, format!("js handler error: {err}"))
                }
            }
        })
    }
}

fn make_tsfn(
    handler: JsFunction,
) -> Result<ThreadsafeFunction<PlainRequest, ErrorStrategy::Fatal>> {
    handler.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<PlainRequest>| {
        let PlainRequest {
            method,
            path,
            body,
            headers,
            params,
            query,
        } = ctx.value;

        let mut obj = ctx.env.create_object()?;
        obj.set_named_property("method", method)?;
        obj.set_named_property("path", path)?;
        obj.set_named_property("body", body)?;

        let mut headers_obj = ctx.env.create_object()?;
        for (name, value) in headers {
            headers_obj.set_named_property(&name, value)?;
        }
        obj.set_named_property("headers", headers_obj)?;

        let mut params_obj = ctx.env.create_object()?;
        for (name, value) in params {
            params_obj.set_named_property(&name, value)?;
        }
        obj.set_named_property("params", params_obj)?;

        let mut query_obj = ctx.env.create_object()?;
        for (name, value) in query {
            query_obj.set_named_property(&name, value)?;
        }
        obj.set_named_property("query", query_obj)?;

        obj.set_named_property("state", ctx.env.create_object()?)?;

        Ok(vec![obj])
    })
}

#[napi]
pub struct App {
    inner: Mutex<CoreApp>,
}

#[napi]
impl App {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CoreApp::new()),
        }
    }

    #[napi]
    pub fn route(&self, method: String, path: String, handler: JsFunction) -> Result<()> {
        let tsfn = make_tsfn(handler)?;
        let mut app = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("app lock poisoned"))?;
        app.route(&method, &path, ReturnJsHandler { tsfn });
        Ok(())
    }

    #[napi]
    pub async fn listen(&self, host: String, port: u32) -> Result<()> {
        let app = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| Error::from_reason("app lock poisoned"))?;
            guard.clone()
        };

        app.listen_host_port(&host, port as u16)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ─── Core helpers ────────────────────────────────────────────────────────────

#[napi]
pub fn get_http_methods() -> Vec<String> {
    HTTP_METHODS.iter().map(|s| (*s).to_string()).collect()
}

#[napi]
pub fn api_resource_name_js(class_name: String) -> String {
    api_resource_name(&class_name)
}

#[napi]
pub fn resolve_route_path_js(template: String, class_name: String) -> String {
    resolve_route_path(&template, &class_name)
}

#[napi]
pub fn coerce_param_js(env: Env, raw: String, kind: Option<String>) -> Result<Unknown> {
    let value = coerce_param(
        &raw,
        param_kind_from_name(kind.as_deref().unwrap_or("auto")),
    );
    json_to_js(&env, &value)
}

#[napi(object)]
pub struct HttpStatusCode {
    pub name: String,
    pub code: u16,
}

#[napi]
pub fn get_http_status_codes() -> Vec<HttpStatusCode> {
    HTTP_STATUS_CODES
        .iter()
        .map(|(name, code)| HttpStatusCode {
            name: (*name).to_string(),
            code: *code,
        })
        .collect()
}

#[napi(object)]
pub struct HttpHeaderConstant {
    pub name: String,
    pub value: String,
}

#[napi]
pub fn get_http_header_constants() -> Vec<HttpHeaderConstant> {
    HTTP_HEADER_CONSTANTS
        .iter()
        .map(|(name, value)| HttpHeaderConstant {
            name: (*name).to_string(),
            value: (*value).to_string(),
        })
        .collect()
}

fn btree_to_hashmap(
    map: std::collections::BTreeMap<String, String>,
) -> std::collections::HashMap<String, String> {
    map.into_iter().collect()
}

#[napi]
pub fn header_attachment(filename: String) -> std::collections::HashMap<String, String> {
    btree_to_hashmap(attachment(&filename))
}

#[napi]
pub fn header_inline(filename: Option<String>) -> std::collections::HashMap<String, String> {
    btree_to_hashmap(inline(filename.as_deref()))
}

#[napi]
pub fn header_content_type(
    media_type: String,
    charset: Option<String>,
) -> std::collections::HashMap<String, String> {
    btree_to_hashmap(content_type(&media_type, charset.as_deref()))
}

#[napi]
pub fn header_location(url: String) -> std::collections::HashMap<String, String> {
    btree_to_hashmap(location(&url))
}

#[napi]
pub fn header_cache_control(value: String) -> std::collections::HashMap<String, String> {
    btree_to_hashmap(cache_control(&value))
}

#[napi]
pub fn header_download(
    filename: String,
    media_type: Option<String>,
) -> std::collections::HashMap<String, String> {
    btree_to_hashmap(download(&filename, media_type.as_deref()))
}

#[napi]
pub fn get_fingerprint_headers() -> std::collections::HashMap<String, String> {
    btree_to_hashmap(fingerprint_headers())
}

/// True when the client prefers JSON (`Accept` or `?format=json`).
#[napi]
pub fn prefers_json_js(accept: Option<String>, format_query: Option<String>) -> bool {
    prefers_json(accept.as_deref(), format_query.as_deref())
}

/// Render a Tera template file relative to `templates_root` (default `"templates"`).
#[napi]
pub fn render_template_js(
    template_name: String,
    context: JsJson,
    templates_root: Option<String>,
) -> Result<String> {
    let root = std::path::PathBuf::from(templates_root.unwrap_or_else(|| "templates".into()));
    render_template(&template_name, &context.0, &root).map_err(|e| Error::from_reason(e))
}

#[napi(object)]
pub struct PaginationParams {
    pub page: u32,
    pub page_size: u32,
    pub offset: u32,
}

impl From<PageParams> for PaginationParams {
    fn from(p: PageParams) -> Self {
        Self {
            page: p.page as u32,
            page_size: p.page_size as u32,
            offset: p.offset as u32,
        }
    }
}

fn query_object_to_map(query: Object) -> Result<std::collections::HashMap<String, String>> {
    let names = query.get_property_names()?;
    let len = names.get_array_length()?;
    let mut map = std::collections::HashMap::new();
    for i in 0..len {
        let name_val: Unknown = names.get_element(i)?;
        let key = name_val.coerce_to_string()?.into_utf8()?.into_owned()?;
        let value: Unknown = query.get_named_property(&key)?;
        if value.get_type()? == ValueType::Undefined || value.get_type()? == ValueType::Null {
            continue;
        }
        let s = value.coerce_to_string()?.into_utf8()?.into_owned()?;
        map.insert(key, s);
    }
    Ok(map)
}

#[napi]
pub fn parse_pagination(
    query: Object,
    default_page_size: Option<u32>,
    max_page_size: Option<u32>,
) -> Result<PaginationParams> {
    let map = query_object_to_map(query)?;
    let config = PageConfig {
        default_page_size: default_page_size.unwrap_or(20) as u64,
        max_page_size: max_page_size.unwrap_or(100) as u64,
    };
    let params = parse_page_params(&map, &config).map_err(|e| {
        Error::new(
            Status::InvalidArg,
            e.detail
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("HTTP {}", e.status)),
        )
    })?;
    Ok(params.into())
}

#[napi]
pub fn paginated_body(
    env: Env,
    items: Unknown,
    total: u32,
    params: PaginationParams,
) -> Result<Unknown> {
    let items_json = js_to_json(items)?;
    let page = PageParams {
        page: params.page as u64,
        page_size: params.page_size as u64,
        offset: params.offset as u64,
    };
    let body = core_paginated_body(items_json, total as u64, &page);
    json_to_js(&env, &body)
}

fn cache_err(e: String) -> Error {
    Error::from_reason(e)
}

fn ttl_secs(ttl: Option<f64>) -> Result<Option<std::time::Duration>> {
    match ttl {
        None => Ok(None),
        Some(s) if s < 0.0 => Err(Error::from_reason("ttl must be >= 0")),
        Some(s) => Ok(Some(std::time::Duration::from_secs_f64(s))),
    }
}

/// Apply `cache.*` from a Settings instance to the process-wide cache.
#[napi]
pub fn cache_configure(settings: &Settings) -> Result<()> {
    let guard = settings
        .inner
        .lock()
        .map_err(|_| Error::from_reason("settings lock poisoned"))?;
    fusion_core::cache::configure_from_settings(&guard).map_err(cache_err)
}

/// Install a driver explicitly (default moka).
#[napi]
pub fn cache_configure_driver(
    driver: Option<String>,
    max_capacity: Option<u32>,
    default_ttl: Option<f64>,
) -> Result<()> {
    let mut cfg = fusion_core::cache::CacheConfig::default();
    if let Some(d) = driver {
        cfg.driver = d;
    }
    if let Some(cap) = max_capacity {
        cfg.max_capacity = u64::from(cap).max(1);
    }
    if let Some(secs) = default_ttl {
        cfg.default_ttl = Some(std::time::Duration::from_secs_f64(secs));
    }
    let instance = fusion_core::cache::Cache::open(cfg).map_err(cache_err)?;
    fusion_core::cache::configure(instance);
    Ok(())
}

#[napi]
pub fn cache_set(key: String, value: JsJson, ttl: Option<f64>) -> Result<()> {
    fusion_core::cache::set(&key, value.0, ttl_secs(ttl)?).map_err(cache_err)
}

#[napi]
pub fn cache_get(env: Env, key: String) -> Result<Unknown> {
    match fusion_core::cache::get(&key).map_err(cache_err)? {
        Some(v) => json_to_js(&env, &v),
        None => env.get_null().map(|n| n.into_unknown()),
    }
}

#[napi]
pub fn cache_delete(key: String) -> Result<bool> {
    fusion_core::cache::delete(&key).map_err(cache_err)
}

#[napi]
pub fn cache_exists(key: String) -> Result<bool> {
    fusion_core::cache::exists(&key).map_err(cache_err)
}

#[napi]
pub fn cache_get_or_set(env: Env, key: String, default: JsJson, ttl: Option<f64>) -> Result<Unknown> {
    let value =
        fusion_core::cache::get_or_set(&key, default.0, ttl_secs(ttl)?).map_err(cache_err)?;
    json_to_js(&env, &value)
}

#[napi]
pub fn cache_delete_or_set(
    env: Env,
    key: String,
    value: JsJson,
    ttl: Option<f64>,
) -> Result<Unknown> {
    let stored =
        fusion_core::cache::delete_or_set(&key, value.0, ttl_secs(ttl)?).map_err(cache_err)?;
    json_to_js(&env, &stored)
}

#[napi]
pub fn cache_exists_or_set(key: String, value: JsJson, ttl: Option<f64>) -> Result<bool> {
    fusion_core::cache::exists_or_set(&key, value.0, ttl_secs(ttl)?).map_err(cache_err)
}

#[napi]
pub fn cache_driver() -> Result<String> {
    fusion_core::cache::driver().map_err(cache_err)
}

#[napi]
pub fn cache_clear() -> Result<()> {
    fusion_core::cache::clear().map_err(cache_err)
}

#[napi]
pub fn cache_reset() {
    fusion_core::cache::reset_global();
}

#[napi]
pub fn cache_snapshot(env: Env) -> Result<Unknown> {
    let value = fusion_core::cache::snapshot().map_err(cache_err)?;
    json_to_js(&env, &value)
}

#[napi]
pub fn cache_panel_context(env: Env) -> Result<Unknown> {
    let value = fusion_core::cache::panel_context().map_err(cache_err)?;
    json_to_js(&env, &value)
}

/// Spawn a JS function on the Tokio background runtime. Returns task id.
///
/// Uses `call_async` so status becomes `done` only after the JS callback runs
/// (sync `Blocking` can return before the Node event loop drains the TSFN).
#[napi]
pub fn task_spawn(callback: JsFunction) -> Result<String> {
    let tsfn: ThreadsafeFunction<(), ErrorStrategy::Fatal> = callback
        .create_threadsafe_function(0, |_ctx: ThreadSafeCallContext<()>| Ok(Vec::<Unknown>::new()))?;
    Ok(fusion_core::spawn_future(async move {
        let _ = tsfn.call_async::<()>(()).await;
    }))
}

/// Spawn a JS function after `delay_ms` milliseconds.
#[napi]
pub fn task_spawn_after(delay_ms: u32, callback: JsFunction) -> Result<String> {
    let tsfn: ThreadsafeFunction<(), ErrorStrategy::Fatal> = callback
        .create_threadsafe_function(0, |_ctx: ThreadSafeCallContext<()>| Ok(Vec::<Unknown>::new()))?;
    Ok(fusion_core::spawn_after_ms_future(u64::from(delay_ms), async move {
        let _ = tsfn.call_async::<()>(()).await;
    }))
}

/// Cancel a background task by id.
#[napi]
pub fn task_cancel(id: String) -> bool {
    fusion_core::task_cancel(&id)
}

/// Status string for a task id, or null if unknown.
#[napi]
pub fn task_status(id: String) -> Option<String> {
    fusion_core::task_status(&id).map(|s| s.as_str().to_string())
}

/// Reset the task registry (tests).
#[napi]
pub fn task_reset() {
    fusion_core::reset_tasks();
}
