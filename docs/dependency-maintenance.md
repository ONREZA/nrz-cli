# Dependency maintenance

## Version policy

All seven workspace packages inherit `rust-version = "1.99"` from the root
manifest. `.prototools` and `rust-toolchain.toml` select the exact development
compiler. Keep the minimum supported compiler distinct from that tool pin.

For Rust libraries, use Cargo's default compatible requirements. `"1.2.3"`
means `>=1.2.3, <2`; `"0.9.2"` means `>=0.9.2, <0.10`. Neither is an exact pin.
Do not widen a pre-1.0 dependency to `"0"` just to admit breaking minor updates.
Keep meaningful minimum versions when callers require newer APIs. `Cargo.lock`
selects exact resolved versions; CI and release builds use `--locked`.
See the [Cargo dependency reference](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html).

Exact pins remain appropriate for tools, code generation and immutable runtime
artifacts. Update them together with their dependent checks and lockfiles:

| Owner | Coupled inputs |
| --- | --- |
| Proto/Moon | `.prototools`, `.moon/toolchains.yml`, `rust-toolchain.toml` |
| Dagger | `.prototools`, `dagger.json`, bootstrap checksum, `.dagger/package.json` and lock |
| OpenAPI generator | `api/generator.lock.json`, reviewed source patch, `oas3-gen-support = "=0.29.0"`, generated output |
| Shared transport | Iroh/bao Git revisions, including the matching deployment workspace patches |
| Runtime/native catalogs | Platform-owned public exports, imported through `gen-contract --from`; uv additionally requires archive and extracted-binary hashes in `python_toolchain.rs` |
| Functions runtime | Signed release identity, manifest and verification key; preserve signature/preflight validation |

Major updates are evaluated with their callers, release notes and behavioral
checks. A larger version number alone does not justify moving to prereleases or
replacing a shared protocol implementation.

## 2026-10-26 audit

Compatible Rust updates: uuid 1.29.0 and zip 9.1.2 (including zip's typed-path
0.13 dependency). `pep508_rs` now permits compatible 0.9.x updates. Removed unused
direct dependencies: root `bytes`, contract `chrono`, publisher `hex` and `libc`.
These crates can still occur transitively; removal does not mean purging them
from the dependency graph.

Developer tool updates include Proto 0.63.2, Moon 2.6.4, Bun 1.4.4,
Dagger 0.21.11 and cargo-binstall 1.26.1. CI also moves to checkout 7.0.1,
setup-node 7.1.0 and npm 12.2.1. Release npm commands use Node 24.22.1,
OIDC permissions and no static npm token; registry setup disables package-manager
caching. Alpine 3.24.3 replaces the unsupported 3.20 packaging/bootstrap image.
With npm 12, global installation requires the package-specific script permission
documented in the [installation guide](../README.md#installation).
Runtime/native projections were imported
from deployment commit `338972c575802e81e4409115bb63a3d5dd12d53c`; the four imported
schemas were unchanged. Python patches, uv, Dart and Flutter now match that export.
Importing catalogs does not prove execution of every native SDK or platform.

Remaining older versions and compatibility paths have specific consumers:

- `schemars08` 0.8.22 is required by the latest stable typify 0.8.0 and the shared
  schema exporter. Typify's 0.10 alpha still requires schemars 0.8.22. Keep the
  stable generator until an upstream migration can preserve the schema contract.
- Cargo audit found no known vulnerabilities in the inspected graph. It reports
  `paste` 1.0.15 as unmaintained through Iroh/netwatch/netlink. It is explicitly
  excluded from this task; no audit suppression was added. Audit results are a
  dated snapshot, not a claim about future advisories.
- Source/Runtime Graph compatibility APIs are called by deployment's retained
  artifact conversion and compute readers. Rejection of obsolete writer inputs
  is validation and must remain.
- KV `--env`/`NRZ_ENV`, `--prod`, `--filter`, config aliases and credentials-file
  migration still implement documented or existing user behavior. Local source
  searches cannot establish that no external user needs them.
- `package-lock.json` still has release consumers: `release-plan.ts` updates its
  version and the release workflow commits it. Removing it requires changing
  those consumers together.
- TypeScript 6 is used by the Dagger SDK's compiler API. The TypeScript 7 alias
  checks release scripts; it does not replace the SDK's compiler dependency.

No server dependency revision, published release or customer state is changed
by this checkout update. A shared-library rollout needs a reviewed Git revision,
downstream checks and the relevant runtime acceptance separately.
