# fusion-ffi

C ABI over [`fusion-core`](../fusion-core) for C# and other FFI consumers.

## Build

```bash
cargo build -p fusion-ffi
# → target/debug/libfusion_ffi.so (Linux) / fusion_ffi.dll / libfusion_ffi.dylib
```

## Surface

| Function | Role |
|----------|------|
| `fusion_app_new` / `free` | App lifecycle |
| `fusion_app_route` | Register method+path+callback |
| `fusion_app_listen` / `listen_from_settings` | Blocking HTTP server |
| `fusion_settings_*` | Load `fusion.<env>.json`, get/merge |
| `fusion_string_dup` / `free` | Cross-boundary UTF-8 strings |
| `fusion_http_status_codes` | JSON map of status constants |
| `fusion_resolve_route_path` | Expand `[module]` |

Handler callbacks must return a string allocated with `fusion_string_dup`.

See `bindings/csharp/FusionFramework` for the managed wrapper.
