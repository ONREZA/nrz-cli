# Quality debt audit

This audit records the 2026-10-08 cleanup, starting from commit
`c924f655442e53123e286ca6bce8eade27a4189d`. Work proceeds in bounded batches of
a few hundred mutations, alongside gradual removal of JSCPD duplicates. A full
workspace mutation run is outside this batch. Local reports under `.cache` and
`.jscpd` are ignored artifacts, not durable release evidence.

## Duplicate code

The starting accepted JSCPD report contains 114 existing clones and 1,543
duplicated lines. The current cleanup reduces this to **64 clones and 875
duplicated lines**, with zero new clones. The reviewed baseline removes the
50 eliminated fingerprints and adds none; it does not accept new debt.

In the starting report, counting only filenames would label nine as production
pairs and one as a production/test pair. Reading the matched source changes that
classification: `src/functions_runtime/tests.rs` is a test module, and the JSON
fixture in `crates/nrz-runtime-artifact/src/source_graph.rs:614` is inside its
`#[cfg(test)]` module. The actual split is **seven production pairs and 107 test
pairs**, with no production/test pair.

The seven production matches are repeated remote project/client setup in domain,
environment, deployment and log commands; two project-command request setup
blocks; and repeated remote-context and file-metadata handling in rules commands.
This batch consolidates those shared owners and repeated test setup. Remaining
test matches need individual review: input/output literals that independently
check a wire contract should remain independent. Do not update the reviewed
baseline just to pass a cleanup gate.

The `source_graph.rs`/`runtime_publication.rs` JSON match is retained deliberately.
Both sides construct verified dependency manifests, but the graph test uses a
fixed descriptor while the publisher test binds its digest and size to the actual
image being uploaded. Sharing this fixture through a production API would add a
test-only dependency between crates and couple two independent contract checks.
The report's filename alone does not justify that change.

## Mutation evidence

At the starting commit, `cargo mutants --workspace --list --json` enumerates
**8,074** mutations: 6,611 in
`nrz` and 1,463 in shared crates. This lists candidates without testing them.
The configured gate keeps workspace consumer tests, `nrz-contract/codegen`,
locked dependencies, and a baseline run. Generated code and test paths remain
excluded; handwritten production code has no debt exceptions.

The completed accepted report in `.cache/debt-review/accepted/outcomes.json`
ran from **08:46:04 to 09:21:06 UTC**: 337 selected mutations, 297 caught,
40 unviable, zero survivors and zero timeouts. Its baseline succeeded. Of the
337 candidates, 309 are in `src/detect/ssr.rs`; the remaining 28 are in build,
application-runtime binding, cache, deploy-plan, Python toolchain and runtime
resolver code. Earlier reports for that cleanup had 84 survivors, then three
array-expression survivors; the final report completed after those were addressed.

The report predates the commit and contains no commit SHA. Its adjacent
`source-snapshot.json` provides hashes for 31 relevant files, which match the
recorded commit. This supports the selected source and tests, not a claim about
every workspace input or subsequent working-tree changes. The accepted summary
explicitly records `full_workspace_mutation_run: false`. Even the SSR file has
407 current candidates, leaving 98 outside the accepted diff selection.

The completed run took about 35 minutes with two workers; completed mutation
build/test phases averaged 11.73 seconds. Applying that rate to 8,074 candidates
estimates **roughly 13 hours** at two workers, before differences in build cost,
timeouts, and fresh baselines. This is an estimate from a focused run, not a full
run measurement. The manual GitHub workflow allows six hours and has no shard
matrix; it may not finish. This explains why the current cleanup uses bounded
batches rather than claiming a full result. See [development.md](development.md)
for the optional full-run procedure.

## Current command batch

The current explicit file selection covers `domains_handler.rs`,
`env_handler.rs`, `projects_handler.rs`, `rules_handler.rs`, `deployments.rs`,
`logs.rs` and the new `cli/remote.rs`: **230 candidates**. Its source
snapshot is `.cache/debt-review/cli-commands/source-snapshot.json`. The initial
run completed from 09:55:13 to 10:40:40 UTC: **133 missed, 83 caught, 14 unviable,
zero timeouts**. Its baseline passed, but mutation acceptance failed (exit 2).
The preserved report is `initial/mutants.out/outcomes.json` under that directory.
Corrections must be verified against the same file scope.

The investigation reproduced an `env validate` reporting bug: one present
required declaration plus one present optional declaration was reported as
"All 2 required variable(s) are set". The new CLI regression failed before the
fix; the success message now counts required declarations. JSON still lists all
present declared keys and only missing required keys. Additional command tests
exercise real HTTP requests, machine/human output and terminal confirmation,
rather than reconstructing request DTOs independently of command execution.

Unused deployment deserialization and its unreachable fallback statuses were
removed. Responses are decoded by the generated SDK and converted into the CLI's
output-only DTO. Rules serialization also removes a repeated `position` filter
and unreachable unknown-field handling after strict authoring validation. TOML
key escaping uses the library serializer; semantic round trips cover dotted,
quoted and Unicode keys. Local file status drops impossible missing-path states.
The same seven-file selection now lists **206 candidates**; the reduction comes
from deleted dead and redundant paths, not mutation exclusions.

The standard `workspace:check` gate passed, as did the separately invoked ignored
native artifact-root regression. Both phases completed within the initial
open-wake job `job-1113113-1791453239541850912-0`.

The corrected batch completed from **10:49:15 to 11:21:17 UTC**: **206 mutations,
193 caught, 13 unviable, zero missed and zero timeouts**. Its unmutated baseline
passed, and the job exited 0. `workspace:check` passed all 13 tasks before this
run. The final JSCPD gate reports 64 clones, 875 duplicated lines and zero new
clones; the reviewed baseline is a strict subset of the original (50 removed,
zero additions).

Evidence is preserved under `.cache/debt-review/cli-commands/`:
`corrected/mutants.out/outcomes.json`, `corrected-mutants.json`,
`corrected-source-snapshot.json` and `accepted-summary.json`. All 440 recorded
source/input hashes matched the working tree at acceptance; only this document
was subsequently updated to record the result. The completed open-wake job is
`job-3426978-1791456510611105336-0`. This is acceptance of the seven-file batch,
not the full workspace. No mutation exclusions or new duplicate fingerprints
were introduced.

Plain `git diff` omits untracked helpers. Build the diff from tracked changes plus
`git diff --no-index -- /dev/null <path>` for every untracked Rust file; the latter
returns exit status 1 when it produces a diff. Test-only additions are still
excluded by the mutation configuration. Then list before running the task gate:

```sh
cargo mutants --workspace --in-diff /tmp/nrz-debt-working.diff --list --json
proto run moon -- run workspace:mutants-diff -- /tmp/nrz-debt-working.diff
```

The current batch uses explicit `--file` arguments for the seven paths above;
the diff command is a narrower alternative for later edits. Keep `--workspace` and the
configured baseline; package-only selection can fail to resolve the required
codegen feature. A focused PASS proves only its enumerated mutations. Empty
`missed.txt`, an inventory, an interrupted run, or one successful shard does not
establish a full workspace PASS.
