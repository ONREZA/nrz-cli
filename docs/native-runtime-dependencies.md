# Native dependencies in platform build artifacts

The platform runner receives `ONREZA_RUNTIME_OS`, `ONREZA_RUNTIME_ARCH` and
`ONREZA_RUNTIME_LIBC` from the admitted Builder Agent runtime target. Missing or
unsupported values fail the build before installation. These values describe
the runtime rootfs, not the CLI binary's compilation platform.

For compute artifacts, packaging reads the traced packages' `os`, `cpu`, `libc`
and dependency metadata. It excludes an incompatible package only when a
compatible parent explicitly references it as optional. Required references
to an incompatible package fail packaging. Unknown metadata and unreferenced
packages are retained conservatively. Optional dependency declarations override
ordinary dependency declarations; optional peer dependencies are recognized.

Resolution follows the artifact's Node dependency ancestry and canonical
symlinks. A package containing another referenced package is retained. An
application path referencing an excluded package fails packaging. The source
workspace is unchanged: filtering precedes file hashes, manifest sizes and
source archive construction, so all published identities describe the same
filtered inventory. Local deploys do not infer a remote runtime target.

The archive regression test uses a fixture containing `sharp` plus Linux x64
glibc and musl optional packages. Set `NRZ_NATIVE_FIXTURE_ROOT` to the fixture
directory, then run:

```sh
cargo test --locked --bin nrz real_sharp_loads_and_encodes_an_image_from_the_pruned_source_archive -- --ignored
```

The test builds and extracts a source archive, then runs `server.js` with Node
and requires successful image encoding. The fixture must print
`sharp-runtime-ok` only after that operation. New CLI releases require a Builder
Agent that supplies the explicit target and a Builder rootfs pinned to that CLI.
