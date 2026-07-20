#!/bin/bash
# Nexus 构建脚本 (Bash)
#
# 用法:
#   bash scripts/build-nexus.sh           # cargo check (快速验证)
#   bash scripts/build-nexus.sh --release # release 构建
#   bash scripts/build-nexus.sh --check   # cargo check (默认)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
NEXUS_SOURCE="$PROJECT_ROOT/nexus"
CARGO_TEMP="/e/cargo-tmp"
PROTOC_BIN="$NEXUS_SOURCE/bin/protoc.exe"

# 纯 ASCII 临时目录 (protoc 不支持 Unicode 路径)
mkdir -p "$CARGO_TEMP"

if [ ! -f "$PROTOC_BIN" ]; then
    echo "ERROR: protoc.exe not found at $PROTOC_BIN"
    echo "Download from: https://github.com/protocolbuffers/protobuf/releases"
    exit 1
fi

echo "========================================"
echo " Nexus Build"
echo "========================================"
echo "PROTOC: $PROTOC_BIN"
echo "TEMP:   $CARGO_TEMP"
echo ""

export PROTOC="$PROTOC_BIN"
export TMP="$CARGO_TEMP"
export TEMP="$CARGO_TEMP"
export TMPDIR="$CARGO_TEMP"

MODE="${1:---check}"
cd "$NEXUS_SOURCE"

case "$MODE" in
    --release|-r)
        echo "Building release..."
        cargo build -p nexus-bin --release
        echo ""
        echo "Build successful!"
        echo "Binary: $NEXUS_SOURCE/target/release/nexus"
        echo ""
        echo "Run:"
        echo "  $NEXUS_SOURCE/target/release/nexus"
        ;;
    *)
        echo "Running cargo check..."
        cargo check -p nexus-bin
        echo ""
        echo "Check passed! Use --release to build."
        ;;
esac
