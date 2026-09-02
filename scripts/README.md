# Local development install scripts

Shell helpers for building and linking **Fusion Framework** bindings directly from this repository. Use them when you are changing Rust core code or binding glue and want to run examples or tests against your working tree instead of published packages.

Published installs remain the default for application projects:

| Binding | Published install |
|---------|-------------------|
| Python | `pip install fusion-framework` |
| Node | `npm install fusion-framework` |
| C# | `dotnet add package Fusion-Framework` |

## Overview

| Script | What it does |
|--------|----------------|
| `dev-install-python.sh` | Editable Python install via `maturin develop` |
| `dev-install-node.sh` | Builds the N-API addon and optionally `npm link`s it |
| `dev-install-csharp.sh` | Builds `fusion-ffi` and the `FusionFramework` project |
| `dev-install-all.sh` | Runs the three scripts above in order |

All scripts are intended to be run **from the repository root**:

```bash
./scripts/dev-install-python.sh
./scripts/dev-install-node.sh
./scripts/dev-install-csharp.sh
# or
./scripts/dev-install-all.sh
```

Shared helpers live in `_dev-common.sh` (sourced by the install scripts; do not run it directly).

## Prerequisites

Install these once on your machine before running the scripts.

| Tool | Required for | Notes |
|------|--------------|-------|
| **Rust** (`cargo`, stable toolchain) | All bindings | Builds `fusion-core`, `fusion-py`, `fusion-node`, and `fusion-ffi` |
| **Python 3.9+** with `pip` | Python | A virtualenv is recommended (`.venv` in the repo root) |
| **maturin** | Python | Installed automatically by `dev-install-python.sh` if missing |
| **Node.js 18+** and **npm** | Node | See `crates/fusion-node/package.json` `engines` |
| **.NET SDK 10.0** | C# | Project targets `net10.0`; install the matching SDK |

Platform build tools (a C/C++ linker, `python3-dev` headers on Linux, etc.) are required for Rust/PyO3/N-API builds—the same toolchain you would use for `cargo build` in this workspace.

## Per-script usage

### Python — `dev-install-python.sh`

Builds the PyO3 extension and installs the `fusion_framework` package in **editable** mode into the active environment.

```bash
# Use the current shell's Python (or repo .venv if present)
./scripts/dev-install-python.sh

# Create .venv and install into it
./scripts/dev-install-python.sh --create-venv

# Use a specific virtualenv
./scripts/dev-install-python.sh --venv .venv
```

**Options**

| Flag | Description |
|------|-------------|
| `--create-venv` | Create `.venv` at the repo root if it does not exist |
| `--venv PATH` | Use `PATH/bin/python` and `PATH/bin/maturin` |
| `-h`, `--help` | Show usage |

After install, activate the venv if you use one:

```bash
source .venv/bin/activate   # bash/zsh
```

### Node — `dev-install-node.sh`

Installs npm devDependencies, compiles the native `.node` addon with `@napi-rs/cli`, runs a local smoke test, and by default runs **`npm link`** so other projects can `require('fusion-framework')` from this build.

```bash
./scripts/dev-install-node.sh
```

**Options**

| Flag | Description |
|------|-------------|
| `--no-link` | Build only; skip `npm link` |
| `--release` | Release build (default is debug / `build:debug`) |
| `-h`, `--help` | Show usage |

To consume the linked package from another project:

```bash
npm link fusion-framework
```

To require the addon without linking, use the path `crates/fusion-node/index.js` from the repo root.

### C# — `dev-install-csharp.sh`

Builds the native `fusion-ffi` library and compiles `bindings/csharp/FusionFramework`. Prints the path to the shared library for `FUSION_FFI_PATH` when auto-discovery is not enough.

```bash
./scripts/dev-install-csharp.sh
```

**Options**

| Flag | Description |
|------|-------------|
| `--release` | Release build for both Cargo and `dotnet build` |
| `--pack` | Also run `dotnet pack` and write packages to `dist/` |
| `-h`, `--help` | Show usage |

Reference the project from an app (as in `examples/csharp_hello`):

```xml
<ProjectReference Include="../../bindings/csharp/FusionFramework/FusionFramework.csproj" />
```

If the native library is not found at runtime, export:

```bash
# Linux
export FUSION_FFI_PATH="$PWD/target/debug/libfusion_ffi.so"

# macOS
export FUSION_FFI_PATH="$PWD/target/debug/libfusion_ffi.dylib"

# Windows (PowerShell)
$env:FUSION_FFI_PATH = "$PWD\target\debug\fusion_ffi.dll"
```

### All bindings — `dev-install-all.sh`

Runs Python, then Node, then C# installs in sequence. Useful after a fresh clone or a large `fusion-core` change.

```bash
./scripts/dev-install-all.sh
./scripts/dev-install-all.sh --create-venv   # also create .venv for Python
```

Individual script flags (for example `--release` or `--no-link`) are not forwarded; run the per-binding scripts directly when you need those options.

## Verification

Each script runs a small smoke check before it finishes. After install, you can verify manually:

**Python**

```bash
python -c "from fusion_framework import settings; print('python ok')"
python examples/template_demo.py
python -m pytest tests/python -q
```

**Node**

```bash
node -e "const f=require('fusion-framework'); console.log('node ok', f.status.HTTP_SUCCESS)"
node examples/node_hello.mjs
node --check crates/fusion-node/index.js
```

**C#**

```bash
export FUSION_FFI_PATH="$PWD/target/debug/libfusion_ffi.so"   # adjust per OS
dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj
dotnet build examples/csharp_hello/csharp_hello.csproj
dotnet run --project examples/csharp_hello
```

**Rust core** (optional, independent of the install scripts):

```bash
cargo test -p fusion-core
cargo check --workspace
```

## Troubleshooting

### `missing required command: …`

Install the tool listed in the error (see [Prerequisites](#prerequisites)). On Debian/Ubuntu, `python3-venv` may be required for `python3 -m venv .venv`.

### Python: `maturin` or build failures

- Use a dedicated venv: `./scripts/dev-install-python.sh --create-venv`
- Ensure Python headers are installed (`python3-dev` on Linux).
- Re-run after `cargo clean` if the PyO3 extension is stale.

### Node: `smoke failed` or missing `.node` file

- Confirm Rust is installed and `napi build` completed without errors.
- Do not run `napi build` without `--js false`; it can overwrite `index.js`. The npm scripts in `crates/fusion-node` already pass the correct flags.
- If `npm link` causes confusion, rebuild with `./scripts/dev-install-node.sh --no-link` and require `./crates/fusion-node/index.js` directly.

### C#: `DllNotFoundException` / could not load `fusion_ffi`

- Run `./scripts/dev-install-csharp.sh` (or `cargo build -p fusion-ffi`) so `target/debug/` contains the native library.
- Set `FUSION_FFI_PATH` to the absolute path printed by the script.
- The binding also searches upward from the app output directory for `target/debug` and `target/release`; keep your app inside or near this monorepo, or use a `ProjectReference` as in the examples.

### C#: SDK or TFM errors

- Install **.NET SDK 10.0** to match `net10.0` in `FusionFramework.csproj`, or retarget the project locally if you must use an older SDK.

### Slow or repeated full rebuilds

- Run only the script for the binding you are working on.
- Use debug builds (default) during development; pass `--release` when you need optimized native code.

### Permission denied running a script

Make scripts executable once:

```bash
chmod +x scripts/dev-install-*.sh
```

## Related scripts

- `set-version.sh` — bump version numbers across Cargo, Python, Node, and C# manifests (release tooling; not part of local dev install).
