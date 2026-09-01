//! Language-neutral settings loaded from ``fusion.<env>.json``.
//!
//! Host bindings only add language-specific overlays (e.g. a Python
//! ``settings.py`` module). JSON discovery, env placeholders, and key
//! lookup live here so Node / C# / Python share one implementation.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::{Error, Result};

fn normalize_key(key: &str) -> String {
    key.trim().replace('-', "_").to_ascii_lowercase()
}

fn resolve_value(value: &Value) -> Value {
    match value {
        Value::String(s)
            if s.len() > 1
                && s.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                && s.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_') =>
        {
            env::var(s)
                .map(Value::String)
                .unwrap_or_else(|_| value.clone())
        }
        Value::Array(items) => Value::Array(items.iter().map(resolve_value).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), resolve_value(v));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Runtime settings shared by every language binding.
#[derive(Debug, Clone)]
pub struct Settings {
    config: Map<String, Value>,
    raw: Value,
    env_name: String,
    loaded_from: Vec<String>,
    auto_loaded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    pub fn new() -> Self {
        Self {
            config: Map::new(),
            raw: Value::Object(Map::new()),
            env_name: env::var("FUSION_ENV").unwrap_or_else(|_| "dev".into()),
            loaded_from: Vec::new(),
            auto_loaded: false,
        }
    }

    pub fn env(&self) -> &str {
        &self.env_name
    }

    pub fn loaded_from(&self) -> &[String] {
        &self.loaded_from
    }

    pub fn raw(&self) -> &Value {
        &self.raw
    }

    pub fn config(&self) -> &Map<String, Value> {
        &self.config
    }

    pub fn host(&self) -> String {
        self.get_str("host").unwrap_or_else(|| "127.0.0.1".into())
    }

    pub fn port(&self) -> u16 {
        self.get_u64("port").unwrap_or(3000) as u16
    }

    pub fn debug(&self) -> bool {
        self.get_bool("debug").unwrap_or(false)
    }

    /// Directory for Tera templates (``templates.dir`` in settings).
    pub fn templates_dir(&self) -> String {
        self.get_str("templates.dir")
            .or_else(|| self.get_str("templates_dir"))
            .unwrap_or_else(|| "templates".into())
    }

    /// Load ``fusion.<env>.json`` (auto-discover) or an explicit path.
    ///
    /// `extra_roots` are searched after the process cwd (e.g. `__main__` dir).
    pub fn load_json(
        &mut self,
        path: Option<&Path>,
        env_name: Option<&str>,
        extra_roots: &[PathBuf],
    ) -> Result<&mut Self> {
        let path = match path {
            Some(p) => {
                if !p.is_file() {
                    return Err(Error::Other(format!(
                        "settings json not found: {}",
                        p.display()
                    )));
                }
                p.to_path_buf()
            }
            None => {
                let env_name = env_name.map(str::to_string).unwrap_or_else(|| {
                    env::var("FUSION_ENV").unwrap_or_else(|_| self.env_name.clone())
                });
                self.env_name = env_name.clone();
                match find_json_file(&env_name, extra_roots) {
                    Some(p) => p,
                    None => {
                        self.auto_loaded = true;
                        return Ok(self);
                    }
                }
            }
        };

        let text = fs::read_to_string(&path).map_err(Error::from)?;
        let data: Value = serde_json::from_str(&text)
            .map_err(|e| Error::Other(format!("invalid settings json {}: {e}", path.display())))?;
        let Value::Object(map) = data else {
            return Err(Error::Other(format!(
                "settings json must be an object: {}",
                path.display()
            )));
        };

        self.raw = Value::Object(map.clone());
        if let Some(Value::String(env)) = map.get("env") {
            self.env_name = env.clone();
        }

        if let Some(Value::Object(config)) = map.get("config") {
            self.merge_map(config.clone());
        } else {
            let mut filtered = Map::new();
            for (k, v) in &map {
                if k != "env" && k != "commands" {
                    filtered.insert(k.clone(), v.clone());
                }
            }
            self.merge_map(filtered);
        }

        if let Some(Value::Object(commands)) = map.get("commands") {
            self.config
                .entry("commands".to_string())
                .or_insert_with(|| Value::Object(commands.clone()));
        }

        self.loaded_from
            .push(path.canonicalize().unwrap_or(path).display().to_string());
        self.auto_loaded = true;
        Ok(self)
    }

    /// Ensure JSON settings are loaded (no-op if already loaded).
    pub fn ensure_loaded(&mut self, extra_roots: &[PathBuf]) -> Result<&mut Self> {
        if self.auto_loaded {
            return Ok(self);
        }
        self.load_json(None, None, extra_roots)
    }

    pub fn merge_map(&mut self, values: Map<String, Value>) {
        for (key, value) in values {
            let normalized = normalize_key(&key);
            self.config.insert(key.clone(), value.clone());
            self.config.insert(normalized, value);
        }
        self.auto_loaded = true;
    }

    pub fn configure(&mut self, values: Map<String, Value>) {
        let _ = self.ensure_loaded(&[]);
        self.merge_map(values);
    }

    pub fn clear(&mut self) {
        self.config.clear();
        self.raw = Value::Object(Map::new());
        self.loaded_from.clear();
        self.auto_loaded = false;
        self.env_name = env::var("FUSION_ENV").unwrap_or_else(|_| "dev".into());
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        if key.contains('.') {
            return self.get_dotted(key);
        }

        let normalized = normalize_key(key);
        for candidate in [key, normalized.as_str()] {
            if let Some(v) = self.config.get(candidate) {
                return Some(resolve_value(v));
            }
        }
        for (store_key, value) in &self.config {
            if normalize_key(store_key) == normalized {
                return Some(resolve_value(value));
            }
        }
        None
    }

    pub fn get_or(&self, key: &str, default: Value) -> Value {
        self.get(key).unwrap_or(default)
    }

    pub fn get_str(&self, key: &str) -> Option<String> {
        match self.get(key)? {
            Value::String(s) => Some(s),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            other => Some(other.to_string()),
        }
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        match self.get(key)? {
            Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i as u64)),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            Value::Bool(b) => Some(b),
            Value::String(s) => {
                let lower = s.to_ascii_lowercase();
                Some(matches!(lower.as_str(), "1" | "true" | "yes" | "on"))
            }
            Value::Number(n) => Some(n.as_u64().unwrap_or(0) != 0),
            _ => None,
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn keys(&self) -> BTreeSet<String> {
        self.config.keys().map(|k| normalize_key(k)).collect()
    }

    fn get_dotted(&self, key: &str) -> Option<Value> {
        let parts: Vec<&str> = key.split('.').collect();

        let dig = |root: &Value| -> Option<Value> {
            let mut cursor = root;
            for part in &parts {
                let Value::Object(map) = cursor else {
                    return None;
                };
                if let Some(next) = map.get(*part) {
                    cursor = next;
                    continue;
                }
                let norm = normalize_key(part);
                let next = map.iter().find(|(k, _)| normalize_key(k) == norm)?.1;
                cursor = next;
            }
            Some(resolve_value(cursor))
        };

        dig(&self.raw).or_else(|| dig(&Value::Object(self.config.clone())))
    }
}

fn find_json_file(env_name: &str, extra_roots: &[PathBuf]) -> Option<PathBuf> {
    let filename = format!("fusion.{env_name}.json");
    let roots = search_roots(extra_roots);

    for root in &roots {
        let candidate = root.join(&filename);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    for root in &roots {
        if let Ok(entries) = fs::read_dir(root) {
            let mut matches: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("fusion.") && n.ends_with(".json"))
                })
                .collect();
            matches.sort();
            if let Some(first) = matches.into_iter().next() {
                return Some(first);
            }
        }
    }
    None
}

