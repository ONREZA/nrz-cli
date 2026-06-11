# Repository Guidelines

## Project Structure & Module Organization
`nrz` is a Rust CLI. Entrypoints are `src/main.rs` (binary) and `src/lib.rs` (shared exports). Core domains are split under `src/`: `cli/`, `deploy/`, `build/`, `detect/`, `dev/`, `emulator/`, `auth/`, `config/`, `init/`, and `upgrade/`.

Integration tests are in `tests/` (for example `tests/cli_integration_test.rs`). Unit tests are colocated as dedicated `*_tests.rs` files (for example `src/build/manifest_tests.rs`). Operational docs live in `docs/`; root config includes `lefthook.yml` and `commitlint.config.js`. The `onreza.toml` JSON Schema is generated in the platform repo and served at `https://docs.onreza.ru/schemas/onreza-project-v1.schema.json` (`nrz init` wires it via `#:schema`).

## LLM-First CLI Contract
Every command must support machine and human modes:
- Global flags/env: `--json`/`NRZ_JSON`, `--token`/`NRZ_TOKEN`, `--workspace`/`NRZ_WORKSPACE`.
- Command-scoped env: `--env`/`NRZ_ENV` only on commands that use environments (`deploy`, `env`, `kv`).
- JSON mode: one JSON object to `stdout`; errors as `{"error":"..."}` with exit code `1`.
- Human mode: readable output/errors in `stderr`.
- Avoid interactive-only behavior; provide non-interactive alternatives (`--project-id`, `--force`, etc.).

## Build, Test, and Development Commands
- `cargo run -- <command>`: run CLI locally (example: `cargo run -- deploy --json`).
- `cargo build` / `cargo build --release`: debug and production builds.
- `cargo test`: run all tests.
- `cargo test --test cli_integration_test`: run one integration suite.
- `cargo fmt`: format code.
- `cargo clippy -- -D warnings`: fail on lints/warnings.
- `npm run prepare`: install Lefthook hooks.
- `dagger call release-metadata --source=. --channel=stable --bump=auto`: local release plan dry run.

## Coding Style & Testing Guidelines
Use Rust 2024 + `rustfmt` defaults (4-space indentation, stable formatting). Naming: files/modules/functions `snake_case`, types `PascalCase`.

Testing rules:
- Keep tests in separate `*_tests.rs` files; avoid inline `#[cfg(test)] mod tests {}` in production files.
- Use `assert_cmd` for CLI behavior, `reqwest` for emulator HTTP checks, and `tempfile::tempdir()` for isolated filesystem scenarios.
- Add regression tests for bug fixes.

## Configuration, Commits, and PRs
`onreza.toml` is committed and is the single project config source. `.onreza/` is local state only (KV/environment refs) and must remain gitignored.

Conventional Commits are enforced by Lefthook + commitlint. Format: `type(scope): subject` (example: `fix(deploy): resolve process entry fallback`). Typical scopes: `cli`, `deploy`, `build`, `config`, `emulator`, `db`, `kv`, `tests`, `release`.

Before opening a PR, run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`. Include a concise problem/solution summary, linked issue (if any), and CLI output snippets for user-visible changes.
