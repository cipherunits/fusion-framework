# Fusion Framework tests

Central test layout for the monorepo. Binding code stays in `crates/` and `bindings/`; **executable tests live here**.

## Layout

```
tests/
├── python/           # pytest
├── node/             # node --test
├── csharp/           # dotnet test (xUnit)
├── fixtures/
└── scripts/
```

| Layer | Location | Runner |
|-------|----------|--------|
| Rust core | `crates/fusion-core/src/**/*.rs` (`#[cfg(test)]`) | `cargo test -p fusion-core` |
| Python binding | `tests/python/` | `pytest` |
| Node binding | `tests/node/` | `node --test` (see below) |
| C# binding | `tests/csharp/` | `dotnet test` |

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

## Node (`node --test`)

Build the native addon first:

```bash
cd crates/fusion-node && npm install && npm run build:debug
```

Run tests:

```bash
./tests/scripts/run-node.sh
```

## C# (xUnit)

```bash
./tests/scripts/run-csharp.sh
```

Requires `fusion_ffi` built (`cargo build -p fusion-ffi`).

## Full local verification

```bash
./tests/scripts/run-all.sh
```

## Writing tests

- Put new Python tests under `tests/python/unit/` (or `tests/python/integration/` when you need a real server).
- Do **not** add `test_*.py` under `crates/fusion-py/python/fusion_framework/` — that package is for runtime code only.
- Route-registered classes leak global state; `conftest.py` clears the registry automatically — avoid extra `setup_function` unless a test needs special setup mid-file.
- Prefer small fixtures in `tests/fixtures/` over duplicating JSON/HTML in every test file.
