#!/usr/bin/env bash
# Bump the Fusion Framework version in every package manifest and header fallback.
#
# Usage:
#   ./scripts/set-version.sh 1.2.3
#   ./scripts/set-version.sh --dry-run 1.2.3
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/set-version.sh [--dry-run] <version>

Set the framework version in all package manifests and header fallbacks.

Examples:
  ./scripts/set-version.sh 1.2.3
  ./scripts/set-version.sh --dry-run 1.3.0
EOF
}

DRY_RUN=0
VERSION=""

for arg in "$@"; do
  case "$arg" in
    -h|--help)
      usage
      exit 0
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    -*)
      echo "error: unknown option: $arg" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [[ -n "$VERSION" ]]; then
        echo "error: extra argument: $arg" >&2
        usage >&2
        exit 2
      fi
      VERSION="$arg"
      ;;
  esac
done

if [[ -z "$VERSION" ]]; then
  usage >&2
  exit 2
fi

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "error: version must look like 1.2.3 (optional pre-release suffix allowed)" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CURRENT="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
if [[ -z "$CURRENT" ]]; then
  echo "error: could not read current version from Cargo.toml" >&2
  exit 1
fi

if [[ "$CURRENT" == "$VERSION" ]]; then
  echo "already at $VERSION"
  exit 0
fi

escape_sed() {
  printf '%s\n' "$1" | sed -e 's/[][(){}.*+?^$|/\\]/\\&/g'
}

OLD_ESC="$(escape_sed "$CURRENT")"

replace_file() {
  local file="$1"
  local expression="$2"
  if [[ ! -f "$file" ]]; then
    echo "error: missing $file" >&2
    exit 1
  fi
  if ! grep -q -- "$CURRENT" "$file"; then
    echo "warn: $CURRENT not found in $file (skipped)"
    return 0
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "dry-run: $file"
    return 0
  fi
  sed -i -E "$expression" "$file"
  echo "updated $file"
}

echo "$CURRENT -> $VERSION"

replace_file "Cargo.toml" \
  "s/^version = \"${OLD_ESC}\"/version = \"${VERSION}\"/"

replace_file "crates/fusion-py/pyproject.toml" \
  "s/^version = \"${OLD_ESC}\"/version = \"${VERSION}\"/"

replace_file "crates/fusion-node/package.json" \
  "s/\"version\": \"${OLD_ESC}\"/\"version\": \"${VERSION}\"/"

replace_file "bindings/csharp/FusionFramework/FusionFramework.csproj" \
  "s|<Version>${OLD_ESC}</Version>|<Version>${VERSION}</Version>|"

replace_file "crates/fusion-py/python/fusion_framework/middleware.py" \
  "s/\"${OLD_ESC}\"/\"${VERSION}\"/g"

replace_file "crates/fusion-node/index.js" \
  "s/'${OLD_ESC}'/'${VERSION}'/g"

replace_file "bindings/csharp/FusionFramework/Header.cs" \
  "s/\"${OLD_ESC}\"/\"${VERSION}\"/"

if [[ -f Cargo.lock ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "dry-run: Cargo.lock"
  else
    python3 - "$CURRENT" "$VERSION" <<'PY'
import pathlib
import re
import sys

old, new = sys.argv[1], sys.argv[2]
path = pathlib.Path("Cargo.lock")
text = path.read_text()
names = ("fusion-core", "fusion-py", "fusion-node", "fusion-ffi")
updated = 0
for name in names:
    pattern = rf'(name = "{re.escape(name)}"\nversion = "){re.escape(old)}(")'
    text, n = re.subn(pattern, rf"\g<1>{new}\2", text, count=1)
    updated += n
if updated == 0:
    print(f"warn: {old} not found in Cargo.lock workspace packages (skipped)")
else:
    path.write_text(text)
    print(f"updated Cargo.lock ({updated} workspace packages)")
PY
  fi
fi

echo
echo "done. next:"
echo "  git add -u"
echo "  git commit -m \"release v${VERSION}\""
echo "  git tag v${VERSION}"
echo "  git push origin HEAD && git push origin v${VERSION}"
