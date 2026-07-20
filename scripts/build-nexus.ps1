# Nexus 构建脚本 (PowerShell)
#
# 用法:
#   .\scripts\build-nexus.ps1              # cargo check (快速验证)
#   .\scripts\build-nexus.ps1 -Release     # release 构建
#   .\scripts\build-nexus.ps1 -Check       # cargo check (默认)

param(
    [switch]$Release,
    [switch]$Check
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$NexusSource = Join-Path $ProjectRoot "nexus"

# 纯 ASCII 临时目录 (protoc 不支持 Unicode 路径)
$CargoTemp = Join-Path $ProjectRoot ".cargo-tmp"
if (-not (Test-Path $CargoTemp)) {
    New-Item -ItemType Directory -Path $CargoTemp -Force | Out-Null
}

# protoc 路径
$ProtocBin = Join-Path $NexusSource "bin/protoc.exe"
if (-not (Test-Path $ProtocBin)) {
    Write-Error "protoc.exe not found at $ProtocBin"
    Write-Error "Download from: https://github.com/protocolbuffers/protobuf/releases"
    exit 1
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Nexus Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "PROTOC: $ProtocBin"
Write-Host "TEMP:   $CargoTemp"
Write-Host ""

# 设置环境变量
$env:PROTOC = $ProtocBin
$env:TMP = $CargoTemp
$env:TEMP = $CargoTemp
$env:TMPDIR = $CargoTemp

Push-Location $NexusSource
try {
    if ($Release) {
        Write-Host "Building release..." -ForegroundColor Green
        cargo build -p nexus-bin --release
        Write-Host ""

        $TargetDir = Join-Path $NexusSource "target/release"
        $ExeName = if ($IsWindows) { "nexus.exe" } else { "nexus" }
        $ExePath = Join-Path $TargetDir $ExeName

        if (Test-Path $ExePath) {
            Write-Host "Build successful!" -ForegroundColor Green
            Write-Host "Binary: $ExePath"
            Write-Host ""
            Write-Host "Run:" -ForegroundColor Yellow
            Write-Host "  $ExePath"
        }
    } else {
        Write-Host "Running cargo check..." -ForegroundColor Green
        cargo check -p nexus-bin
        Write-Host ""
        Write-Host "Check passed! Use -Release to build." -ForegroundColor Green
    }
} finally {
    Pop-Location
}
