use std::path::PathBuf;
use std::sync::Mutex;

use fusion_core::Settings as CoreSettings;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value as JsonValue;

use crate::{js_to_json, json_to_js};

#[napi]
pub struct Settings {
    pub(crate) inner: Mutex<CoreSettings>,
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
    pub fn reload(&self) -> Result<bool> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| Error::from_reason("settings lock poisoned"))?;
        let _ = guard.ensure_loaded(&[]);
        Ok(guard.reload())
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
