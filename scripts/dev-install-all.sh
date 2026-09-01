#!/usr/bin/env bash
# Install all Fusion bindings from source (Python, Node, C#).
#
# Usage (from repo root):
#   ./scripts/dev-install-all.sh
#   ./scripts/dev-install-all.sh --create-venv
set -euo pipefail

# shellcheck source=_dev-common.sh
source "$(cd "$(dirname "$0")" && pwd)/_dev-common.sh"

CREATE_VENV=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --create-venv)
      CREATE_VENV=1
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Run all local dev install scripts in order: Python, Node, C#.

Options:
  --create-venv   Pass through to dev-install-python.sh
  -h, --help      Show this help
EOF
      exit 0
      ;;
    *)
      shift
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

info "1/3 Python"
if [[ "$CREATE_VENV" -eq 1 ]]; then
  bash "$SCRIPT_DIR/dev-install-python.sh" --create-venv
else
  bash "$SCRIPT_DIR/dev-install-python.sh"
fi

echo ""
info "2/3 Node"
bash "$SCRIPT_DIR/dev-install-node.sh"

echo ""
info "3/3 C#"
bash "$SCRIPT_DIR/dev-install-csharp.sh"

ok "all bindings installed from source"

cat <<'EOF'

Quick verify:
  python -c "from fusion_framework import settings; print('python ok')"
  node -e "const f=require('fusion-framework'); console.log('node ok', f.status.HTTP_SUCCESS)"
  export FUSION_FFI_PATH=$PWD/target/debug/libfusion_ffi.so  # adjust extension on macOS/Windows
  dotnet build bindings/csharp/FusionFramework/FusionFramework.csproj
EOF
