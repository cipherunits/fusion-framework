---
name: fusion-bindings-parity
description: >-
  Keeps Python, Node, and C# Fusion bindings aligned when changing APIs, routes,
  Swagger, middleware, or permissions. Use when editing more than one binding
  or when the user asks to add a feature (always implement all three languages
  unless they limit scope).
---

# Bindings Parity

## Hard rule

If the user says “add X” (permissions, middleware, route option, Swagger behavior, settings key, etc.) and X is framework surface area, implement it for:

1. **Python**
2. **Node**
3. **C#**

in the **same** change set unless they explicitly say “only Python” (or only one binding).

Do not leave one language behind “for later” without saying so and getting confirmation.

## Checklist (every cross-binding change)

- [ ] `fusion-core` updated if semantics are shared
- [ ] Python: `fusion_framework/` + `crates/fusion-py/src/api_types.rs` as needed
- [ ] Node: `crates/fusion-node/index.js` (+ `index.d.ts` if public types change)
- [ ] C#: `bindings/csharp/FusionFramework/*.cs`
- [ ] Tests under `tests/python/`, `tests/node/`, and/or `tests/csharp/` when behavior is testable
- [ ] **Examples in all three languages** under `examples/` (`<feature>.py`, `<feature>.mjs`, `<feature>.cs`) showing how to use the new API
- [ ] README in C# binding updated if public API changed
- [ ] Skills/docs updated if agents need new knowledge (`fusion-cli`, `fusion-http-routes`, …)

## Examples rule

New public surface → show usage in **Python + Node + C#**. Prefer the same basename for the trio (see `custom_http_routes.*`, `pagination.*`). Examples should be short and runnable enough to see the API shape, not full apps.

## Parity matrix

| Concept | Python | Node | C# |
|---------|--------|------|-----|
| Module route | `@route("/api/[module]")` | `route('/api/[module]')(Cls)` | `[Route("/api/[module]")]` |
| Convention HTTP | `def get(self)` | `get()` method | `Get()` method |
| Custom HTTP | `@http_get("path/[action]")` | `httpGet('path/[action]')(proto.method)` | `[HttpGet("path/[action]")]` |
| Middleware | `middleware.py` factories | factories in `index.js` | `Middleware.cs` |
| Static files | `static_files()` | `staticFiles()` | `Middleware.StaticFiles()` |
| Permissions | `permissions=` / `require_permissions` | `permissions` / `requirePermissions` | `PermissionTypes` / `RequirePermissions` |
| OpenAPI / Swagger | `app.py` + `api_types.rs` | `buildOpenApi` in `index.js` | `Swagger.cs` |
| Version navbar | per-version OpenAPI routes | same | same |
| Template routes | omit from OpenAPI | omit | omit |

## Verification commands

```bash
cargo test -p fusion-core naming
cargo check -p fusion-py
node --check crates/fusion-node/index.js
dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj
./tests/scripts/run-python.sh -q
./tests/scripts/run-node.sh
./tests/scripts/run-csharp.sh -q
```

## Style

- Match existing naming in each language (snake_case Python, camelCase Node helpers, PascalCase C#).
- Prefer minimal diffs; do not refactor unrelated binding code.
- Comment new exported helpers (see `fusion-coding-standards`).
