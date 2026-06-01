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
| Database | `nrz db shell`, `nrz db execute "<sql>"`, `nrz db migrate create <name>`, `nrz db migrate apply` |
| KV | `nrz kv get <key>`, `nrz kv set <key> <value> --ttl 60`, `nrz kv list --prefix app_` |
| Environment | `nrz env list`, `nrz env pull`, `nrz env push .env.local --declared-only`, `nrz env validate` |
| Domains | `nrz domains list`, `nrz domains add example.com`, `nrz domains verify <domain_id>` |
| Account | `nrz whoami`, `nrz workspace list`, `nrz workspace switch <slug>`, `nrz upgrade` |

Prerelease binaries can be tested with `nrz upgrade --channel beta`, `nrz upgrade --channel rc`,
or a pinned version such as `nrz upgrade --version v0.33.0-beta.0`.

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
- Global flags: `--json`, `--token`, `--workspace`, `--env`.
- Env vars: `NRZ_JSON`, `NRZ_TOKEN`, `NRZ_WORKSPACE`, `NRZ_ENV`.
- In JSON mode, commands return structured output in `stdout`; errors are JSON with exit code `1`.

Example:
```bash
nrz deploy --json --token "$NRZ_TOKEN" --workspace my-team --env production
```

## Configuration

Project configuration is stored in `onreza.toml` (committed to git). Local runtime state is stored in `.onreza/` (must stay gitignored).

Reference:
- [`docs/onreza-toml.md`](docs/onreza-toml.md)
- [`onreza.schema.json`](onreza.schema.json)

## Development

```bash
npm run prepare                # install git hooks (lefthook)
cargo fmt                      # format Rust code
cargo clippy -- -D warnings    # strict lint
cargo test                     # all tests
cargo build --release          # release build
dagger call release-metadata --source=. --channel=beta --bump=minor
```

Commit messages are validated with Conventional Commits via `commitlint` + `lefthook`, format: `type(scope): subject`.

## Supported Binary Targets

| Target |
|--------|
| `linux-x64` |
| `darwin-x64` |
| `darwin-arm64` |
| `win32-x64` |

## License

MIT
