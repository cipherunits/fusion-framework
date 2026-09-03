#!/usr/bin/env bash
# Install fusion-framework (Python) from this repo into a local environment.
#
# Usage (from repo root):
#   ./scripts/dev-install-python.sh
#   ./scripts/dev-install-python.sh --venv .venv
#
# Verify:
#   python -c "from fusion_framework import settings; print('ok')"
set -euo pipefail

# shellcheck source=_dev-common.sh
source "$(cd "$(dirname "$0")" && pwd)/_dev-common.sh"
cd "$ROOT"

PYTHON="${PYTHON:-}"
MATURIN="${MATURIN:-}"
CREATE_VENV=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --venv)
      PYTHON="$2/bin/python"
      MATURIN="$2/bin/maturin"
      shift 2
      ;;
    --create-venv)
      CREATE_VENV=1
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Install the Python binding from source (maturin develop).

Options:
  --venv PATH       Use Python/maturin from PATH/bin
  --create-venv     Create .venv if missing, then install into it
  -h, --help        Show this help

Examples:
  ./scripts/dev-install-python.sh
  ./scripts/dev-install-python.sh --create-venv
  ./scripts/dev-install-python.sh --venv .venv
EOF
      exit 0
      ;;
    *)
      shift
      ;;
  esac
done

if [[ "$CREATE_VENV" -eq 1 && ! -x "$ROOT/.venv/bin/python" ]]; then
  info "creating virtualenv at .venv"
  require_cmd python3
  python3 -m venv "$ROOT/.venv"
  PYTHON="$ROOT/.venv/bin/python"
  MATURIN="$ROOT/.venv/bin/maturin"
fi

if [[ -z "$PYTHON" ]]; then
  if [[ -n "${VIRTUAL_ENV:-}" ]]; then
    PYTHON="${VIRTUAL_ENV}/bin/python"
    MATURIN="${VIRTUAL_ENV}/bin/maturin"
  elif [[ -x "$ROOT/.venv/bin/python" ]]; then
    PYTHON="$ROOT/.venv/bin/python"
    MATURIN="$ROOT/.venv/bin/maturin"
  else
    PYTHON="$(command -v python3 || true)"
    MATURIN="$(command -v maturin || true)"
  fi
fi

[[ -n "$PYTHON" && -x "$PYTHON" ]] || die "no Python found (create .venv: python3 -m venv .venv)"

if [[ -z "$MATURIN" || ! -x "$MATURIN" ]]; then
  info "installing maturin"
  "$PYTHON" -m pip install -q maturin
  MATURIN="$("$PYTHON" -c 'import shutil; print(shutil.which("maturin") or "")')"
fi
[[ -n "$MATURIN" && -x "$MATURIN" ]] || die "maturin not available"

info "building and installing fusion-framework (editable)"
"$MATURIN" develop --manifest-path crates/fusion-py/Cargo.toml

PREFIX="$("$PYTHON" -c 'import sys; print(sys.prefix)')"
ok "Python binding installed into $PREFIX (editable)"

info "smoke test"
"$PYTHON" -c "from fusion_framework import settings; from fusion_framework.template import FusionBaseTemplate; print('ok')"

cat <<EOF

Next steps:
  source .venv/bin/activate    # if you use the repo .venv
  python examples/template_demo.py
  python3 -m pytest tests/python -q
EOF
