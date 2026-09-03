---
name: fusion-testing
description: >-
  Runs Fusion Framework verification across Rust, Python, Node, and C#. Use
  after code changes, before commits, when CI-like validation is needed, or
  when tests fail and need root-cause investigation.
---

# Testing & Verification

## Prefer writing tests

New behavior should land with tests when practical:

| Layer | Where |
|-------|--------|
| Rust shared logic | `#[cfg(test)]` in `crates/fusion-core` (and related crates) |
| Python | `tests/python/unit/` (pytest) |
| Node | `tests/node/unit/*.test.js` (`node --test`) |
| C# | `tests/csharp/FusionFramework.Tests/` (xUnit) |

Do **not** put `test_*.py` under `crates/fusion-py/python/fusion_framework/`.

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

```bash
./scripts/dev-install-python.sh --venv .venv
source .venv/bin/activate
pytest
./tests/scripts/run-python.sh
pytest tests/python/unit/test_http_route.py -q
```

## Node

```bash
cd crates/fusion-node && npm install && npm run build:debug
./tests/scripts/run-node.sh
```

## C#

```bash
./tests/scripts/run-csharp.sh
# builds fusion-ffi then: dotnet test … -c Release
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
| Node | `./tests/scripts/run-node.sh` |
| C# | `./tests/scripts/run-csharp.sh` |

## Layout

See `tests/README.md` for folder structure and conventions.

## When tests fail (required)

1. **Investigate** — read assertion, stack, expected vs actual.
2. **Explain** — tell the user the root cause (e.g. class name → `[module]` stripping, async middleware not awaited, CORS header casing, version prefix missing).
3. **Fix** — correct implementation or wrong expectation; do not hide failures.
4. Order of suspicion for route/OpenAPI issues:
   - `fusion-core` naming
   - Python registry (`api_types.rs`) as reference
   - Node / C# mount + OpenAPI fill

## What to exclude from git

- `bindings/csharp/**/bin/`, `obj/`
- `target/`, `node_modules/`, `__pycache__/`, `.pytest_cache/`
- Local `.pdb` changes from debug builds
- Scratch `.nuget/` / `.tmp/` caches if created locally
