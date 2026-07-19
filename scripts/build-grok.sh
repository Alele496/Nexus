#!/bin/bash
# Agent-SYS grok-build 构建脚本 (Bash)
# 解决 Windows 下三个编译问题:
# 1. protoc 路径 (PROTOC env)
# 2. Unicode 临时目录 (TMP/TEMP 重定向到纯 ASCII 路径)
# 3. /dev/stdout 不可用 (已通过源码补丁修复)
#
# 用法:
#   bash scripts/build-grok.sh           # cargo check (快速验证)
#   bash scripts/build-grok.sh --release # release 构建
#   bash scripts/build-grok.sh --check   # cargo check (默认)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
GROK_SOURCE="$PROJECT_ROOT/grok-build-main"
CARGO_TEMP="/e/cargo-tmp"
PROTOC_BIN="$GROK_SOURCE/bin/protoc.exe"

# 纯 ASCII 临时目录 (protoc 不支持 Unicode 路径)
mkdir -p "$CARGO_TEMP"

if [ ! -f "$PROTOC_BIN" ]; then
    echo "ERROR: protoc.exe not found at $PROTOC_BIN"
    echo "Download from: https://github.com/protocolbuffers/protobuf/releases"
    exit 1
fi

echo "========================================"
echo " Agent-SYS grok-build Compiler"
echo "========================================"
echo "PROTOC: $PROTOC_BIN"
echo "TEMP:   $CARGO_TEMP"
echo ""

export PROTOC="$PROTOC_BIN"
export TMP="$CARGO_TEMP"
export TEMP="$CARGO_TEMP"
export TMPDIR="$CARGO_TEMP"

MODE="${1:---check}"
cd "$GROK_SOURCE"

case "$MODE" in
    --release|-r)
        echo "Building release..."
        cargo build -p xai-grok-pager-bin --release
        echo ""
        echo "Build successful!"
        echo "Binary: $GROK_SOURCE/target/release/xai-grok-pager"
        echo ""
        echo "To use as 'grok' CLI, copy to PATH:"
        echo "  cp $GROK_SOURCE/target/release/xai-grok-pager /usr/local/bin/grok"
        ;;
    *)
        echo "Running cargo check..."
        cargo check -p xai-grok-pager-bin
        echo ""
        echo "Check passed! Use --release to build."
        ;;
esac
