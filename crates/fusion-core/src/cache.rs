//! Application cache with pluggable drivers.
//!
//! Default driver is **moka** (in-process). Redis is reserved via settings
//! (`cache.driver = "redis"`) but not implemented yet.
//!
//! Settings (under `fusion.<env>.json`):
//! ```json
//! "cache": {
//!   "driver": "moka",
//!   "max_capacity": 10000,
//!   "default_ttl": null,
//!   "connection_string": null,
//!   "host": "127.0.0.1",
//!   "port": 6379,
//!   "username": null,
//!   "password": null,
//!   "db": 0
//! }
//! ```

use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use moka::sync::Cache as MokaCache;
use serde_json::Value;

use crate::settings::Settings;

/// Canonical default driver name (in-process moka).
pub const DEFAULT_DRIVER: &str = "moka";

/// Alias accepted in settings (`mako` → moka).
const DRIVER_ALIASES_MOKA: &[&str] = &["moka", "mako"];

#[derive(Debug, Clone)]
struct Entry {
    value: Value,
    expires_at: Option<Instant>,
}

impl Entry {
    fn alive(&self) -> bool {
        match self.expires_at {
            Some(at) => Instant::now() < at,
            None => true,
        }
    }
}

/// Cache configuration parsed from settings.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub driver: String,
    pub max_capacity: u64,
    pub default_ttl: Option<Duration>,
    pub connection_string: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            driver: DEFAULT_DRIVER.to_string(),
            max_capacity: 10_000,
            // null / None = no expiry unless the caller passes an explicit ttl.
            default_ttl: None,
            connection_string: None,
            host: None,
            port: None,
            username: None,
            password: None,
            db: None,
        }
    }
}

impl CacheConfig {
    /// Build config from Fusion settings (`cache.*` keys).
    pub fn from_settings(settings: &Settings) -> Self {
        let mut cfg = Self::default();
        if let Some(driver) = settings.get_str("cache.driver") {
            cfg.driver = normalize_driver(&driver);
        }
        if let Some(cap) = settings.get_u64("cache.max_capacity") {
            cfg.max_capacity = cap.max(1);
        }
        match settings.get("cache.default_ttl") {
            // Explicit null (or missing after Default) → infinite unless set(..., ttl=...).
            None | Some(Value::Null) => cfg.default_ttl = None,
            Some(Value::Number(n)) => {
                cfg.default_ttl = n.as_u64().map(Duration::from_secs);
            }
            Some(Value::String(s)) if s.eq_ignore_ascii_case("null") || s.is_empty() => {
                cfg.default_ttl = None;
            }
            _ => cfg.default_ttl = None,
        }
        cfg.connection_string = settings.get_str("cache.connection_string");
        cfg.host = settings.get_str("cache.host");
        cfg.port = settings.get_u64("cache.port").map(|p| p as u16);
        cfg.username = settings.get_str("cache.username");
        cfg.password = settings.get_str("cache.password");
        cfg.db = settings.get_u64("cache.db");
        cfg
    }
}

fn normalize_driver(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    if DRIVER_ALIASES_MOKA.contains(&lower.as_str()) {
        DEFAULT_DRIVER.to_string()
    } else {
        lower
    }
}

/// Shared cache handle used by all language bindings.
#[derive(Clone)]
pub struct Cache {
    inner: Arc<dyn CacheBackend>,
    default_ttl: Option<Duration>,
    driver: String,
}

trait CacheBackend: Send + Sync {
    fn set(&self, key: &str, entry: Entry);
    fn get(&self, key: &str) -> Option<Entry>;
    fn delete(&self, key: &str) -> bool;
    fn clear(&self);
}

struct MokaBackend {
    store: MokaCache<String, Entry>,
}

impl MokaBackend {
    fn new(max_capacity: u64) -> Self {
        Self {
            store: MokaCache::builder().max_capacity(max_capacity).build(),
        }
    }
}

impl CacheBackend for MokaBackend {
    fn set(&self, key: &str, entry: Entry) {
        self.store.insert(key.to_string(), entry);
    }

    fn get(&self, key: &str) -> Option<Entry> {
        let entry = self.store.get(key)?;
        if entry.alive() {
            Some(entry)
        } else {
            self.store.invalidate(key);
            None
        }
    }

    fn delete(&self, key: &str) -> bool {
        let existed = self.store.contains_key(key);
        self.store.invalidate(key);
        existed
    }

    fn clear(&self) {
        self.store.invalidate_all();
    }
}

impl Cache {
    /// Create a cache for the given config (errors on unknown/unsupported drivers).
    pub fn open(config: CacheConfig) -> Result<Self, String> {
        let driver = normalize_driver(&config.driver);
        let backend: Arc<dyn CacheBackend> = match driver.as_str() {
            "moka" => Arc::new(MokaBackend::new(config.max_capacity)),
            "redis" => {
                return Err(
                    "cache driver \"redis\" is not implemented yet; use \"moka\"".into(),
                );
            }
            other => {
                return Err(format!(
                    "unknown cache driver \"{other}\"; supported: moka (default)"
                ));
            }
        };
        Ok(Self {
            inner: backend,
            default_ttl: config.default_ttl,
            driver,
        })
    }