fn search_roots(extra_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        roots.push(cwd.clone());
        roots.extend(cwd.ancestors().skip(1).map(|p| p.to_path_buf()));
    }
    roots.extend(extra_roots.iter().cloned());

    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for root in roots {
        if let Ok(resolved) = root.canonicalize() {
            if seen.insert(resolved.display().to_string()) {
                unique.push(resolved);
            }
        } else if seen.insert(root.display().to_string()) {
            unique.push(root);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    #[test]
    fn loads_config_object() {
        let dir = tempfile_dir();
        let path = dir.join("fusion.dev.json");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            r#"{{"env":"dev","config":{{"host":"0.0.0.0","port":8088,"debug":true}}}}"#
        )
        .unwrap();

        let mut settings = Settings::new();
        settings.load_json(Some(&path), None, &[]).unwrap();
        assert_eq!(settings.host(), "0.0.0.0");
        assert_eq!(settings.port(), 8088);
        assert!(settings.debug());
    }

    #[test]
    fn plain_object_is_json_data_not_envelope_confusion() {
        let mut settings = Settings::new();
        settings.merge_map({
            let mut m = Map::new();
            m.insert("secret_key".into(), json!("abc"));
            m
        });
        assert_eq!(settings.get_str("secret_key").as_deref(), Some("abc"));
    }

    fn tempfile_dir() -> PathBuf {
        let dir = env::temp_dir().join(format!("fusion-settings-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        dir
    }
}
