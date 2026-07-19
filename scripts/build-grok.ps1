# Agent-SYS grok-build 构建脚本 (PowerShell)
# 解决 Windows 下三个编译问题:
# 1. protoc 路径 (PROTOC env)
# 2. Unicode 临时目录 (TMP/TEMP 重定向到纯 ASCII 路径)
# 3. /dev/stdout 不可用 (已通过源码补丁修复)
#
# 用法:
#   .\scripts\build-grok.ps1              # cargo check (快速验证)
#   .\scripts\build-grok.ps1 -Release     # release 构建
#   .\scripts\build-grok.ps1 -Check       # cargo check (默认)

param(
    [switch]$Release,
    [switch]$Check
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$GrokSource = Join-Path $ProjectRoot "grok-build-main"

# 纯 ASCII 临时目录 (protoc 不支持 Unicode 路径)
$CargoTemp = Join-Path $ProjectRoot ".cargo-tmp"
if (-not (Test-Path $CargoTemp)) {
    New-Item -ItemType Directory -Path $CargoTemp -Force | Out-Null
}

# protoc 路径
$ProtocBin = Join-Path $GrokSource "bin/protoc.exe"
if (-not (Test-Path $ProtocBin)) {
    Write-Error "protoc.exe not found at $ProtocBin"
    Write-Error "Download from: https://github.com/protocolbuffers/protobuf/releases"
    exit 1
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Agent-SYS grok-build Compiler" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "PROTOC: $ProtocBin"
Write-Host "TEMP:   $CargoTemp"
Write-Host ""

# 设置环境变量
$env:PROTOC = $ProtocBin
$env:TMP = $CargoTemp
$env:TEMP = $CargoTemp
$env:TMPDIR = $CargoTemp

Push-Location $GrokSource
try {
    if ($Release) {
        Write-Host "Building release..." -ForegroundColor Green
        cargo build -p xai-grok-pager-bin --release
        Write-Host ""

        $TargetDir = Join-Path $GrokSource "target/release"
        $ExeName = if ($IsWindows) { "xai-grok-pager.exe" } else { "xai-grok-pager" }
        $ExePath = Join-Path $TargetDir $ExeName

        if (Test-Path $ExePath) {
            Write-Host "Build successful!" -ForegroundColor Green
            Write-Host "Binary: $ExePath"
            Write-Host ""
            Write-Host "To use as 'grok' CLI, copy to PATH:" -ForegroundColor Yellow
            Write-Host "  cp $ExePath /usr/local/bin/grok"
        }
    } else {
        Write-Host "Running cargo check..." -ForegroundColor Green
        cargo check -p xai-grok-pager-bin
        Write-Host ""
        Write-Host "Check passed! Use -Release to build." -ForegroundColor Green
    }
} finally {
    Pop-Location
}
