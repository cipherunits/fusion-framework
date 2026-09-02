#!/usr/bin/env bash
# Run the full Fusion Framework test suite (local).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "==> Rust (fusion-core)"
cargo test -p fusion-core

echo "==> Node syntax"
node --check crates/fusion-node/index.js

echo "==> C# build"
dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj --no-restore 2>/dev/null \
  || dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj

echo "==> Python (pytest)"
if ! python3 -c "import fusion_framework" 2>/dev/null; then
  echo "    fusion_framework not installed — run: ./scripts/dev-install-python.sh --venv .venv"
  exit 1
fi
python3 -m pytest tests/python -q

echo "==> All checks passed"
