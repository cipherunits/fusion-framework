use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Mutex;

use fusion_core::{
    App as CoreApp, Handler, Request, Response, Settings as CoreSettings, api_resource_name,
    coerce_param, param_kind_from_name, resolve_route_path, response_from_value, HTTP_METHODS,
};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi::{JsFunction, Status, ValueType};
use napi_derive::napi;
use serde_json::{Map, Number, Value as JsonValue};

// ─── JS ↔ JSON bridge ────────────────────────────────────────────────────────

fn js_to_json(value: Unknown) -> Result<JsonValue> {
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

fn json_to_js(env: &Env, value: &JsonValue) -> Result<Unknown> {
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

// ─── Settings ────────────────────────────────────────────────────────────────

#[napi]
pub struct Settings {
    inner: Mutex<CoreSettings>,
}

#[napi]
impl Settings {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CoreSettings::new()),
        }
    }

    #[napi]
    pub fn load_json(
        &self,
        path: Option<String>,
        env_name: Option<String>,
        extra_roots: Option<Vec<String>>,
    ) -> Result<()> {
        let roots: Vec<PathBuf> = extra_roots
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        guard
            .load_json(
                path.as_deref().map(std::path::Path::new),
                env_name.as_deref(),
                &roots,
            )
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    #[napi]
    pub fn ensure_loaded(&self, extra_roots: Option<Vec<String>>) -> Result<()> {
        let roots: Vec<PathBuf> = extra_roots
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        guard
            .ensure_loaded(&roots)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(())
    }

    #[napi]
    pub fn merge(&self, env: Env, values: Unknown) -> Result<()> {
        let JsonValue::Object(map) = js_to_json(values)? else {
            return Err(Error::from_reason("merge expects an object"));
        };
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        guard.merge_map(map);
        let _ = env;
        Ok(())
    }

    #[napi]
    pub fn get(&self, env: Env, key: String, default: Option<Unknown>) -> Result<Unknown> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        guard
            .ensure_loaded(&[])
            .map_err(|e| Error::from_reason(e.to_string()))?;
        match guard.get(&key) {
            Some(v) => json_to_js(&env, &v),
            None => Ok(default.unwrap_or_else(|| env.get_undefined().unwrap().into_unknown())),
        }
    }

    #[napi(getter)]
    pub fn host(&self) -> Result<String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        let _ = guard.ensure_loaded(&[]);
        Ok(guard.host())
    }

    #[napi(getter)]
    pub fn port(&self) -> Result<u32> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        let _ = guard.ensure_loaded(&[]);
        Ok(guard.port() as u32)
    }

    #[napi(getter)]
    pub fn debug(&self) -> Result<bool> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        let _ = guard.ensure_loaded(&[]);
        Ok(guard.debug())
    }

    #[napi(getter)]
    pub fn env(&self) -> Result<String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        let _ = guard.ensure_loaded(&[]);
        Ok(guard.env().to_string())
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
}

struct ReturnJsHandler {
    tsfn: ThreadsafeFunction<PlainRequest, ErrorStrategy::CalleeHandled>,
}

impl Handler for ReturnJsHandler {
    fn call(&self, req: Request) -> Response {
        let body = req.body_str();
        let plain = PlainRequest {
            method: req.method,
            path: req.path,
            body,
            headers: req.headers,
            params: req.params.into_iter().collect(),
        };

        let (tx, rx) = mpsc::channel();

        let status = self.tsfn.call_with_return_value(
            Ok(plain),
            ThreadsafeFunctionCallMode::Blocking,
            move |value: Unknown| {
                let parsed = js_to_json(value).unwrap_or_else(|err| {
                    JsonValue::Object(Map::from_iter([
                        ("status".into(), JsonValue::Number(Number::from(500u64))),
                        (
                            "body".into(),
                            JsonValue::String(format!("js handler error: {err}")),
                        ),
                    ]))
                });
                let _ = tx.send(parsed);
                Ok(())
            },
        );

        if status != Status::Ok {
            return Response::text(500, format!("failed to call js handler: {status}"));
        }

        match rx.recv() {
            Ok(value) => response_from_value(value),
            Err(_) => Response::text(500, "js handler did not return a response"),
        }
    }
}

fn make_tsfn(
    handler: JsFunction,
) -> Result<ThreadsafeFunction<PlainRequest, ErrorStrategy::CalleeHandled>> {
    handler.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<PlainRequest>| {
        let PlainRequest {
            method,
            path,
            body,
            headers,
            params,
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
    let value = coerce_param(&raw, param_kind_from_name(kind.as_deref().unwrap_or("auto")));
    json_to_js(&env, &value)
}
