//! Process-wide Fusion monitor panel settings (cache + background tasks UI).
//!
//! Top-level settings (preferred):
//! ```json
//! "monitor": {
//!   "enabled": true,
//!   "path": "/__fusion/monitor"
//! }
//! ```
//!
//! Legacy fallback: ``cache.monitor.enabled`` / ``cache.monitor.path``.

use std::sync::{OnceLock, RwLock};

use crate::settings::Settings;

/// Default URL for the built-in HTML + JSON monitor.
pub const DEFAULT_PATH: &str = "/__fusion/monitor";

/// Gate + path for the Fusion monitor panel (independent of ``cache.*``).
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub enabled: bool,
    pub path: String,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: DEFAULT_PATH.to_string(),
        }
    }
}

impl MonitorConfig {
    /// Read ``monitor.*``, falling back to legacy ``cache.monitor.*``.
    pub fn from_settings(settings: &Settings) -> Self {
        let mut cfg = Self::default();
        cfg.enabled = settings
            .get_bool("monitor.enabled")
            .or_else(|| settings.get_bool("cache.monitor.enabled"))
            .unwrap_or(false);
        let path = settings
            .get_str("monitor.path")
            .or_else(|| settings.get_str("cache.monitor.path"));
        if let Some(path) = path {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                cfg.path = normalize_path(trimmed);
            }
        }
        cfg
    }
}

/// Normalize monitor URL path (leading slash, no trailing slash).
pub fn normalize_path(raw: &str) -> String {
    let trimmed = raw.trim();
    let with_slash = if trimmed.is_empty() {
        DEFAULT_PATH.to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    let stripped = with_slash.trim_end_matches('/');
    if stripped.is_empty() {
        DEFAULT_PATH.to_string()
    } else {
        stripped.to_string()
    }
}

static GLOBAL: OnceLock<RwLock<MonitorConfig>> = OnceLock::new();

fn slot() -> &'static RwLock<MonitorConfig> {
    GLOBAL.get_or_init(|| RwLock::new(MonitorConfig::default()))
}

/// Install monitor settings from Fusion env JSON.
pub fn configure_from_settings(settings: &Settings) {
    configure(MonitorConfig::from_settings(settings));
}

/// Replace the process-wide monitor config.
pub fn configure(cfg: MonitorConfig) {
    if let Ok(mut guard) = slot().write() {
        *guard = cfg;
    }
}

/// Current monitor config (defaults if never configured).
pub fn current() -> MonitorConfig {
    slot()
        .read()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Whether the monitor should be mounted.
pub fn enabled() -> bool {
    current().enabled
}

/// Monitor UI path (JSON at ``{path}/json``).
pub fn path() -> String {
    current().path
}

/// Reset monitor config (tests).
pub fn reset() {
    if let Ok(mut guard) = slot().write() {
        *guard = MonitorConfig::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefers_top_level_monitor_settings() {
        let mut s = Settings::new();
        s.merge_map(
            json!({
                "monitor": { "enabled": true, "path": "/ops" },
                "cache": { "monitor": { "enabled": false, "path": "/old" } }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let cfg = MonitorConfig::from_settings(&s);
        assert!(cfg.enabled);
        assert_eq!(cfg.path, "/ops");
    }

    #[test]
    fn falls_back_to_legacy_cache_monitor() {
        let mut s = Settings::new();
        s.merge_map(
            json!({
                "cache": { "monitor": { "enabled": true, "path": "/__fusion/cache" } }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let cfg = MonitorConfig::from_settings(&s);
        assert!(cfg.enabled);
        assert_eq!(cfg.path, "/__fusion/cache");
    }
}
