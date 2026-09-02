---
name: fusion-bindings-parity
description: >-
  Keeps Python, Node, and C# Fusion bindings aligned when changing APIs, routes,
  Swagger, or middleware. Use when editing more than one binding or adding
  cross-language behavior.
---

# Bindings Parity

## Checklist (every cross-binding change)

- [ ] `fusion-core` updated if semantics are shared
- [ ] Python: `fusion_framework/` + `crates/fusion-py/src/api_types.rs`
- [ ] Node: `crates/fusion-node/index.js`
- [ ] C#: `bindings/csharp/FusionFramework/*.cs`
- [ ] Example snippet in `examples/` (at least one runnable file + others documented)
- [ ] README in C# binding updated if public API changed

## Parity matrix

| Concept | Python | Node | C# |
|---------|--------|------|-----|
| Module route | `@route("/api/[module]")` | `route('/api/[module]')(Cls)` | `[Route("/api/[module]")]` |
| Convention HTTP | `def get(self)` | `get()` method | `Get()` method |
| Custom HTTP | `@http_get("path/[action]")` | `httpGet('path/[action]')(proto.method)` | `[HttpGet("path/[action]")]` |
| OpenAPI / Swagger | `app.py` + `api_types.rs` | `buildOpenApi` in `index.js` | `Swagger.cs` |
| Version navbar | per-version OpenAPI routes | same | same |

## Verification commands

```bash
cargo test -p fusion-core naming
cargo check -p fusion-py
node --check crates/fusion-node/index.js
dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj
python -m pytest tests/python -q
```

## Style

- Match existing naming in each language (snake_case Python, camelCase Node helpers, PascalCase C#).
- Prefer minimal diffs; do not refactor unrelated binding code.
