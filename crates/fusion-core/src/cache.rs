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
//!   "max_events": 50
//! },
//! "monitor": {
//!   "enabled": true,
//!   "path": "/__fusion/monitor"
//! }
//! ```
//!
//! The HTML/JSON panel is gated by top-level ``monitor.enabled`` (see
//! [`crate::monitor`]). Legacy ``cache.monitor.*`` is still accepted.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use moka::sync::Cache as MokaCache;
use serde_json::{json, Value};

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
    /// Ring-buffer size for recent set/delete/clear events.
    pub max_events: usize,
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
            max_events: 50,
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
        // Prefer cache.max_events; legacy cache.monitor.max_events still works.
        if let Some(n) = settings
            .get_u64("cache.max_events")
            .or_else(|| settings.get_u64("cache.monitor.max_events"))
        {
            cfg.max_events = (n as usize).clamp(1, 10_000);
        }
        cfg
    }
}

/// Deprecated alias: prefer [`crate::monitor::MonitorConfig`].
#[deprecated(note = "use fusion_core::monitor::MonitorConfig")]
pub type MonitorConfig = crate::monitor::MonitorConfig;

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
struct CacheEvent {
    op: String,
    key: Option<String>,
    at_ms: u64,
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
    events: Arc<Mutex<VecDeque<CacheEvent>>>,
    max_events: usize,
}

trait CacheBackend: Send + Sync {
    fn set(&self, key: &str, entry: Entry);
    fn get(&self, key: &str) -> Option<Entry>;
    fn delete(&self, key: &str) -> bool;
    fn clear(&self);
    fn entries(&self) -> Vec<(String, Entry)>;
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

    fn entries(&self) -> Vec<(String, Entry)> {
        let mut out = Vec::new();
        for (key, entry) in self.store.iter() {
            if entry.alive() {
                out.push((key.as_ref().clone(), entry));
            } else {
                self.store.invalidate(key.as_ref());
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
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
            events: Arc::new(Mutex::new(VecDeque::new())),
            max_events: config.max_events.max(1),
        })
    }

    fn record(&self, op: &str, key: Option<&str>) {
        let Ok(mut guard) = self.events.lock() else {
            return;
        };
        guard.push_front(CacheEvent {
            op: op.to_string(),
            key: key.map(str::to_string),
            at_ms: now_unix_ms(),
        });
        while guard.len() > self.max_events {
            guard.pop_back();
        }
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
        self.record("set", Some(key));
    }

