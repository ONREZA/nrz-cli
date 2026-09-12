# Local Functions runtime development

`nrz functions check` and deployment preflight use the native Functions runtime.
The CLI writes the captured source into a temporary snapshot, imports it through
the embedded inspection module and checks its exported handlers. Changes to the
working directory after collection cannot change the code being inspected.
The inspector evaluates module initialization and `config`; it does not invoke
user handlers. Fetch and platform bindings are disabled during inspection, and
the subprocess receives no CLI token or project environment variables.

The default runtime comes from the signed, immutable release pinned in the CLI.
`assets/functions-runtime.lock.json` owns its release ID, manifest URLs and
manifest SHA-256. The resolver embeds that file; release provenance includes the
same pin and the signing-key digest.
To use a locally built standalone runtime:

```sh
export NRZ_FUNCTIONS_RUNTIME_PATH=/absolute/path/to/onreza-functions-runtime
cargo run -- functions runtime status --json
cargo run -- functions check --dir /path/to/project --json
```

The override is explicit and fails if the file is missing or not executable.
Its identity is `local-sha256:<binary digest>`, so it cannot be confused with a
signed release. Rebuilding the executable changes that identity without requiring
a development release. `runtime status`, `runtime path`, `runtime install` and
preflight use the same selection.

Behavioral tests can run against a qualified native binary without downloading
anything during the test:

```sh
NRZ_TEST_FUNCTIONS_RUNTIME=/absolute/path/to/onreza-functions-runtime \
  cargo test --locked -p nrz --bin nrz functions_runtime -- --include-ignored
```

These tests cover real ESM/TypeScript evaluation, computed configuration, handler
exports, trigger compatibility, captured sources, missing dependencies and disabled
fetch. Ordinary unit tests also cover signed installation, local binary identity
and bounded stdio messages.

Discovery only captures source bytes. Runtime preflight evaluates `config`, resolves
the function name and validates the resulting metadata against generated OpenAPI
types. A publish payload requires successful inspection of every captured function.
Server ingest consumes the same generated types and validates names, paths,
handler/trigger consistency, byte limits and snapshot hashes. It does not parse
or execute JavaScript. Local inspection is developer feedback, not a server
attestation: runtime isolation still applies when deployed code executes.

Module initialization and declaration failures carry `INVALID_CONFIG`. Runtime
startup, protocol and control timeout failures remain infrastructure errors.
The native integration suite covers this distinction explicitly; ordinary unit
tests do not download a Functions runtime.
