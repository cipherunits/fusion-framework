---
name: fusion-testing
description: >-
  Runs Fusion Framework verification across Rust, Python, Node, and C#. Use
  after code changes, before commits, or when CI-like validation is needed
  locally.
---

# Testing & Verification

## Quick smoke (after route/API changes)

```bash
cargo test -p fusion-core naming
cargo check -p fusion-py
node --check crates/fusion-node/index.js
dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj --no-restore 2>/dev/null || dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj
python -m pytest crates/fusion-py/python/fusion_framework/test_http_route.py -q
```

## Full Rust workspace

```bash
cargo check --workspace
cargo test --workspace
```

## Binding-specific

| Binding | Command |
|---------|---------|
| Python package | `cd crates/fusion-py && maturin develop` (if building extension) |
| Node | `node --check crates/fusion-node/index.js` |
| C# | `dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj` |

## What to exclude from git

- `bindings/csharp/**/bin/`, `obj/`
- `target/`, `node_modules/`, `__pycache__/`
- Local `.pdb` changes from debug builds

## When tests fail

1. Fix `fusion-core` first if naming/route tests fail.
2. Then fix Python registry (`api_types.rs`) — often the reference implementation.
3. Align Node and C# mount/OpenAPI with Python behavior.