    /// Fetch a value if present and not expired.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.inner.get(key).map(|e| e.value)
    }

    /// Remove a key; returns whether it existed.
    pub fn delete(&self, key: &str) -> bool {
        let existed = self.inner.delete(key);
        if existed {
            self.record("delete", Some(key));
        }
        existed
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
        self.record("clear", None);
    }

    /// Whether the Fusion monitor is enabled (from top-level ``monitor`` settings).
    pub fn monitor_enabled(&self) -> bool {
        let _ = self;
        crate::monitor::enabled()
    }

    /// Monitor UI path (from top-level ``monitor`` settings).
    pub fn monitor_path(&self) -> String {
        let _ = self;
        crate::monitor::path()
    }

    /// JSON snapshot for the monitor panel (entries + recent events + tasks).
    pub fn snapshot(&self) -> Value {
        let entries: Vec<Value> = self
            .inner
            .entries()
            .into_iter()
            .map(|(key, entry)| {
                let ttl_remaining_secs = entry.expires_at.map(|at| {
                    at.saturating_duration_since(Instant::now()).as_secs()
                });
                json!({
                    "key": key,
                    "value": entry.value,
                    "ttl_remaining_secs": ttl_remaining_secs,
                })
            })
            .collect();
        let events: Vec<Value> = self
            .events
            .lock()
            .map(|g| {
                g.iter()
                    .map(|e| {
                        json!({
                            "op": e.op,
                            "key": e.key,
                            "at_ms": e.at_ms,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mon = crate::monitor::current();
        let tasks = crate::tasks::snapshot();
        json!({
            "driver": self.driver,
            "entry_count": entries.len(),
            "event_count": events.len(),
            "entries": entries,
            "events": events,
            "tasks": tasks,
            "monitor": {
                "enabled": mon.enabled,
                "path": mon.path,
            }
        })
    }

    /// Template context for the Fusion monitor panel (cache + background tasks).
    pub fn panel_context(&self) -> Value {
        let snap = self.snapshot();
        let entries = snap["entries"].as_array().cloned().unwrap_or_default();
        let events = snap["events"].as_array().cloned().unwrap_or_default();
        let tasks = snap["tasks"]["tasks"].as_array().cloned().unwrap_or_default();
        let entry_rows: Vec<Value> = entries
            .iter()
            .map(|e| {
                let key = e["key"].as_str().unwrap_or("").to_string();
                let value = display_cache_value(&e["value"]);
                let ttl = match e.get("ttl_remaining_secs") {
                    Some(Value::Null) | None => "∞".to_string(),
                    Some(v) => v.to_string(),
                };
                json!([key, value, ttl])
            })
            .collect();
        let event_rows: Vec<Value> = events
            .iter()
            .map(|e| {
                let op = e["op"].as_str().unwrap_or("").to_string();
                let key = e
                    .get("key")
                    .and_then(|k| k.as_str())
                    .unwrap_or("—")
                    .to_string();
                let at = e
                    .get("at_ms")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into());
                json!([op, key, at])
            })
            .collect();
        let task_rows: Vec<Value> = tasks
            .iter()
            .map(|t| {
                let id = t["id"].as_str().unwrap_or("").to_string();
                let status = t["status"].as_str().unwrap_or("").to_string();
                let delay = match t.get("delay_ms") {
                    Some(Value::Null) | None => "—".to_string(),
                    Some(v) => v.to_string(),
                };
                let created = t
                    .get("created_at_ms")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into());
                json!([id, status, delay, created])
            })
            .collect();
        let path = crate::monitor::normalize_path(&self.monitor_path());
        let entry_count = entries.len();
        let event_count = events.len();
        let task_count = snap["tasks"]["task_count"].as_u64().unwrap_or(0) as usize;
        let active_count = snap["tasks"]["active_count"].as_u64().unwrap_or(0) as usize;
        json!({
            "title": "Fusion Monitor",
            "driver": self.driver,
            "driver_label": self.driver,
            "entry_count": entry_count,
            "event_count": event_count,
            "task_count": task_count,
            "active_task_count": active_count,
            "entry_badge": format!("{entry_count} keys"),
            "event_badge": format!("{event_count} events"),
            "task_badge": format!("{active_count}/{task_count} tasks"),
            "empty_entries": entry_count == 0,
            "empty_events": event_count == 0,
            "empty_tasks": task_count == 0,
            "entry_headers": ["Key", "Value", "TTL (s)"],
            "entry_rows": entry_rows,
            "event_headers": ["Op", "Key", "Time (ms)"],
            "event_rows": event_rows,
            "task_headers": ["Id", "Status", "Delay (ms)", "Created (ms)"],
            "task_rows": task_rows,
            "path": path,
            "json_path": format!("{path}/json"),
        })
    }
}

fn display_cache_value(value: &Value) -> String {
    let raw = serde_json::to_string(value).unwrap_or_else(|_| "null".into());
    if raw.chars().count() > 160 {
        let truncated: String = raw.chars().take(157).collect();
        format!("{truncated}...")
    } else {
        raw
    }
}

static GLOBAL: OnceLock<RwLock<Option<Cache>>> = OnceLock::new();

fn global_slot() -> &'static RwLock<Option<Cache>> {
    GLOBAL.get_or_init(|| RwLock::new(None))
}

/// Install (or replace) the process-wide cache from settings.
pub fn configure_from_settings(settings: &Settings) -> Result<(), String> {
    crate::monitor::configure_from_settings(settings);
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

/// Process-wide monitor snapshot (entries + events).
pub fn snapshot() -> Result<Value, String> {
    with_global(|c| c.snapshot())
}

/// Template context for the built-in monitor HTML panel.
pub fn panel_context() -> Result<Value, String> {
    with_global(|c| c.panel_context())
}

/// Whether the global cache wants the monitor mounted.
pub fn monitor_path() -> Result<String, String> {
    Ok(crate::monitor::path())
}

/// Whether the Fusion monitor should be mounted.
pub fn monitor_enabled() -> Result<bool, String> {
    Ok(crate::monitor::enabled())
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
    fn snapshot_lists_entries_and_events() {
        crate::monitor::configure(crate::monitor::MonitorConfig {
            enabled: true,
            path: "/__fusion/monitor".into(),
        });
        let cache = Cache::open(CacheConfig {
            default_ttl: None,
            max_events: 10,
            ..CacheConfig::default()
        })
        .unwrap();
        cache.set("a", json!(1), None);
        cache.set("b", json!({"x": true}), None);
        let _ = cache.delete("a");
        let snap = cache.snapshot();
        assert_eq!(snap["driver"], "moka");
        assert_eq!(snap["entry_count"], 1);
        assert_eq!(snap["monitor"]["enabled"], true);
        assert_eq!(snap["monitor"]["path"], "/__fusion/monitor");
        let entries = snap["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["key"], "b");
        let events = snap["events"].as_array().unwrap();
        assert!(events.len() >= 2);
        assert_eq!(events[0]["op"], "delete");
        assert!(snap["tasks"].is_object());
        assert!(snap["tasks"]["tasks"].is_array());
        let ctx = cache.panel_context();
        assert_eq!(ctx["title"], "Fusion Monitor");
        assert_eq!(ctx["task_headers"][0], "Id");
        assert!(ctx["task_badge"].as_str().unwrap_or("").contains("tasks"));
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