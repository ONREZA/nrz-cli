# nrz - ONREZA Platform CLI

[![CI](https://github.com/ONREZA/nrz-cli/actions/workflows/release.yml/badge.svg)](https://github.com/ONREZA/nrz-cli/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`nrz` is a Rust CLI for ONREZA project lifecycle: detect, dev, build, deploy, database, KV, env vars, domains, and releases.

## Installation

### Linux/macOS
```bash
curl -fsSL https://raw.githubusercontent.com/onreza/nrz-cli/main/install.sh | bash
```

### Windows (PowerShell 7+)
```powershell
iwr -useb https://raw.githubusercontent.com/onreza/nrz-cli/main/install.ps1 | iex
```

### npm
```bash
npm install -g @onreza/nrz
npm install -g @onreza/nrz@beta  # prerelease channel
```

### From source
```bash
cargo install --git https://github.com/onreza/nrz-cli
```

## Quick Start

```bash
# 1) Authenticate (or pass --token / NRZ_TOKEN in CI)
nrz login

# 2) Create local scaffold and optionally link/create platform project
nrz init
# alternatives:
# nrz init --local
# nrz init --create --name my-app
# nrz init --project-id proj_abc123

# 3) Detect framework and persist it to onreza.toml
nrz detect --save

# Explain what build/deploy config the CLI will use
nrz config explain --json
# Local-only explanation without fetching server project settings:
# nrz config explain --local --json

# 4) Local development with ONREZA emulation (KV + DB)
nrz dev

# 5) Validate build output
nrz build

# 6) Deploy
nrz deploy --prod
```

## Core Commands

| Area | Commands |
|------|----------|
| Project | `nrz init`, `nrz link`, `nrz projects list`, `nrz projects create --name <slug>` |
| Build/Deploy | `nrz detect`, `nrz build`, `nrz deploy`, `nrz rollback` |
| Runtime | `nrz deployments`, `nrz logs` |
| Database | `nrz db list`, `nrz db query "SELECT 1"`, `nrz db schema`, `nrz db branches` |
| KV | `nrz kv get <key> --env preview`, `nrz kv set <key> <value> --ttl 60 --env preview`, `nrz kv list --prefix app_` |
| Environment | `nrz context use <environment>`, `nrz env list`, `nrz env set`, `nrz env validate`, `nrz env exec -- <command>` |
| Domains | `nrz domains list`, `nrz domains add example.com`, `nrz domains verify <domain_id>` |
| Account | `nrz whoami`, `nrz workspace list`, `nrz workspace switch <slug>`, `nrz upgrade` |

Prerelease binaries can be tested with `nrz upgrade --channel beta`,
or a pinned version such as `nrz upgrade --version v0.33.0-beta.0`.

Breaking beta migrations and exact required actions are documented in
[`docs/breaking-changes.md`](docs/breaking-changes.md). Sections marked
`UNRELEASED` are plans, not current CLI behavior.

## Agent Skills

This repository includes reusable skills for AI coding assistants in `skills/`:
- `nrz-cli-deploy`
- `nrz-cli-ci-automation`
- `nrz-cli-env-db-kv`
- `nrz-cli-project-bootstrap`

Install from Context7:
```bash
npx ctx7 skills install /onreza/nrz-cli nrz-cli-deploy
npx ctx7 skills install /onreza/nrz-cli nrz-cli-ci-automation
npx ctx7 skills install /onreza/nrz-cli nrz-cli-env-db-kv
npx ctx7 skills install /onreza/nrz-cli nrz-cli-project-bootstrap
```

## Automation and JSON Mode

The CLI is designed for both human and machine usage:
- Global flags: `--json`, `--token`, `--workspace`.
- Env vars: `NRZ_JSON`, `NRZ_TOKEN`, `NRZ_WORKSPACE`; `NRZ_ENVIRONMENT` selects the platform execution context for every Environment-aware command. Legacy `NRZ_ENV` is scoped only to the local `kv` namespace.
- In JSON mode, commands return structured output in `stdout`; errors are JSON with exit code `1`.

Example:
```bash
nrz deploy --json --token "$NRZ_TOKEN" --workspace my-team --environment production
```

## Configuration

Project configuration is stored in `onreza.toml` (committed to git). Local runtime state is stored in `.onreza/` (must stay gitignored).

Reference:
- [`docs/onreza-toml.md`](docs/onreza-toml.md)
- [JSON Schema](https://docs.onreza.ru/schemas/onreza-project-v1.schema.json) (generated; `nrz init` wires it via `#:schema`)

## Development

```bash
mise install                   # install pinned local tools
mise run hooks                 # install git hooks (lefthook)
mise run fmt                   # format Rust code
mise run clippy                # strict lint
mise run test                  # all tests
mise run check                 # standard local quality gate
cargo build --release          # release build
dagger call release-metadata --source=. --channel=beta --bump=minor
```

Commit messages are validated with Conventional Commits via Cocogitto + Lefthook, format: `type(scope): subject`.
Release version/package metadata and GitHub publishing remain Dagger-owned; do not use `cog bump` as the nrz release path.

## Supported Binary Targets

| Target |
|--------|
| `linux-x64` |
| `darwin-x64` |
| `darwin-arm64` |
| `win32-x64` |

## License

MIT
