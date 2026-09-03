#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "Building fusion-ffi (required for C# tests)..."
cargo build -p fusion-ffi --release

dotnet test tests/csharp/FusionFramework.Tests/FusionFramework.Tests.csproj -c Release "$@"
