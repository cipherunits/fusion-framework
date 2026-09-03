---
name: fusion-architecture
description: >-
  Explains Fusion Framework repository layout, crate boundaries, where new
  logic belongs (fusion-core vs Python/Node/C# bindings), tests layout, and
  relationship to the fusion CLI. Use when navigating the codebase, adding
  features, or deciding which layer to change.
---

# Fusion Architecture

## Repository map

| Path | Role |
|------|------|
| `crates/fusion-core/` | Shared Rust: naming, route tokens, HTTP conventions, settings helpers |
| `crates/fusion-py/` | Python binding (PyO3) + `python/fusion_framework/` package |
| `crates/fusion-node/` | Node binding (`index.js`, N-API) |
| `crates/fusion-ffi/` | C ABI for the C# binding |
| `bindings/csharp/FusionFramework/` | C# binding (NuGet layout source of truth) |
| `tests/` | Executable tests (Python / Node / C#) — not inside installable packages |
| `examples/` | Runnable samples per binding |
| `scripts/` | Dev install, version bumps (`set-version.sh`) |
| `.agents/skills/` | Agent skills for this repo |

**Related external repo:** [fusion-tool](https://github.com/cipherunits/fusion-tool) — `fusion` CLI that scaffolds apps. See `fusion-cli` skill.

## Layering rules

1. **Put shared semantics in `fusion-core`** — route token resolution (`[module]`, `[action]`), path joining, handler naming. Bindings should call Rust helpers via FFI where possible.
2. **Bindings mirror behavior** — Python decorators, Node functions, C# attributes must produce the same mount paths and OpenAPI shapes.
3. **Do not duplicate business logic in three languages** — only binding-specific glue (decorators, reflection, module registration).
4. **Tests live under `tests/`** — do not add `test_*.py` inside `fusion_framework/` package sources.

## Key entry points

- **Python**: `FusionApp` in `crates/fusion-py/python/fusion_framework/app.py`; route registry in `crates/fusion-py/src/api_types.rs`.
- **Node**: `FusionApp`, `route()`, `httpGet()` in `crates/fusion-node/index.js`.
- **C#**: `FusionApp`, `Route.Register<T>()`, attributes in `bindings/csharp/FusionFramework/`.

## When adding a feature

1. Identify if it is cross-binding (yes → start in `fusion-core` when semantics are shared).
2. Implement in **Python, Node, and C#** in one change set (see `fusion-bindings-parity`).
3. Add tests under `tests/` (preferred) and/or Rust unit tests.
4. Add **usage examples in all three languages** under `examples/` (`<feature>.py` / `.mjs` / `.cs`) so the API shape is visible.
5. If scaffolds or env JSON contracts change, update the `fusion-cli` skill and consider fusion-tool templates.
6. Comment new functions; document dense logic (see `fusion-coding-standards`).
