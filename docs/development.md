# Contributing to nrz-cli

## CLI and framework adapters

Clone this public repository and install the pinned tools with `proto install`,
then run `proto run moon -- setup` for Rust components and pinned Cargo binaries.
The CLI, detection, framework adapters, generated SDK and shared Rust libraries
build from this checkout. Ordinary development and fixture tests run locally without an ONREZA account.

```sh
cargo run -- detect --dir /path/to/project --json
cargo test --locked --workspace
proto run moon -- run workspace:check
```

Detection lives under `src/detect`, build orchestration under `src/build`, and
the Next.js adapter under `src/nextjs_adapter.rs` and `src/nextjs_adapter/`.
The binary consumes `nrz::detect` from the library. Runtime binding and Python
launch resolution are part of that library API; keep detection and its tests
under one owner rather than compiling a second copy in the binary.
Behavioral fixtures live under `tests`. Add a representative project fixture and
assert its detection/build output when introducing a framework. Keep framework
behavior in these public modules; server-side publication consumes their typed
artifacts.

## Toolchain and quality gates

`.prototools` pins Proto-managed runtimes and tools. `.moon/toolchains.yml` pins
Cargo binaries, including cargo-deny, Cocogitto and cargo-mutants. Moon owns the
task graph in `moon.yml`; local hooks and CI use the same tasks. Dagger reads
Rust/Bun image versions from `.prototools` and cargo-deny from the Moon toolchain,
so container checks use the same pins as local development.

Moon caches static checks by their declared inputs. Tests, duplicate detection
and mutation testing always run: their results must not be reused from the task
cache. Cargo build dependencies and Proto downloads remain cached. Local cache,
`node_modules` and mutation reports are ignored by Git and Dagger source uploads.

```sh
proto run moon -- run workspace:hooks
proto run moon -- run workspace:check
proto run moon -- run workspace:cpd
proto run moon -- run workspace:mutants
```

The duplicate gate uses jscpd with a minimum of eight lines and 80 tokens.
Existing duplication has a reviewed baseline; every new clone fails the gate.
Refactor new duplication instead of refreshing the baseline to make a check pass.
Use `workspace:cpd-baseline` only for an explicit, reviewed baseline update.

cargo-mutants mutates handwritten production Rust; generated models retain their
drift checks. It runs the baseline tests before mutations and fails on surviving
or timed-out mutants. The full workspace run is expensive (thousands of mutations)
and is intended for manual runs. The full GitHub workflow has a six-hour timeout;
it may need several shards to finish on this workspace. For example:

```sh
proto run moon -- run workspace:mutants -- --shard 1/16
```

Run every shard from `1/16` to `16/16` to check the full workspace. Pull request CI checks mutations
in changed lines using `--in-diff`; it does not replace the full test suite or
claim a clean result for unchanged code. Inspect reports in `.cache/mutants.out` when a
mutation gate fails.

For a focused investigation, keep the workspace selected and filter mutations by
source path or name. This preserves workspace consumer tests and the configured
`nrz-contract/codegen` feature:

```sh
proto run moon -- run workspace:mutants -- --file crates/nrz-source-bundle/src/handoff.rs
proto run moon -- run workspace:mutants -- --file src/detect/fs.rs --re 'normalize_path'
```

In the pinned cargo-mutants 27.1.0, struct-field deletion mutations bypass
`--re` and `--exclude-re` in the
[mutation visitor](https://github.com/sourcefrog/cargo-mutants/blob/v27.1.0/src/visit.rs#L704-L748).
Use `--file` to bound source scope and inspect `--list` before running: a name
filter alone does not guarantee that only the requested functions are tested.

Selecting only a package with `-p` can fail before testing any mutation: the
configured `nrz-contract/codegen` feature is not available to every package's
baseline build. A report with zero tested mutations is not a successful mutation
check; inspect its baseline outcome and log. A focused run proves only its selected
mutations. Do not disable the baseline or remove required features to obtain a
passing report.

Before starting a full run, `cargo mutants --workspace --list --json` lists the
current mutation scope without building it. Estimate execution time from completed
reports, including build and test phases, and run every shard against the same
source and configuration. Surviving mutations need investigation: they may expose
missing assertions, redundant code or a behaviorally equivalent mutation. A full
gate is clean only when it completes without survivors or timeouts; an interrupted
run or one completed shard does not establish that result.

A diff containing only tests, generated code or no mutatable Rust produces no
mutations; cargo-mutants may skip its baseline in that case. The ordinary
`workspace:check` gate still runs the required test suite.

## Generated contracts

The complete inputs needed by the CLI are included in this repository:
`api/openapi.json`, the operation inventory and `crates/nrz-contract/schemas`.
Use them to regenerate or test the SDK and artifact models:

```sh
proto run moon -- run workspace:api-generate
proto run moon -- run workspace:api-check
proto run moon -- run workspace:contracts-generate
proto run moon -- run workspace:contracts-check
cargo test --locked -p nrz-api -p nrz-contract
```

For command/API changes, add HTTP fixtures to exercise the generated request
and response types. An API extension can be proposed with a contract fixture;
maintainers coordinate its server implementation before accepting a new wire
contract. Access to server source is not part of the contribution workflow.

The [API guide](../api/README.md) explains operation selection and generator
pinning. The [public crate guide](../crates/README.md) covers artifact schemas.

## Functions runtime

Use `NRZ_FUNCTIONS_RUNTIME_PATH` to select a local standalone runtime binary.
Inspection and `functions check` use that exact binary without a development
release. See [Functions runtime development](functions-runtime-development.md)
for the identity and native test commands.

## Release ownership

CLI binary/npm releases remain independent of server releases and library Git
revisions. Run the release workflow on a reviewed public revision with the
selected stable/beta channel. Dagger derives versions, updates only the CLI
package's versions, and produces the release metadata. Native CI builds use
`Cargo.lock`; publishing verifies all assets before finalizing the release.

A local worktree can calculate an explicit release plan without committing or
publishing anything:

```sh
bun .dagger/scripts/capture-git-metadata.ts
dagger call release-metadata --source=. --git-metadata=.nrz-release/git.json \
  --channel=beta --version=0.41.0-beta.0
```

The version above is a dry-run example. The result reports the selected worktree
revision and whether it is dirty. Release preparation requires a clean checkout.
`release-metadata.json`, published with the archives and their checksums, binds
the OpenAPI, generator, shared contracts and Functions runtime used by the CLI.

User-facing compatibility changes and upgrade instructions are documented in
[the migration guide](breaking-changes.md).
