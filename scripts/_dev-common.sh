# Shared helpers for local dev install scripts. Source from bash only.
if [[ -z "${BASH_VERSION:-}" ]]; then
  echo "scripts must be run with bash" >&2
  exit 1
fi

_DEV_COMMON_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$_DEV_COMMON_DIR/.." && pwd)"

info() {
  echo "==> $*"
}

ok() {
  echo "✓ $*"
}

warn() {
  echo "warning: $*" >&2
}

die() {
  echo "error: $*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

fusion_ffi_lib() {
  local profile="${1:-debug}"
  local target_dir="${CARGO_TARGET_DIR:-$ROOT/target}"
  local base="$target_dir/$profile"
  local candidate
  case "$(uname -s)" in
    Darwin) candidate="$base/libfusion_ffi.dylib" ;;
    MINGW*|MSYS*|CYGWIN*) candidate="$base/fusion_ffi.dll" ;;
    *) candidate="$base/libfusion_ffi.so" ;;
  esac
  if [[ -f "$candidate" ]]; then
    echo "$candidate"
    return 0
  fi
  find "$base" -maxdepth 1 \( -name 'libfusion_ffi.so' -o -name 'libfusion_ffi.dylib' -o -name 'fusion_ffi.dll' \) -print -quit 2>/dev/null || true
}
