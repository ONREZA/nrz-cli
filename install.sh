#!/bin/bash
# Installation script for nrz CLI
# Usage: download this script, review it if desired, then run it with bash.

set -euo pipefail

REPO="onreza/nrz-cli"
BINARY_NAME="nrz"

# Detect OS and architecture
detect_platform() {
    local os arch
    
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)
    
    case "$os" in
        linux)
            case "$arch" in
                x86_64) echo "linux-x64" ;;
                *) echo "unsupported: $os $arch" ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64) echo "darwin-x64" ;;
                aarch64|arm64) echo "darwin-arm64" ;;
                *) echo "unsupported: $os $arch" ;;
            esac
            ;;
        mingw*|cygwin*|msys*|windows*)
            echo "win32-x64"
            ;;
        *)
            echo "unsupported: $os"
            ;;
    esac
}

# Get latest release version
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

sha256_file() {
    local path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{ print tolower($1) }'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{ print tolower($1) }'
    elif command -v openssl >/dev/null 2>&1; then
        openssl dgst -sha256 "$path" | awk '{ print tolower($NF) }'
    else
        echo "❌ SHA-256 verification requires sha256sum, shasum, or openssl" >&2
        return 1
    fi
}

# Download and install
echo "🔧 Installing $BINARY_NAME..."

PLATFORM=$(detect_platform)
if [[ "$PLATFORM" == unsupported* ]]; then
    echo "❌ Unsupported platform: $PLATFORM"
    echo "Please build from source: cargo install --git https://github.com/$REPO"
    exit 1
fi

VERSION="${NRZ_VERSION:-$(get_latest_version)}"
if [ -z "$VERSION" ]; then
    echo "❌ Failed to get latest version"
    exit 1
fi
if [[ "$VERSION" != v* ]]; then
    VERSION="v$VERSION"
fi
if [[ ! "$VERSION" =~ ^v[0-9A-Za-z][0-9A-Za-z._+-]*$ ]]; then
    echo "❌ Invalid release version: $VERSION"
    exit 1
fi

echo "📦 Version: $VERSION"
echo "💻 Platform: $PLATFORM"

# Create temp directory
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# Download archive and release checksums
ASSET_NAME="${BINARY_NAME}-${PLATFORM}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"
CHECKSUMS_NAME="checksums-sha256.txt"
CHECKSUMS_URL="https://github.com/${REPO}/releases/download/${VERSION}/${CHECKSUMS_NAME}"
ARCHIVE_PATH="${TMP_DIR}/${ASSET_NAME}"
CHECKSUMS_PATH="${TMP_DIR}/${CHECKSUMS_NAME}"

echo "⬇️  Downloading from $URL..."
if ! curl -fsSL --connect-timeout 15 --max-time 300 --max-filesize 268435456 \
    "$URL" -o "$ARCHIVE_PATH"; then
    echo "❌ Download failed"
    exit 1
fi
if ! curl -fsSL --connect-timeout 15 --max-time 60 --max-filesize 1048576 \
    "$CHECKSUMS_URL" -o "$CHECKSUMS_PATH"; then
    echo "❌ Failed to download release checksums"
    exit 1
fi
if [ "$(wc -c < "$ARCHIVE_PATH")" -gt 268435456 ] ||
    [ "$(wc -c < "$CHECKSUMS_PATH")" -gt 1048576 ]; then
    echo "❌ Downloaded release data exceeds the size limit"
    exit 1
fi

EXPECTED_SHA256=$(awk -v asset="$ASSET_NAME" '
    ($2 == asset || $2 == "*" asset) &&
    length($1) == 64 &&
    $1 !~ /[^0-9A-Fa-f]/ {
        count += 1
        digest = tolower($1)
    }
    END {
        if (count == 1) {
            print digest
        } else {
            exit 1
        }
    }
' "$CHECKSUMS_PATH") || {
    echo "❌ Expected one valid checksum for $ASSET_NAME"
    exit 1
}
ACTUAL_SHA256=$(sha256_file "$ARCHIVE_PATH")
if [ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]; then
    echo "❌ Checksum verification failed for $ASSET_NAME"
    echo "   Expected: $EXPECTED_SHA256"
    echo "   Actual:   $ACTUAL_SHA256"
    exit 1
fi
echo "✅ Checksum verified"

# Extract archive
echo "📂 Extracting archive..."
if ! tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"; then
    echo "❌ Failed to extract archive"
    exit 1
fi

# Find extracted binary
if [[ "$PLATFORM" == "win32-x64" ]]; then
    BINARY_FILE="${BINARY_NAME}.exe"
else
    BINARY_FILE="${BINARY_NAME}"
fi

if [ -f "${TMP_DIR}/${BINARY_FILE}" ]; then
    BINARY_PATH="${TMP_DIR}/${BINARY_FILE}"
else
    BINARY_PATH=$(find "$TMP_DIR" -type f -name "$BINARY_FILE" -print -quit)
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found in archive"
    exit 1
fi

# Make executable (not for Windows)
if [[ "$PLATFORM" != "win32-x64" ]]; then
    chmod +x "$BINARY_PATH"
fi

# Determine install location
if [ -n "${INSTALL_DIR:-}" ]; then
    INSTALL_LOCATION="$INSTALL_DIR"
elif [ -w "/usr/local/bin" ]; then
    INSTALL_LOCATION="/usr/local/bin"
else
    INSTALL_LOCATION="$HOME/.local/bin"
    mkdir -p "$INSTALL_LOCATION"
fi

# Install
echo "📁 Installing to $INSTALL_LOCATION..."
if [[ "$PLATFORM" == "win32-x64" ]]; then
    INSTALL_PATH="${INSTALL_LOCATION}/${BINARY_NAME}.exe"
else
    INSTALL_PATH="${INSTALL_LOCATION}/${BINARY_NAME}"
fi

if [ -w "$INSTALL_LOCATION" ]; then
    mv "$BINARY_PATH" "$INSTALL_PATH"
else
    echo "🔑 sudo required for $INSTALL_LOCATION"
    sudo mv "$BINARY_PATH" "$INSTALL_PATH"
fi

# Verify installation
echo "✅ Installed to $INSTALL_PATH"
"$INSTALL_PATH" --version
if ! command -v "$BINARY_NAME" >/dev/null 2>&1; then
    echo "⚠️  Installed but not in PATH"
    echo "Add $INSTALL_LOCATION to your PATH or run:"
    echo "  export PATH=\"$INSTALL_LOCATION:\$PATH\""
fi
