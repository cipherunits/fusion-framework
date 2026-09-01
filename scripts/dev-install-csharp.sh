#!/usr/bin/env bash
# Build fusion-ffi and the C# FusionFramework project from source.
#
# Usage (from repo root):
#   ./scripts/dev-install-csharp.sh
#
# Verify:
#   dotnet build examples/csharp_hello/csharp_hello.csproj
set -euo pipefail

# shellcheck source=_dev-common.sh
source "$(cd "$(dirname "$0")" && pwd)/_dev-common.sh"
cd "$ROOT"

PROFILE=debug
PACK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      PROFILE=release
      shift
      ;;
    --pack)
      PACK=1
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Build fusion-ffi and the C# binding for local development.

Options:
  --release   Release build (default: debug)
  --pack      Also run dotnet pack (local NuGet in dist/)
  -h, --help  Show this help

Examples:
  ./scripts/dev-install-csharp.sh
  export FUSION_FFI_PATH=$PWD/target/debug/libfusion_ffi.so
  dotnet run --project examples/csharp_hello
EOF
      exit 0
      ;;
    *)
      shift
      ;;
  esac
done

require_cmd cargo
require_cmd dotnet

CARGO_PROFILE="$PROFILE"
[[ "$PROFILE" == "release" ]] && CARGO_PROFILE=release

info "building fusion-ffi ($CARGO_PROFILE)"
if [[ "$CARGO_PROFILE" == "release" ]]; then
  cargo build -p fusion-ffi --release
else
  cargo build -p fusion-ffi
fi

FFI_PATH="$(fusion_ffi_lib "$CARGO_PROFILE")"
[[ -n "$FFI_PATH" && -f "$FFI_PATH" ]] || die "native library not found under ${CARGO_TARGET_DIR:-$ROOT/target}/$CARGO_PROFILE (run: cargo build -p fusion-ffi)"

export FUSION_FFI_PATH="$FFI_PATH"
ok "fusion-ffi: $FFI_PATH"

CONFIG=Debug
[[ "$PROFILE" == "release" ]] && CONFIG=Release

PROJ="$ROOT/bindings/csharp/FusionFramework/FusionFramework.csproj"
info "building FusionFramework ($CONFIG)"
dotnet build -c "$CONFIG" "$PROJ"

if [[ "$PACK" -eq 1 ]]; then
  mkdir -p "$ROOT/dist"
  info "packing NuGet package"
  dotnet pack -c "$CONFIG" -o "$ROOT/dist" "$PROJ"
  ok "NuGet package(s) in dist/"
fi

cat <<EOF

Next steps:
  export FUSION_FFI_PATH="$FFI_PATH"
  dotnet build examples/csharp_hello/csharp_hello.csproj
  dotnet run --project examples/csharp_hello

In your app .csproj:
  <ProjectReference Include="../../bindings/csharp/FusionFramework/FusionFramework.csproj" />
EOF
