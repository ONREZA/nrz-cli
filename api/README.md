# Platform API client

`openapi.json` is a schema-preserving selection from the actual server OpenAPI.
`operations.json` chooses operation IDs; it does not define request or response
types. `client/src/generated` belongs to oas3-gen. The Cargo manifest and HTTP
policy surrounding it are ordinary maintained code.

Builds use committed Rust sources and `Cargo.lock`. Generation is an explicit
development operation; ordinary builds do not run it.

## Local contract development

The selected OpenAPI and operation inventory are committed here. Regenerate and
verify them from this checkout:

```sh
mise run api:generate
mise run api:check
cargo test --locked -p nrz-api
```

HTTP fixtures under `client/tests` exercise serialization, responses and wire
behavior without a live platform. For a proposed API change, include a schema
fixture and its behavioral test. Maintainers coordinate server compatibility
before accepting wire changes. `--from /path/to/openapi.json` selects another
OpenAPI document explicitly when testing a supplied contract.

## Generator provenance

`generator.lock.json` pins the upstream revision and SHA-256 of a reviewed patch
to the generator's source. First generation builds this pinned tool under the
checkout's disposable cache; subsequent generations reuse it. The patch includes
its behavioral tests and regenerated upstream fixtures. It changes optional
nullable properties, required/default semantics, typed raw HTTP operations and
bidirectional serde for CLI output, typed nullable objects/arrays, and exact
URL path separators. Nullable enum unions and recursive JSON null values retain
their wire semantics. Fixed header pairs retain positional string types and exact
lengths as Rust tuples. JSON Schema constants are enforced during serialization
and deserialization, so equal-shaped responses with different `kind` values
remain distinct Rust enum variants. It never rewrites generated Rust text.

A future upstream/fork release can replace the source patch after the same
contract tests pass. `NRZ_OPENAPI_GENERATOR_SOURCE=/path/to/oas3-gen` selects
a local generator checkout explicitly, without publishing it.

Regular API requests use generated operation methods, models and response
parsers. Raw methods retain status, headers and body for HTTP policy and device
authorization. SDK compilation alone is insufficient: serialization, defaults,
error responses and command workflows must pass behavioral tests.

Authentication, projects, domains, environment variables, execution context and
framework-detection sync, build-log sessions, deployment observation, preview
access, deployment listing, rollback, runtime logs, functions, edge rules,
deployment admission/source registration and managed database commands use the
generated client.
Malformed success responses
are compatibility failures; their deserialization errors omit received values
because auth/environment responses can contain secrets. Network/HTTP errors
retain the status and retry policy.

The operation selector rejects endpoints without an explicit success response,
so an incomplete OpenAPI document cannot silently produce an unusable operation.

Shared source-publisher upload, completion, status and runtime-artifact operations
also use this client. The generic CLI HTTP methods and handwritten success-response
envelope detection have been removed. Publication retains retry deadlines,
idempotency identities and independent durable artifact verification.

Each CLI release publishes `release-metadata.json` alongside the binary archives.
It records the reviewed source revision, SHA-256 digests of the selected OpenAPI,
operation inventory, generator and generated Rust, shared artifact schemas, and
the exact Functions runtime pin. The publisher checks these inputs against the
release checkout, includes the metadata in checksums, and reads back published
asset bytes before finalizing the release. Local metadata dry runs report dirty
source explicitly; release preparation requires a clean reviewed checkout.
