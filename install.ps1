# Installation script for nrz CLI (Windows)
# Usage: download this script, review it if desired, then run it with pwsh -File.

$ErrorActionPreference = "Stop"

$Repo = "onreza/nrz-cli"
$BinaryName = "nrz.exe"

function Detect-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE
    
    switch ($arch) {
        "AMD64" { return "win32-x64" }
        "x86" { 
            Write-Host "❌ x86 architecture not supported. Use x64." -ForegroundColor Red
            exit 1
        }
        default {
            Write-Host "❌ Unsupported architecture: $arch" -ForegroundColor Red
            exit 1
        }
    }
}

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        return $response.tag_name
    }
    catch {
        Write-Host "❌ Failed to get latest version" -ForegroundColor Red
        exit 1
    }
}

function Get-ExpectedSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$ChecksumPath,
        [Parameter(Mandatory = $true)][string]$AssetName
    )

    $digests = @()
    foreach ($line in Get-Content $ChecksumPath) {
        if ($line -match '^([A-Fa-f0-9]{64})\s+\*?(.+)$') {
            $name = Split-Path -Leaf $Matches[2]
            if ($name -eq $AssetName) {
                $digests += $Matches[1].ToLowerInvariant()
            }
        }
    }
    if ($digests.Count -ne 1) {
        throw "Expected one valid checksum for $AssetName"
    }
    return $digests[0]
}

function Assert-FileSha256 {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$ExpectedSha256
    )

    $actualSha256 = (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
    if ($actualSha256 -ne $ExpectedSha256) {
        throw "Checksum verification failed for $(Split-Path -Leaf $Path): expected $ExpectedSha256, got $actualSha256"
    }
}

Write-Host "🔧 Installing $BinaryName..." -ForegroundColor Cyan

$Platform = Detect-Platform
$Version = if ($env:NRZ_VERSION) { $env:NRZ_VERSION } else { Get-LatestVersion }
if (!$Version.StartsWith("v")) {
    $Version = "v$Version"
}
if ($Version -notmatch '^v[0-9A-Za-z][0-9A-Za-z._+-]*$') {
    throw "Invalid release version: $Version"
}

Write-Host "📦 Version: $Version" -ForegroundColor Gray
Write-Host "💻 Platform: $Platform" -ForegroundColor Gray

# Create temp directory
$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "nrz-$([guid]::NewGuid())"
New-Item -ItemType Directory -Path $TmpDir | Out-Null

# Download archive and release checksums
$AssetName = "nrz-$Platform.tar.gz"
$Url = "https://github.com/$Repo/releases/download/$Version/$AssetName"
$ChecksumsName = "checksums-sha256.txt"
$ChecksumsUrl = "https://github.com/$Repo/releases/download/$Version/$ChecksumsName"
$ArchivePath = Join-Path $TmpDir $AssetName
$ChecksumsPath = Join-Path $TmpDir $ChecksumsName

try {
    Write-Host "⬇️  Downloading from $Url..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $Url -OutFile $ArchivePath -UseBasicParsing -TimeoutSec 300
    Invoke-WebRequest -Uri $ChecksumsUrl -OutFile $ChecksumsPath -UseBasicParsing -TimeoutSec 60
    if ((Get-Item $ArchivePath).Length -gt 268435456 -or
        (Get-Item $ChecksumsPath).Length -gt 1048576) {
        throw "Downloaded release data exceeds the size limit"
    }
    $ExpectedSha256 = Get-ExpectedSha256 -ChecksumPath $ChecksumsPath -AssetName $AssetName
    Assert-FileSha256 -Path $ArchivePath -ExpectedSha256 $ExpectedSha256
    Write-Host "✅ Checksum verified" -ForegroundColor Green
}
catch {
    Write-Host "❌ Download or verification failed: $_" -ForegroundColor Red
    exit 1
}

# Extract archive
try {
    Write-Host "📂 Extracting archive..." -ForegroundColor Cyan
    tar -xzf $ArchivePath -C $TmpDir
}
catch {
    Write-Host "❌ Failed to extract archive: $_" -ForegroundColor Red
    exit 1
}

# Find extracted binary
$BinaryPath = Join-Path $TmpDir $BinaryName
if (!(Test-Path $BinaryPath -PathType Leaf)) {
    $FoundBinary = Get-ChildItem -Path $TmpDir -Recurse -File -Filter $BinaryName | Select-Object -First 1
    if ($FoundBinary) {
        $BinaryPath = $FoundBinary.FullName
    }
}
if (!(Test-Path $BinaryPath -PathType Leaf)) {
    Write-Host "❌ Binary not found in archive" -ForegroundColor Red
    exit 1
}

# Determine install location
if ($env:INSTALL_DIR) {
    $InstallDir = $env:INSTALL_DIR
}
elseif (Test-Path "$env:ProgramFiles\nrz") {
    $InstallDir = "$env:ProgramFiles\nrz"
}
else {
    $InstallDir = "$env:LOCALAPPDATA\Programs\nrz"
}

# Create install directory if not exists
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$InstallPath = Join-Path $InstallDir $BinaryName

# Install
Write-Host "📁 Installing to $InstallPath..." -ForegroundColor Cyan
try {
    Copy-Item $BinaryPath $InstallPath -Force
}
catch {
    Write-Host "❌ Installation failed (try running as Administrator): $_" -ForegroundColor Red
    exit 1
}

# Cleanup
Remove-Item $TmpDir -Recurse -Force

# Check if in PATH
$PathDirs = $env:PATH -split ";"
if ($InstallDir -notin $PathDirs) {
    Write-Host "⚠️  $InstallDir is not in your PATH" -ForegroundColor Yellow
    Write-Host "   Add to PATH manually or run:" -ForegroundColor Yellow
    Write-Host "   [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$InstallDir', 'User')" -ForegroundColor Yellow
    Write-Host ""
}

# Verify installation using the exact installed path
Write-Host "✅ Installed to $InstallPath" -ForegroundColor Green
Write-Host ""
& $InstallPath --version
if (!(Get-Command $BinaryName -ErrorAction SilentlyContinue)) {
    Write-Host "⚠️  Restart your terminal or add $InstallDir to PATH" -ForegroundColor Yellow
}
