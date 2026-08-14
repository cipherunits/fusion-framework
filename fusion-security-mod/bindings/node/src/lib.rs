#![deny(clippy::all)]

use napi_derive::napi;

#[napi]
pub fn hello(name: Option<String>) -> String {
    fusion_security_mod::hello(name.as_deref().unwrap_or("Fusion"))
}
