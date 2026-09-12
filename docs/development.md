# Contributing to nrz-cli

## CLI and framework adapters

Clone this public repository and install the pinned tools with `mise install`.
The CLI, detection, framework adapters, generated SDK and shared Rust libraries
build from this checkout. Ordinary development and fixture tests run locally without an ONREZA account.

```sh
cargo run -- detect --dir /path/to/project --json
cargo test --locked --workspace
mise run check
```

Detection lives under `src/detect`, build orchestration under `src/build`, and
the Next.js adapter under `src/nextjs_adapter.rs` and `src/nextjs_adapter/`.
Behavioral fixtures live under `tests`. Add a representative project fixture and
assert its detection/build output when introducing a framework. Keep framework
behavior in these public modules; server-side publication consumes their typed
artifacts.

## Generated contracts

The complete inputs needed by the CLI are included in this repository:
`api/openapi.json`, the operation inventory and `crates/nrz-contract/schemas`.
Use them to regenerate or test the SDK and artifact models:

```sh
mise run api:generate
mise run api:check
mise run contracts:generate
mise run contracts:check
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
