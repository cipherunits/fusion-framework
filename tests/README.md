# Fusion Framework tests

Central test layout for the monorepo. Binding code stays in `crates/` and `bindings/`; **executable tests live here**.

## Layout

```
tests/
├── python/           # pytest — primary binding test suite
│   ├── conftest.py   # route/middleware isolation per test
│   └── unit/         # fast, no live server
├── fixtures/         # shared JSON, templates, sample apps
└── scripts/          # local runners (CI uses workflow steps)
```

| Layer | Location | Runner |
|-------|----------|--------|
| Rust core | `crates/fusion-core/src/**/*.rs` (`#[cfg(test)]`) | `cargo test -p fusion-core` |
| Python binding | `tests/python/` | `pytest` (see below) |
| Node | syntax + smoke in CI | `node --check crates/fusion-node/index.js` |
| C# | build in CI | `dotnet build bindings/csharp/...` |

## Python (pytest)

Install the extension from the working tree first:

```bash
./scripts/dev-install-python.sh --venv .venv
source .venv/bin/activate
```

Run all Python tests:

```bash
pytest
# or
./tests/scripts/run-python.sh
```

Run one file:

```bash
pytest tests/python/unit/test_http_route.py -q
```

## Full local verification

```bash
./tests/scripts/run-all.sh
```

## Writing tests

- Put new Python tests under `tests/python/unit/` (or `tests/python/integration/` when you need a real server).
- Do **not** add `test_*.py` under `crates/fusion-py/python/fusion_framework/` — that package is for runtime code only.
- Route-registered classes leak global state; `conftest.py` clears the registry automatically — avoid extra `setup_function` unless a test needs special setup mid-file.
- Prefer small fixtures in `tests/fixtures/` over duplicating JSON/HTML in every test file.
