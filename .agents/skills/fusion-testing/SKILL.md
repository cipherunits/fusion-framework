---
name: fusion-testing
description: >-
  Runs Fusion Framework verification across Rust, Python, Node, and C#. Use
  after code changes, before commits, or when CI-like validation is needed
  locally.
---

# Testing & Verification

## Full suite (recommended)

```bash
./tests/scripts/run-all.sh
```

## Quick smoke (after route/API changes)

```bash
cargo test -p fusion-core naming
cargo check -p fusion-py
node --check crates/fusion-node/index.js
dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj
./tests/scripts/run-python.sh -q
```

## Python (pytest)

Tests live under `tests/python/` (not inside the installable package).

```bash
./scripts/dev-install-python.sh --venv .venv
source .venv/bin/activate
pytest                          # uses pytest.ini at repo root
pytest tests/python/unit/test_http_route.py -q
```

## Full Rust workspace

```bash
cargo check --workspace
cargo test --workspace
```

## Binding-specific

| Binding | Command |
|---------|---------|
| Python | `pytest tests/python` (after `dev-install-python.sh`) |
| Node | `node --check crates/fusion-node/index.js` |
| C# | `dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj` |

## Layout

See `tests/README.md` for folder structure and conventions.

## What to exclude from git

- `bindings/csharp/**/bin/`, `obj/`
- `target/`, `node_modules/`, `__pycache__/`, `.pytest_cache/`
- Local `.pdb` changes from debug builds

## When tests fail

1. Fix `fusion-core` first if naming/route tests fail.
2. Then fix Python registry (`api_types.rs`) — often the reference implementation.
3. Align Node and C# mount/OpenAPI with Python behavior.
