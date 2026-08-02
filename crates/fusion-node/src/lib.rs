use std::sync::mpsc;
use std::sync::Mutex;

use bytes::Bytes;
use fusion_core::{App as CoreApp, Handler, Request, Response};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi::{JsFunction, JsObject, JsUnknown, Status, ValueType};
use napi_derive::napi;

struct BridgeResponse {
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
}

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
                let parsed = parse_js_response(value).unwrap_or_else(|err| BridgeResponse {
                    status: 500,
                    body: format!("js handler error: {err}"),
                    headers: vec![],
                });
                let _ = tx.send(parsed);
                Ok(())
            },
        );

        if status != Status::Ok {
            return Response::text(500, format!("failed to call js handler: {status}"));
        }

        match rx.recv() {
            Ok(res) => {
                let mut response = Response::new(res.status, Bytes::from(res.body));
                response.headers = res.headers;
                response
            }
            Err(_) => Response::text(500, "js handler did not return a response"),
        }
    }
}

fn parse_js_response(value: Unknown) -> Result<BridgeResponse> {
    match value.get_type()? {
        ValueType::String => {
            let s = value.coerce_to_string()?.into_utf8()?.into_owned()?;
            Ok(BridgeResponse {
                status: 200,
                body: s,
                headers: vec![],
            })
        }
        ValueType::Object => {
            let obj = value.coerce_to_object()?;
            let status: u16 = if obj.has_named_property("status")? {
                obj.get_named_property::<u16>("status").unwrap_or(200)
            } else {
                200
            };

            let body = if obj.has_named_property("body")? {
                let body_val: JsUnknown = obj.get_named_property("body")?;
                body_val.coerce_to_string()?.into_utf8()?.into_owned()?
            } else {
                String::new()
            };

            let mut headers = Vec::new();
            if obj.has_named_property("headers")? {
                if let Ok(headers_obj) = obj.get_named_property::<JsObject>("headers") {
                    let names = headers_obj.get_property_names()?;
                    let len = names.get_array_length()?;
                    for i in 0..len {
                        let name_val: Unknown = names.get_element(i)?;
                        let name = name_val.coerce_to_string()?.into_utf8()?.into_owned()?;
                        let value: Unknown = headers_obj.get_named_property(&name)?;
                        let value = value.coerce_to_string()?.into_utf8()?.into_owned()?;
                        headers.push((name, value));
                    }
                }
            }

            Ok(BridgeResponse {
                status,
                body,
                headers,
            })
        }
        _ => {
            let s = value.coerce_to_string()?.into_utf8()?.into_owned()?;
            Ok(BridgeResponse {
                status: 200,
                body: s,
                headers: vec![],
            })
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
