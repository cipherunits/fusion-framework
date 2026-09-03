---
name: fusion-release
description: >-
  Bumps Fusion Framework version across Cargo, Python, Node, and C# manifests
  using scripts/set-version.sh. Use when preparing releases or syncing version
  numbers across bindings.
---

# Release & Versioning

## Single source script

```bash
./scripts/set-version.sh 1.2.4
```

Updates (when present):

- Root and crate `Cargo.toml` / `Cargo.lock`
- `crates/fusion-py/pyproject.toml`
- `crates/fusion-node/package.json`
- `bindings/csharp/FusionFramework/FusionFramework.csproj`
- Header/version fallbacks referenced in the script

## Workflow

1. Run `set-version.sh` with the new semver.
2. Run full checks (see `fusion-testing` skill).
3. Commit with message like `release vX.Y.Z` (match recent history).
4. **Do not push** unless the user explicitly asks.

## Rules

- Keep all bindings on the **same version** for a release.
- Do not commit `bin/`, `obj/`, or `.pdb` artifacts from local `dotnet build`.
- Changelog/README updates only when the user requests documentation.
- Stage release files **individually** (never `git add .`).
- After a release that changes public APIs used by scaffolds, note whether fusion-tool’s `FUSION_FRAMEWORK_VERSION` / templates need a bump (see `fusion-cli`).
