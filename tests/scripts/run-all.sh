#!/usr/bin/env bash
# Run the full Fusion Framework test suite (local).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "==> Rust (fusion-core)"
cargo test -p fusion-core

echo "==> Node syntax"
node --check crates/fusion-node/index.js

echo "==> Node unit tests"
if ls crates/fusion-node/*.node >/dev/null 2>&1; then
  ./tests/scripts/run-node.sh
else
  echo "    skip: build addon with (cd crates/fusion-node && npm run build:debug)"
fi

echo "==> C# tests"
cargo build -p fusion-ffi --release -q
./tests/scripts/run-csharp.sh -q

echo "==> Python (pytest)"
if ! python3 -c "import fusion_framework" 2>/dev/null; then
  echo "    fusion_framework not installed — run: ./scripts/dev-install-python.sh --venv .venv"
  exit 1
fi
python3 -m pytest tests/python -q

echo "==> All checks passed"
