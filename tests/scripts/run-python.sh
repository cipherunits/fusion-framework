#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

if ! python3 -c "import fusion_framework" 2>/dev/null; then
  echo "fusion_framework not installed — run: ./scripts/dev-install-python.sh --venv .venv" >&2
  exit 1
fi

python3 -m pytest tests/python "$@"