    /// Driver name currently in use (`moka`, …).
    pub fn driver(&self) -> &str {
        &self.driver
    }

    /// Store a JSON value under `key`.
    ///
    /// `ttl`:
    /// - `Some(duration)` — expire after that duration
    /// - `None` — use `default_ttl` from settings; if that is also `None`, keep forever
    pub fn set(&self, key: &str, value: Value, ttl: Option<Duration>) {
        let ttl = ttl.or(self.default_ttl);
        let expires_at = ttl.map(|d| Instant::now() + d);
        self.inner.set(
            key,
            Entry {
                value,
                expires_at,
            },
        );
    }

    /// Fetch a value if present and not expired.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.inner.get(key).map(|e| e.value)
    }

    /// Remove a key; returns whether it existed.
    pub fn delete(&self, key: &str) -> bool {
        self.inner.delete(key)
    }

    /// True when the key is present and not expired.
    pub fn exists(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }

    /// Return cached value, or store `default` and return it.
    pub fn get_or_set(&self, key: &str, default: Value, ttl: Option<Duration>) -> Value {
        if let Some(existing) = self.get(key) {
            return existing;
        }
        self.set(key, default.clone(), ttl);
        default
    }

    /// Delete then set (force replace); returns the stored value.
    pub fn delete_or_set(&self, key: &str, value: Value, ttl: Option<Duration>) -> Value {
        let _ = self.delete(key);
        self.set(key, value.clone(), ttl);
        value
    }

    /// If the key already exists, leave it and return `true`.
    /// Otherwise set `value` and return `false`.
    pub fn exists_or_set(&self, key: &str, value: Value, ttl: Option<Duration>) -> bool {
        if self.exists(key) {
            return true;
        }
        self.set(key, value, ttl);
        false
    }

    /// Drop all entries (test helper / admin).
    pub fn clear(&self) {
        self.inner.clear();
    }
}

static GLOBAL: OnceLock<RwLock<Option<Cache>>> = OnceLock::new();

fn global_slot() -> &'static RwLock<Option<Cache>> {
    GLOBAL.get_or_init(|| RwLock::new(None))
}

/// Install (or replace) the process-wide cache from settings.
pub fn configure_from_settings(settings: &Settings) -> Result<(), String> {
    let cfg = CacheConfig::from_settings(settings);
    let cache = Cache::open(cfg)?;
    let mut guard = global_slot()
        .write()
        .map_err(|_| "cache lock poisoned".to_string())?;
    *guard = Some(cache);
    Ok(())
}

/// Install a concrete cache instance as the process-wide default.
pub fn configure(cache: Cache) {
    if let Ok(mut guard) = global_slot().write() {
        *guard = Some(cache);
    }
}

/// Ensure a global cache exists (default moka if never configured).
pub fn ensure_configured() -> Result<(), String> {
    {
        let guard = global_slot()
            .read()
            .map_err(|_| "cache lock poisoned".to_string())?;
        if guard.is_some() {
            return Ok(());
        }
    }
    let cache = Cache::open(CacheConfig::default())?;
    configure(cache);
    Ok(())
}

fn with_global<R>(f: impl FnOnce(&Cache) -> R) -> Result<R, String> {
    ensure_configured()?;
    let guard = global_slot()
        .read()
        .map_err(|_| "cache lock poisoned".to_string())?;
    let cache = guard
        .as_ref()
        .ok_or_else(|| "cache is not configured".to_string())?;
    Ok(f(cache))
}

/// Process-wide `set`.
pub fn set(key: &str, value: Value, ttl: Option<Duration>) -> Result<(), String> {
    with_global(|c| c.set(key, value, ttl))
}

/// Process-wide `get`.
pub fn get(key: &str) -> Result<Option<Value>, String> {
    with_global(|c| c.get(key))
}

/// Process-wide `delete`.
pub fn delete(key: &str) -> Result<bool, String> {
    with_global(|c| c.delete(key))
}

/// Process-wide `exists`.
pub fn exists(key: &str) -> Result<bool, String> {
    with_global(|c| c.exists(key))
}

/// Process-wide `get_or_set`.
pub fn get_or_set(key: &str, default: Value, ttl: Option<Duration>) -> Result<Value, String> {
    with_global(|c| c.get_or_set(key, default, ttl))
}

/// Process-wide `delete_or_set`.
pub fn delete_or_set(key: &str, value: Value, ttl: Option<Duration>) -> Result<Value, String> {
    with_global(|c| c.delete_or_set(key, value, ttl))
}

