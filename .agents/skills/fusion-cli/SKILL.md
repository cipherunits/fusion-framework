---
name: fusion-cli
description: >-
  Documents the Fusion Tool CLI (fusion init, command, module, add, update),
  the scaffolded project tree, and how the CLI relates to this framework repo.
  Use when explaining project layout, scaffolding, env JSON, or when framework
  API changes must stay compatible with fusion-tool generators.
---

# Fusion CLI (fusion-tool)

Apps are usually created with **Fusion Tool** (`fusion`), a separate repo:
https://github.com/cipherunits/fusion-tool

This skill describes the CLI from the **framework** side so agents know what
generated projects look like and what must stay compatible.

## Install & entry

```bash
fusion --help
fusion --version
```

Binary name: `fusion`. Source of truth for generators: `fusion-tool` (`src/command/`, `src/setting/structure.rs`, `src/setting/environment.rs`).

## Commands overview

| Command | Purpose |
|---------|---------|
| `fusion init` | Scaffold a new Fusion app (Python / TypeScript / ASP.NET Core) |
| `fusion command <name>` | Run a named command from `fusion.<env>.json` |
| `fusion load-env` | Load `fusion.<env>.json` into the process environment |
| `fusion module init` | Scaffold a **publishable library package** (not an app route module) |
| `fusion add --github OWNER/REPO` | Vendor a module into the current app |
| `fusion update` | Self-update the CLI binary |

### `fusion init`

```bash
fusion init
fusion init my-app
fusion init --lang python --name myproject --description "…"
```

| Flag / arg | Values |
|------------|--------|
| `[DIRECTORY]` | Target dir (created if missing; default = cwd) |
| `--lang` | `python`, `typescript`, `asp-core` |
| `--name` | Project name |
| `--description` | Short description |

Writes `fusion-framework.toml`, `fusion.{dev,stage,prod}.json`, `.gitignore`, language entrypoint, sample route module, templates, and dependency pins to the framework version.

### `fusion command`

Commands live under the `commands` object in `fusion.<env>.json`.

```bash
fusion command run                 # default env: FUSION_ENV or `dev`
fusion command run --stage
fusion command run:stage           # same
fusion command run --prod
fusion command run --env test
fusion command --stage             # list commands for that env
```

Runs via the shell from the project root with `FUSION_ENV` set so `core/settings` loads the matching file.

### Modules vs route modules

| Term | Meaning |
|------|---------|
| Route module | App code: `FusionBaseApi` / template under `src/modules/…` |
| Library module | Separate package from `fusion module init` (`fusion.module.toml`), installed with `fusion add` |

Do not confuse the two when naming APIs or writing docs.

### `fusion module init` / `fusion add`

```bash
fusion module init --lang python --name example --description "…"
fusion add --github OWNER/MODULE_NAME
fusion add --github OWNER/MODULE_NAME@v1.0.0
```

Vendors under `.fusion/modules/<id>/` and records `[[modules]]` in `fusion-framework.toml`.

## Scaffolded app layout (`fusion init`)

Python shown; TypeScript/C# use the same tree with language extensions.

```text
<project>/
├── core/
│   └── settings.py          # Overlay (RELOAD, TEMPLATES_DIR, …)
├── src/
│   └── modules/
│       └── products/
│           └── products.py  # HomePage (template) + ProductModule (API)
├── templates/
│   └── home/
│       ├── index.html
│       └── style.css
├── main.py                  # Register middleware + FusionApp.listen()
├── requirements.txt         # Python pin (or package.json / *.csproj)
├── pyproject.toml           # Python only
├── fusion-framework.toml    # Project metadata + tool/framework versions
├── fusion.dev.json          # env=dev, port 8080, swagger on, reload
├── fusion.stage.json        # port 8081
├── fusion.prod.json         # port 9090
└── .gitignore
```

TypeScript: `main.ts`, `core/settings.ts`, `package.json`, `tsconfig.json`.  
C# (`asp-core`): `main.cs`, `*.csproj` (`net10.0`), `[Route]` / `[HttpGet]`.

### What the starter demonstrates

- `FusionBaseTemplate` at `/` (Tera templates; **not** listed in Swagger).
- Welcome UI via built-in components: `fusion.badge`, `fusion.button`, `fusion.card`, `fusion.table` (optional `page_size={10}` for client-side row pagination; styles: `{% include "fusion/components.css" %}`).
- `FusionBaseApi` at `api/[module]` with `version="v1"` → `/v1/api/product/…`.
- Convention verbs (`get` / `post` / …) plus one custom slot (`http_get` / `httpGet` / `[HttpGet]` with `[action]`).
- Opt-in middleware list in `main` (e.g. `request_id`, `cors`, `cache_headers`, `security_headers`, `framework_headers`). Framework does **not** auto-enable middleware; the scaffold opts in.
- Application cache defaults to **moka** (`cache` block in env JSON); see `fusion-cache` skill.
- Cache monitor (`monitor.enabled`) is on in **dev** and off in **stage/prod**; when off, no monitor HTTP endpoints are registered.

### Default ports

| Env | Port |
|-----|------|
| dev | 8080 |
| stage | 8081 |
| prod | 9090 |

### Environment JSON shape

```json
{
  "env": "dev",
  "config": {
    "host": "127.0.0.1",
    "port": 8080,
    "debug": true,
    "fingerprint": { "enabled": false },
    "swagger": { "enabled": true, "path": "/swagger" }
  },
  "commands": {
    "run": "python main.py"
  }
}
```

`FUSION_ENV` selects `fusion.<env>.json` (default `dev`). Unresolved `HOST` placeholders must not crash listen — framework resolves safe defaults.

## Compatibility duties (framework ↔ CLI)

When changing public Fusion APIs used by scaffolds:

1. Prefer keeping generated starter patterns working (or update **fusion-tool** templates in a follow-up / paired PR).
2. Do not invent decorators/config keys that only exist in one binding.
3. After middleware / route / settings changes, check whether `fusion-tool` `structure.rs` / `environment.rs` comments or defaults need updates.
4. Version pin in the CLI (`FUSION_FRAMEWORK_VERSION`) is separate from this repo’s version bump (`./scripts/set-version.sh`).

## Related

- Framework layout: `fusion-architecture`
- Binding alignment: `fusion-bindings-parity`
- Routes: `fusion-http-routes`
- CLI repo skills (if editing the tool itself): fusion-tool `.agents/skills/fusion-cli*`
