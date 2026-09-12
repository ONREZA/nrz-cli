# Public shared Rust libraries

These crates are maintained here alongside `nrz-api` and the CLI. Contributors
build and test all of them from this workspace.
Detection and framework adapters remain in the CLI source tree.

| Crate | Responsibility |
| --- | --- |
| `nrz-contract` | Generated artifact and Edge Rules models, committed schema inputs and codegen |
| `nrz-source-bundle` | Source manifest, archive verification and build handoff |
| `nrz-runtime-artifact` | Runtime graph and launch contract validation |
| `nrz-dependency-materializer` | Runtime dependency closure and image materialization |
| `nrz-source-publisher` | Typed platform publication, retries and durable readback |
| `../api/client` (`nrz-api`) | OpenAPI-generated platform HTTP operations and models |

Run `cargo test --locked --workspace` for ordinary offline-fixture tests. Native
Functions integration has its own explicit runtime input; see
[Functions runtime development](../docs/functions-runtime-development.md).

## Working on artifact contracts

The versioned schemas used by these crates are included in
`nrz-contract/schemas`. Generate and validate the Rust models locally:

```sh
mise run contracts:generate
mise run contracts:check
cargo test --locked --workspace
```

Propose schema changes together with fixtures that demonstrate the intended
artifact behavior. Maintainers coordinate changes to the platform wire contract.
Generated Rust is not edited by hand. `--check --from <schema directory>` can
also compare the committed inputs against an explicitly supplied schema export.
HTTP contracts use the separate [OpenAPI workflow](../api/README.md).

Applications can consume these crates at a selected public Git revision,
independently of CLI binary releases. This does not require a temporary CLI
release when testing library changes.
