---
name: fusion-cache
description: >-
  Documents Fusion application cache (default moka driver, Redis reserved),
  settings under fusion.<env>.json, sync/async APIs, TTL rules, the optional
  Fusion monitor panel (cache + tasks), and clear across Python, Node, and C#.
  Use when adding cache usage or changing drivers.
---

# Fusion cache

Process-wide cache shared by Python / Node / C# via `fusion-core`.

## Default driver

**moka** (in-process Rust cache). Settings alias `mako` is accepted and maps to `moka`.

Redis (`cache.driver = "redis"`) is reserved in settings but **not implemented yet**.

## Settings (`fusion.dev.json` / stage / prod)

```json
"cache": {
  "driver": "moka",
  "max_capacity": 10000,
  "default_ttl": null,
  "max_events": 50,
  "connection_string": null,
  "host": "127.0.0.1",
  "port": 6379,
  "username": null,
  "password": null,
  "db": 0
},
"monitor": {
  "enabled": true,
  "path": "/__fusion/monitor"
}
```

| Key | Purpose |
|-----|---------|
| `driver` | `moka` (default) or future `redis` |
| `max_capacity` | moka max entries |
| `default_ttl` | seconds, or **`null` = no expiry** unless code passes `ttl=` |
| `max_events` | Ring-buffer size for recent set/delete/clear events (legacy: `cache.monitor.max_events`) |
| `connection_string` / `host` / `port` / `username` / `password` / `db` | Redis connection (future) |
| `monitor.enabled` | Mount HTML + JSON monitor routes (dev scaffold: `true`; stage/prod: `false`) |
| `monitor.path` | UI path; JSON at `{path}/json` (default `/__fusion/monitor`) |

When `monitor.enabled` is **false**, bindings must **not** register the monitor endpoints (security: disable routes, not only UI). Legacy `cache.monitor.enabled` / `cache.monitor.path` still work.

The HTML panel and `{path}/json` include **cache entries**, **recent cache events**, and **background tasks**.

### TTL rules

1. `cache.set("k", value, ttl=200)` → expires in 200 seconds (always wins).
2. `cache.set("k", value)` with `default_ttl: null` → **forever** (until delete/clear).
3. `cache.set("k", value)` with `default_ttl: 3600` → expires in 3600 seconds.

Same rules apply to `get_or_set` / `delete_or_set` / `exists_or_set` and their async variants.

## Sync API

| Python | Node | C# |
|--------|------|-----|
| `cache.set(..., ttl=?)` | `cache.set(..., ttl?)` | `Cache.Set(..., ttlSeconds?)` |
| `cache.get` | `cache.get` | `Cache.Get` |
| `cache.delete` | `cache.delete` | `Cache.Delete` |
| `cache.exists` | `cache.exists` | `Cache.Exists` |
| `cache.get_or_set` | `cache.getOrSet` | `Cache.GetOrSet` |
| `cache.delete_or_set` | `cache.deleteOrSet` | `Cache.DeleteOrSet` |
| `cache.exists_or_set` | `cache.existsOrSet` | `Cache.ExistsOrSet` |
| `cache.clear` | `cache.clear` | `Cache.Clear` |
| `cache.snapshot` | `cache.snapshot` | `Cache.Snapshot` |
| `cache.panel_context` | `cache.panelContext` | `Cache.PanelContext` |

## Async API

| Python | Node | C# |
|--------|------|-----|
| `await cache.aset` | `await cache.aset` | `await Cache.SetAsync` |
| `await cache.aget` | `await cache.aget` | `await Cache.GetAsync` |
| `await cache.adelete` | `await cache.adelete` | `await Cache.DeleteAsync` |
| `await cache.aexists` | `await cache.aexists` | `await Cache.ExistsAsync` |
| `await cache.aget_or_set` | `await cache.agetOrSet` | `await Cache.GetOrSetAsync` |
| `await cache.adelete_or_set` | `await cache.adeleteOrSet` | `await Cache.DeleteOrSetAsync` |
| `await cache.aexists_or_set` | `await cache.aexistsOrSet` | `await Cache.ExistsOrSetAsync` |
| `await cache.aclear` | `await cache.aclear` | `await Cache.ClearAsync` |

Notes:

- **clear** — drop all keys (keeps the cache instance); **reset** (tests) drops the global instance
- **snapshot** — entries + recent events + embedded `tasks` object (monitor JSON)
- **panel_context** — template vars for `fusion/monitor.html` (including task table)

Values must be JSON-compatible.

## Fusion monitor panel

Built-in HTML panel auto-mounted on `listen` / `Mount` when `monitor.enabled` is true:

- `GET {path}` — HTML (auto-refresh every 5s): cache entries, recent events, background tasks
- `GET {path}/json` — raw snapshot (includes top-level `tasks`)

Scaffold (fusion-tool): **dev** `enabled: true`; **stage/prod** `enabled: false`.

## Examples

`examples/cache.py` / `.mjs` / `.cs`  
`examples/monitor.py` / `.mjs` / `.cs`

## Implementation

- Core: `crates/fusion-core/src/cache.rs` + `monitor.rs` + `assets/templates/fusion/monitor.html`
- Python: `fusion_framework.cache` + `monitor.mount_monitor`
- Node: `cache` export + `mountMonitor` in `FusionApp.mount`
- C#: `Cache` + `FusionMonitor.Mount` via FFI
