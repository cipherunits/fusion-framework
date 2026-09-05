#!/usr/bin/env bash
# Build and link fusion-framework (Node) from this repo.
#
# Usage (from repo root):
#   ./scripts/dev-install-node.sh
#
# Verify:
#   node -e "const f=require('fusion-framework'); console.log(f.status.HTTP_SUCCESS)"
set -euo pipefail

# shellcheck source=_dev-common.sh
source "$(cd "$(dirname "$0")" && pwd)/_dev-common.sh"
cd "$ROOT"

LINK=1
PROFILE=debug

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-link)
      LINK=0
      shift
      ;;
    --release)
      PROFILE=release
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Build the Node native addon and optionally npm link it globally.

Options:
  --no-link    Build only; skip npm link
  --release    Release build (default: debug)
  -h, --help   Show this help

Examples:
  ./scripts/dev-install-node.sh
  node examples/node_hello.mjs
EOF
      exit 0
      ;;
    *)
      shift
      ;;
  esac
done

require_cmd node
require_cmd npm
require_cmd cargo

NODE_DIR="$ROOT/crates/fusion-node"
cd "$NODE_DIR"

info "installing npm devDependencies"
npm install

if [[ "$PROFILE" == "release" ]]; then
  info "building native addon (release)"
  npm run build:napi
else
  info "building native addon (debug)"
  npm run build:napi:debug
fi

info "smoke test (local require)"
node -e "const f=require('./index.js'); if(!f.FusionApp||!f.status) throw new Error('smoke failed'); console.log('ok', f.status.HTTP_SUCCESS)"

if [[ "$LINK" -eq 1 ]]; then
  info "npm link (global fusion-framework -> this repo)"
  if npm link 2>/dev/null; then
    ok "linked package name 'fusion-framework'"
    VERIFY="node -e \"const f=require('fusion-framework'); console.log('ok', f.status.HTTP_SUCCESS)\""
  else
    warn "npm link failed (permission denied?). Use the local path instead:"
    VERIFY="node -e \"const f=require('./crates/fusion-node/index.js'); console.log('ok', f.status.HTTP_SUCCESS)\""
    echo "  cd crates/fusion-node && npm link   # retry with sudo or a user npm prefix"
  fi
else
  ok "built at crates/fusion-node (not linked)"
  VERIFY="node -e \"const f=require('./crates/fusion-node/index.js'); console.log('ok', f.status.HTTP_SUCCESS)\""
fi

cat <<EOF

Next steps:
  $VERIFY
  node examples/node_hello.mjs
  node examples/pagination.mjs

From another project:
  npm link fusion-framework
EOF
