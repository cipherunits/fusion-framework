# Fusion Framework

Class-based HTTP APIs with one shared Rust core and first-class bindings for **Python**, **Node.js**, and **C#**. Routes, middleware, OpenAPI/Swagger, cache, templates, and background tasks share the same semantics across languages — write once in `fusion-core`, expose through each binding.

| | |
|---|---|
| **Docs** | [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/) |
| **CLI / scaffolds** | [cipherunits/fusion-tool](https://github.com/cipherunits/fusion-tool) |
| **License** | [BSD 3-Clause](LICENSE) |
| **Version** | `2.0.1` |

## Why this repo

Most multi-language frameworks drift: each binding invents its own router rules and OpenAPI shapes. Fusion keeps shared behavior in Rust (`crates/fusion-core`) and thin glue in each language, so `api/[module]/{id}`, versioned Swagger, and middleware chains stay aligned.

## Install (applications)

Use published packages for app projects. Scaffold with Fusion Tool when you want a full project tree:

```bash
# CLI
# https://github.com/cipherunits/fusion-tool
fusion init --lang python --name my-app   # or node / csharp
```

| Language | Package |
|----------|---------|
| Python | `pip install fusion-framework` |
| Node.js | `npm install fusion-framework` |
| C# | `dotnet add package Fusion-Framework` |

Binding-specific quick starts live next to the packages:

- [Python](crates/fusion-py/README.md)
- [Node](crates/fusion-node/README.md)
- [C#](bindings/csharp/FusionFramework/README.md)

## Repository layout

```text
crates/fusion-core/     Shared Rust: routing, HTTP, cache, templates, tasks
crates/fusion-py/       Python (PyO3) + fusion_framework package
crates/fusion-node/     Node (N-API) package published as fusion-framework
crates/fusion-ffi/      C ABI used by the C# binding
bindings/csharp/        Managed C# API (NuGet: Fusion-Framework)
examples/               Side-by-side demos (.py / .mjs / .cs)
tests/                  Executable tests (not inside installable packages)
scripts/                Dev install + version bump helpers
.agents/skills/         Agent skills for contributors working in this repo
```

## Develop from source

Prerequisites: Rust (stable), Python 3.9+, Node 18+, .NET SDK matching the C# target (`net10.0`).

From the repo root:

```bash
./scripts/dev-install-python.sh --create-venv   # or --venv .venv
./scripts/dev-install-node.sh
./scripts/dev-install-csharp.sh
# or all three:
./scripts/dev-install-all.sh
```

Details: [scripts/README.md](scripts/README.md).

### Tests

```bash
./tests/scripts/run-all.sh
```

Per binding and layout notes: [tests/README.md](tests/README.md).

### Examples

Runnable samples under `examples/` mirror public APIs across languages (routing, cache, middleware, templates, static files, and more). Prefer adding a trio (`.py` / `.mjs` / `.cs`) when you introduce a new cross-binding feature.

## Contributing

### Cross-binding parity

If a feature is user-facing in one binding (routes, middleware, permissions, settings, Swagger, UI helpers, …), implement it in **Python, Node, and C#** in the same change set unless the change is explicitly scoped. Put shared semantics in `fusion-core` first.

### Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(optional-scope): <short summary>

feat(cache): add get_or_set helper
fix(node): restore swagger navbar versions
docs: expand root README install section
chore(release): bump to 2.0.2
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`, `release`.

Optional local check (POSIX shell, no npm):

```bash
git config core.hooksPath .githooks
```

That installs [`.githooks/commit-msg`](.githooks/commit-msg). Editor defaults: [`.editorconfig`](.editorconfig).
### Staging

Stage files by path (`git add path/to/file`). Do not use `git add .` / `git add -A` in this repository.

## Links

- Site & docs: https://fusion.cipherunit.xyz/
- Issues: https://github.com/cipherunits/fusion-framework/issues
- Fusion Tool (CLI): https://github.com/cipherunits/fusion-tool
- Author: [CipherUnits](https://cipherunit.xyz)

## License

BSD 3-Clause. See [LICENSE](LICENSE).
