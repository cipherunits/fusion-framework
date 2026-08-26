---
name: fusion-architecture
description: >-
  Explains Fusion Framework repository layout, crate boundaries, and where
  new logic belongs (fusion-core vs Python/Node/C# bindings). Use when
  navigating the codebase, adding features, or deciding which layer to change.
---

# Fusion Architecture

## Repository map

| Path | Role |
|------|------|
| `crates/fusion-core/` | Shared Rust: naming, route tokens, HTTP conventions |
| `crates/fusion-py/` | Python binding (PyO3) + `python/fusion_framework/` package |
| `crates/fusion-node/` | Node binding (`index.js`, N-API) |
| `bindings/csharp/FusionFramework/` | C# binding (source of truth for NuGet layout) |
| `examples/` | Runnable samples per binding |
| `scripts/` | Release tooling (`set-version.sh`) |

## Layering rules

1. **Put shared semantics in `fusion-core`** — route token resolution (`[module]`, `[action]`), path joining, handler naming. Bindings should call Rust helpers via FFI where possible.
2. **Bindings mirror behavior** — Python decorators, Node functions, C# attributes must produce the same mount paths and OpenAPI shapes.
3. **Do not duplicate business logic in three languages** — only binding-specific glue (decorators, reflection, module registration).

## Key entry points

- **Python**: `FusionApp` in `crates/fusion-py/python/fusion_framework/app.py`; route registry in `crates/fusion-py/src/api_types.rs`.
- **Node**: `FusionApp`, `route()`, `httpGet()` in `crates/fusion-node/index.js`.
- **C#**: `FusionApp`, `Route.Register<T>()`, attributes in `bindings/csharp/FusionFramework/`.

## When adding a feature

1. Identify if it is cross-binding (yes → start in `fusion-core`).
2. Implement mount + OpenAPI in all three bindings in one PR when possible.
3. Add or extend an example under `examples/`.
