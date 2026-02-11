#!/bin/bash
# Installation script for nrz CLI
# Usage: curl -fsSL https://raw.githubusercontent.com/onreza/nrz-cli/main/install.sh | bash

set -e

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
                aarch64|arm64) echo "linux-arm64" ;;
                *) echo "unsupported: $os $arch" ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64) echo "macos-x64" ;;
                aarch64|arm64) echo "macos-arm64" ;;
                *) echo "unsupported: $os $arch" ;;
            esac
            ;;
        mingw*|cygwin*|msys*|windows*)
            echo "windows-x64.exe"
            ;;
        *)
            echo "unsupported: $os"
            ;;
    esac
}

# Get latest release version
get_latest_version() {
    curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

# Download and install
echo "🔧 Installing $BINARY_NAME..."

PLATFORM=$(detect_platform)
if [[ "$PLATFORM" == unsupported* ]]; then
    echo "❌ Unsupported platform: $PLATFORM"
    echo "Please build from source: cargo install --git https://github.com/$REPO"
    exit 1
fi

VERSION=$(get_latest_version)
if [ -z "$VERSION" ]; then
    echo "❌ Failed to get latest version"
    exit 1
fi

echo "📦 Version: $VERSION"
echo "💻 Platform: $PLATFORM"

# Create temp directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

# Download binary
ASSET_NAME="${BINARY_NAME}-${PLATFORM}"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"

echo "⬇️  Downloading from $URL..."
if ! curl -fsSL "$URL" -o "${TMP_DIR}/${BINARY_NAME}"; then
    echo "❌ Download failed"
    exit 1
fi

# Make executable (not for Windows)
if [[ "$PLATFORM" != *".exe" ]]; then
    chmod +x "${TMP_DIR}/${BINARY_NAME}"
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
if [ -w "$INSTALL_LOCATION" ]; then
    mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_LOCATION}/"
else
    echo "🔑 sudo required for $INSTALL_LOCATION"
    sudo mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_LOCATION}/"
fi

# Verify installation
if command -v "$BINARY_NAME" &> /dev/null; then
    echo "✅ $BINARY_NAME installed successfully!"
    echo ""
    "$BINARY_NAME" --version
else
    echo "⚠️  Installed but not in PATH"
    echo "Add $INSTALL_LOCATION to your PATH or run:"
    echo "  export PATH=\"$INSTALL_LOCATION:\$PATH\""
fi
