# nrz — ONREZA Platform CLI

[![CI](https://github.com/ONREZA/nrz-cli/actions/workflows/release.yml/badge.svg)](https://github.com/ONREZA/nrz-cli/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> Analog of `vercel` / `wrangler` for ONREZA platform. Rust-based, single binary.

## Installation

### Quick Install

**Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/ONREZA/nrz-cli/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/ONREZA/nrz-cli/main/install.ps1 | iex
```

### From Source

```bash
cargo install --git https://github.com/ONREZA/nrz-cli
```

## Usage

```bash
# Development mode with platform emulation
nrz dev

# Validate build output
nrz build

# Deploy to platform
nrz deploy

# Manage KV store
nrz kv set mykey "my value"
nrz kv get mykey
nrz kv list

# Manage D1-compatible SQLite database
nrz db execute "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)"
nrz db info

# Self-update
nrz upgrade
```

## Supported Platforms

| Platform | Binary |
|----------|--------|
| Linux x64 | `nrz-linux-x64` |
| macOS x64 | `nrz-macos-x64` |
| macOS ARM64 | `nrz-macos-arm64` |
| Windows x64 | `nrz-windows-x64.exe` |

## Features

- 🚀 **Dev Server** — Local development with KV, DB, and Context emulation
- 📦 **Build Validation** — Verify output against BUILD_OUTPUT_SPEC v1
- ☁️ **Deploy** — Push to ONREZA platform
- 🔄 **Self-update** — Built-in upgrade mechanism
- 🔧 **Framework Detection** — Auto-detect Astro, Nuxt, SvelteKit, Nitro

## Development

```bash
# Run tests
cargo test

# Build release binary
cargo build --release

# Generate changelog
git-cliff -o CHANGELOG.md
```

## Commit Convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(scope): add new feature
fix(scope): fix bug
docs(scope): update documentation
perf(scope): optimize performance
```

## Related

- [onreza/adapters](https://github.com/onreza/adapters) — TypeScript adapters
- [onreza/deployment](https://github.com/onreza/deployment) — Platform infrastructure

## License

MIT © ONREZA
