#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/crates/fusion-node"

if ! ls ./*.node >/dev/null 2>&1; then
  echo "Native addon missing — run: npm run build:debug (in crates/fusion-node)" >&2
  exit 1
fi

node --test "$ROOT/tests/node/unit/"*.test.js "$@"
