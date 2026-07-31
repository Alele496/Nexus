#
# Nexus CLI installer for PowerShell — https://github.com/Alele496/Nexus/releases
#
# Downloads the latest Nexus binary from GitHub Releases and installs to ~/.nexus/bin/
#
# Usage:
#   irm https://raw.githubusercontent.com/Alele496/Nexus/agent-dev/nexus/crates/codegen/nexus-pager/scripts/install.ps1 | iex
#   $env:NEXUS_VERSION="0.3.0"; irm <url> | iex  # specific version
#

param(
    [Parameter(Position = 0)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'

# TLS 1.2 for PS 5.1 compatibility
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$ProgressPreference = 'SilentlyContinue'

if (-not $Version -and $env:NEXUS_VERSION) {
    $Version = $env:NEXUS_VERSION
}

# Platform guard: Windows-only (PowerShell Core on Linux/macOS detected via Platform property)
if ($PSVersionTable.Platform -and $PSVersionTable.Platform -ne 'Win32NT') {
    Write-Error "This installer is for Windows. On macOS/Linux, use: curl -fsSL https://raw.githubusercontent.com/Alele496/Nexus/agent-dev/nexus/crates/codegen/nexus-pager/scripts/install.sh | bash"
    exit 1
}

$NexusDir = if ($env:NEXUS_HOME) { $env:NEXUS_HOME } else { Join-Path $env:USERPROFILE '.nexus' }

# ── Helpers ────────────────────────────────────────────────────────────────

function Download-File([string]$Url, [string]$OutFile) {
    $request = [System.Net.HttpWebRequest]::Create($Url)
    $request.Timeout = 300000
    $request.AutomaticDecompression = [System.Net.DecompressionMethods]::GZip -bor [System.Net.DecompressionMethods]::Deflate
    $response = $request.GetResponse()
    $totalBytes = $response.ContentLength
    $stream = $response.GetResponseStream()
    $fileStream = [System.IO.File]::Create($OutFile)
    $buffer = New-Object byte[] 65536
    $totalRead = 0
    $lastPercent = -1
    $lastMb = -1

    try {
        while (($read = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
            $fileStream.Write($buffer, 0, $read)
            $totalRead += $read
            $mb = [math]::Round($totalRead / 1MB, 1)
            if ($totalBytes -gt 0) {
                $percent = [math]::Min(100, [math]::Floor(($totalRead / $totalBytes) * 100))
                if ($percent -ne $lastPercent) {
                    $totalMb = [math]::Round($totalBytes / 1MB, 1)
                    Write-Host "`r  Downloading... ${mb} MB / ${totalMb} MB (${percent}%)" -NoNewline
                    $lastPercent = $percent
                }
            } elseif ($mb -ne $lastMb) {
                Write-Host "`r  Downloading... ${mb} MB" -NoNewline
                $lastMb = $mb
            }
        }
        Write-Host ''
    } finally {
        $fileStream.Close()
        $stream.Close()
        $response.Close()
    }
}

# ── Detect architecture ────────────────────────────────────────────────────

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x86_64' }
    'x86'   { 'x86_64' }
    'ARM64' { 'aarch64' }
    default { $null }
}

if (-not $arch) {
    Write-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
    exit 1
}

$platform = "windows-$arch"

# ── Resolve version ────────────────────────────────────────────────────────

$BinDir = if ($env:NEXUS_BIN_DIR) { $env:NEXUS_BIN_DIR } else { Join-Path $NexusDir 'bin' }
$DownloadDir = Join-Path $NexusDir 'downloads'
New-Item -ItemType Directory -Path $DownloadDir -Force | Out-Null
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null

$GitHubReleases = "https://github.com/Alele496/Nexus/releases"

if (-not $Version) {
    Write-Host "Fetching latest Nexus version..." -ForegroundColor DarkGray
    try {
        $latestUrl = "https://api.github.com/repos/Alele496/Nexus/releases/latest"
        $release = Invoke-RestMethod -Uri $latestUrl -UseBasicParsing
        $tag = $release.tag_name
        $Version = $tag -replace '^v', ''
        Write-Host "  Latest: v$Version" -ForegroundColor DarkGray
    } catch {
        Write-Error "Failed to fetch latest version from GitHub. Try specifying a version: irm <url> | iex -Version 0.3.0"
        exit 1
    }
}

Write-Host "Installing Nexus v$Version ($platform)..." -ForegroundColor Cyan

# ── Download binary ────────────────────────────────────────────────────────

$zipName = "nexus-$Version-windows-$arch.zip"
$downloadUrl = "https://github.com/Alele496/Nexus/releases/download/v$Version/$zipName"
$zipPath = Join-Path $DownloadDir $zipName

try {
    Download-File $downloadUrl $zipPath
} catch {
    if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
    Write-Error "Download failed from $downloadUrl"
    exit 1
}

Write-Host "  Extracting..." -ForegroundColor DarkGray
$extractDir = Join-Path $DownloadDir "nexus-$Version"
if (Test-Path $extractDir) { Remove-Item $extractDir -Recurse -Force }
Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
$exeName = "nexus-$Version-windows-$arch.exe"
$binaryPath = Join-Path $extractDir $exeName

if (-not (Test-Path $binaryPath)) {
    Write-Error "Extracted archive does not contain $exeName"
    exit 1
}

# ── Install binary ─────────────────────────────────────────────────────────

$dest = Join-Path $BinDir 'nexus.exe'
$old = "$dest.old"

if (Test-Path $old) { Remove-Item $old -Force -ErrorAction SilentlyContinue }

try {
    Copy-Item -Path $binaryPath -Destination $dest -Force
} catch {
    try {
        if (Test-Path $dest) { Rename-Item $dest $old -Force -ErrorAction SilentlyContinue }
        Copy-Item -Path $binaryPath -Destination $dest -Force
    } catch {
        if (Test-Path $old) { Rename-Item $old $dest -Force -ErrorAction SilentlyContinue }
        Write-Error "Failed to install nexus.exe"
        exit 1
    }
}

Write-Host "  Installed to $BinDir\nexus.exe" -ForegroundColor DarkGray

# ── Add to PATH ────────────────────────────────────────────────────────────

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$pathEntries = if ($userPath) { $userPath -split ';' | Where-Object { $_ -ne '' } } else { @() }
if ($pathEntries -notcontains $BinDir) {
    $newPath = (@($BinDir) + $pathEntries) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    Write-Host "  Added $BinDir to your User PATH." -ForegroundColor DarkGray
    if ($env:Path -notlike "*$BinDir*") {
        $env:Path = "$BinDir;$env:Path"
    }
}

Write-Host ''
Write-Host "Nexus v$Version installed successfully!" -ForegroundColor Green
Write-Host ''
Write-Host "Run 'nexus' to get started!" -ForegroundColor Cyan
Write-Host "  • Press F2 in Nexus to open settings"
Write-Host "  • First run will guide you through setup"
Write-Host "  • Docs: https://github.com/Alele496/Nexus"
