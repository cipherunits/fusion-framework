---
name: fusion-http-routes
description: >-
  Documents Fusion HTTP routing: convention methods, custom http_get/HttpGet
  routes, [module] and [action] tokens, mount slots, and Swagger path
  generation. Use when implementing or debugging routes in any binding.
---

# HTTP Routes & Swagger

## Route tokens

| Token | Source | Example |
|-------|--------|---------|
| `[module]` | Class/module name, lowercased, trailing `Module` stripped | `UserModule` → `user` |
| `[action]` | Method name, lowercased, trailing `Action` stripped | `UserAction` → `user` |

Resolution lives in `crates/fusion-core/src/naming.rs` (`resolve_handler_route`, `join_route_paths`).

## Two route modes

1. **Convention** — `get`, `post`, `put`, `patch`, `delete` on the class map to `GET/POST/...` at the module base path.
2. **Custom** — explicit template per method (e.g. `test/[action]`) mounted alongside convention routes.

## Examples

See `examples/custom_http_routes.py`, `custom_http_routes.mjs`, `custom_http_routes.cs`.

**Python**

```python
@route("/api/[module]")
class UserModule(FusionBaseApi):
    def get(self): ...
    @http_get("test/[action]")
    def UserAction(self): ...
# GET /api/user  and  GET /api/user/test/user
```

**Node** — attach metadata before `route()`:

```javascript
httpGet('test/[action]')(UserModule.prototype.UserAction)
route('/api/[module]')(UserModule)
```

**C#**

```csharp
[Route("/api/[module]")]
public class UserModule : FusionBaseApi {
    [HttpGet("test/[action]")]
    public object UserAction() => ...;
}
```

## Implementation notes

- Registry uses **route mount slots** (path + method + handler) in Python (`api_types.rs`) and equivalents in Node/C#.
- OpenAPI `paths` must be built from the same slots the HTTP server mounts.
- Swagger version navbar: each API version gets its own spec URL; do not mix paths across versions.

## Tests

- Rust: `cargo test -p fusion-core naming`
- Python: `pytest tests/python`
