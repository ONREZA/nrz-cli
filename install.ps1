# Installation script for nrz CLI (Windows)
# Usage: iwr -useb https://raw.githubusercontent.com/onreza/nrz-cli/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "onreza/nrz-cli"
$BinaryName = "nrz.exe"

function Detect-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE
    
    switch ($arch) {
        "AMD64" { return "windows-x64.exe" }
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

Write-Host "🔧 Installing $BinaryName..." -ForegroundColor Cyan

$Platform = Detect-Platform
$Version = Get-LatestVersion

Write-Host "📦 Version: $Version" -ForegroundColor Gray
Write-Host "💻 Platform: $Platform" -ForegroundColor Gray

# Create temp directory
$TmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }

# Download binary
$AssetName = "nrz-$Platform"
$Url = "https://github.com/$Repo/releases/download/$Version/$AssetName"
$TmpFile = Join-Path $TmpDir $BinaryName

try {
    Write-Host "⬇️  Downloading from $Url..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $Url -OutFile $TmpFile -UseBasicParsing
}
catch {
    Write-Host "❌ Download failed: $_" -ForegroundColor Red
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
    Copy-Item $TmpFile $InstallPath -Force
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

# Verify installation
if (Get-Command $BinaryName -ErrorAction SilentlyContinue) {
    Write-Host "✅ $BinaryName installed successfully!" -ForegroundColor Green
    Write-Host ""
    & $BinaryName --version
}
else {
    Write-Host "✅ Installed to $InstallPath" -ForegroundColor Green
    Write-Host "⚠️  Restart your terminal or add $InstallDir to PATH" -ForegroundColor Yellow
}