/// Process-wide `exists_or_set`.
pub fn exists_or_set(key: &str, value: Value, ttl: Option<Duration>) -> Result<bool, String> {
    with_global(|c| c.exists_or_set(key, value, ttl))
}

/// Remove every entry from the process-wide cache.
pub fn clear() -> Result<(), String> {
    with_global(|c| c.clear())
}

/// Active driver name (`moka`, …).
pub fn driver() -> Result<String, String> {
    with_global(|c| c.driver().to_string())
}

/// Reset global cache (tests).
pub fn reset_global() {
    if let Ok(mut guard) = global_slot().write() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::thread;

    #[test]
    fn moka_set_get_delete_exists() {
        let cache = Cache::open(CacheConfig {
            default_ttl: None,
            ..CacheConfig::default()
        })
        .unwrap();
        assert!(!cache.exists("a"));
        cache.set("a", json!({"n": 1}), None);
        assert!(cache.exists("a"));
        assert_eq!(cache.get("a"), Some(json!({"n": 1})));
        assert!(cache.delete("a"));
        assert!(!cache.exists("a"));
    }

    #[test]
    fn get_or_set_and_exists_or_set() {
        let cache = Cache::open(CacheConfig {
            default_ttl: None,
            ..CacheConfig::default()
        })
        .unwrap();
        let v = cache.get_or_set("k", json!("first"), None);
        assert_eq!(v, json!("first"));
        let v2 = cache.get_or_set("k", json!("second"), None);
        assert_eq!(v2, json!("first"));
        assert!(cache.exists_or_set("k", json!("third"), None));
        assert!(!cache.exists_or_set("missing", json!(1), None));
        assert_eq!(cache.get("missing"), Some(json!(1)));
    }

    #[test]
    fn delete_or_set_replaces() {
        let cache = Cache::open(CacheConfig {
            default_ttl: None,
            ..CacheConfig::default()
        })
        .unwrap();
        cache.set("k", json!(1), None);
        let out = cache.delete_or_set("k", json!(2), None);
        assert_eq!(out, json!(2));
        assert_eq!(cache.get("k"), Some(json!(2)));
    }

    #[test]
    fn ttl_expires() {
        let cache = Cache::open(CacheConfig {
            default_ttl: None,
            ..CacheConfig::default()
        })
        .unwrap();
        cache.set("t", json!(true), Some(Duration::from_millis(40)));
        assert!(cache.exists("t"));
        thread::sleep(Duration::from_millis(60));
        assert!(!cache.exists("t"));
    }

    #[test]
    fn omitted_ttl_is_infinite_when_default_ttl_is_null() {
        let cache = Cache::open(CacheConfig {
            default_ttl: None,
            ..CacheConfig::default()
        })
        .unwrap();
        cache.set("forever", json!(1), None);
        thread::sleep(Duration::from_millis(40));
        assert!(cache.exists("forever"));
        assert_eq!(cache.get("forever"), Some(json!(1)));
    }

    #[test]
    fn omitted_ttl_uses_settings_default_ttl() {
        let cache = Cache::open(CacheConfig {
            default_ttl: Some(Duration::from_millis(40)),
            ..CacheConfig::default()
        })
        .unwrap();
        cache.set("k", json!(1), None);
        assert!(cache.exists("k"));
        thread::sleep(Duration::from_millis(60));
        assert!(!cache.exists("k"));
    }

    #[test]
    fn explicit_ttl_overrides_default_ttl() {
        let cache = Cache::open(CacheConfig {
            default_ttl: Some(Duration::from_millis(40)),
            ..CacheConfig::default()
        })
        .unwrap();
        // Explicit long TTL must not expire with the short default.
        cache.set("k", json!(1), Some(Duration::from_secs(60)));
        thread::sleep(Duration::from_millis(60));
        assert!(cache.exists("k"));
    }

    #[test]
    fn mako_alias_maps_to_moka() {
        let cfg = CacheConfig {
            driver: "mako".into(),
            default_ttl: None,
            ..CacheConfig::default()
        };
        let cache = Cache::open(cfg).unwrap();
        assert_eq!(cache.driver(), "moka");
    }

    #[test]
    fn clear_removes_all_keys() {
        let cache = Cache::open(CacheConfig {
            default_ttl: None,
            ..CacheConfig::default()
        })
        .unwrap();
        cache.set("a", json!(1), None);
        cache.set("b", json!(2), None);
        cache.clear();
        assert!(!cache.exists("a"));
        assert!(!cache.exists("b"));
    }

    #[test]
    fn redis_not_implemented() {
        let result = Cache::open(CacheConfig {
            driver: "redis".into(),
            ..CacheConfig::default()
        });
        let err = match result {
            Ok(_) => panic!("expected redis to fail"),
            Err(e) => e,
        };
        assert!(err.contains("not implemented"));
    }
}