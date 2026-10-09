# Quality debt audit

This audit records the 2026-10-08 cleanup, starting from commit
`c924f655442e53123e286ca6bce8eade27a4189d`. Work proceeds in bounded batches of
a few hundred mutations, alongside gradual removal of JSCPD duplicates. A full
workspace mutation run is outside this batch. Local reports under `.cache` and
`.jscpd` are ignored artifacts, not durable release evidence.

## Duplicate code

The starting accepted JSCPD report contains 114 existing clones and 1,543
duplicated lines. The current cleanup reduces this to **49 clones and 632
duplicated lines**, with zero new clones. The reviewed baseline removes the
65 eliminated fingerprints and adds none; it does not accept new debt.

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

## Source bundle batch

The next batch starts from `ee1ff6dba339c1ddd13ec597f40d396e7304a261` and
selects all seven production files in `crates/nrz-source-bundle/src/` plus
`src/artifact/source_bundle_v1.rs`: **654 initial candidates**. The inventory
and the actual initial run select exactly the same mutation names. Both workers
run in isolated copies; changes to the working tree do not alter that run.
The initial source snapshot records all 440 tracked input hashes.
The complete initial run tested all 654 candidates: **166 missed, 439 caught,
49 unviable, zero timeouts**. It ran from 11:32:33 to 13:46:10 UTC on
2026-10-08. These are pre-correction results, not mutation acceptance.

The investigation reproduced these source bundle defects before correction:

- The verifier stopped after the first Zstd frame. A stream error after that
  frame was ignored, and concatenated frames could be accepted or rejected
  depending on chunk boundaries. Verification now reads the complete stream.
- TAR header checksums were not checked. Admission could accept an archive
  that the extraction library rejected. Verification now checks the checksum;
  handwritten archive fixtures also write valid checksums.
- Reserved metadata could be a symlink header with a JSON body. Metadata must
  use regular-file entries, matching the actual bundle writer.
- Invalid UTF-8 TAR names were silently replaced while extraction rejected
  them. Names and symlink targets now require exact UTF-8 decoding. Zstd decode
  failures also retain their own error code instead of being confused with
  source-stream failures.
- Dangling or consecutive PAX descriptors were admitted while extraction
  rejected them. Descriptor state now survives even an empty PAX body and must
  be consumed by exactly one following entry.
- Lexical symlink resolution did not model physical directory traversal:
  a missing directory followed by `..` could be admitted, while a valid parent
  traversal through a directory alias could be rejected. A shared archive path
  index now resolves physical components for both producer and verifier and
  rejects file/symlink ancestors of nested entries.
- Acyclic aliases could expand exponentially and lead to links that Linux
  cannot follow. Resolution counts all traversals, including repeated aliases,
  against the verified Linux limit of 40. Regression tests unpack and read
  a 40-link chain, and demonstrate `ELOOP` for 41 links and a branching alias
  graph. No arbitrary depth or archive-size policy was introduced.
- Dependency extraction created parent directories before checking their
  containment. A cross-root symlink followed by a descendant entry could create
  directories outside the tree before returning an error. Extraction now checks
  each parent component before creating the next directory.
- Packaging re-read files after scanning but checked only their size. Rewriting
  a file with different bytes of the same length produced an inconsistent
  archive. The writer now verifies the hash of the actual bytes being copied.
- Error cleanup could remove an already existing destination after `create_new`
  failed. Cleanup now belongs to the writer and starts only after file creation
  succeeds. Its failing regression followed a behavior-preserving extraction
  of the original cleanup policy into the writer entrypoint.

Runtime declaration validation shares borrowed argument and target checks rather
than cloning launch arguments and validating them twice. The redundant private
manifest/archive comparison was removed: both representations are constructed
from the same immutable entries, while independent consumer verification remains.
Unix and non-Unix implementations share a single function with platform blocks,
so unreachable host-specific duplicate function mutations disappear without
adding exclusions. The archive hashing writer only wraps its actual `File`
caller. Copy buffer arithmetic was replaced by its unchanged numeric value;
changing that buffer size does not change the published bytes.
PAX encoding and its framing budget use the same byte-preserving codec. The
accepted mutation selection includes the new `pax.rs` and `path_graph.rs`
owners; moving code does not remove it from acceptance.
The physical-parent regression also reproduced through the public deploy
scanner, before the bundle planner was called. Both readers now derive
`resolved_path` from the canonical filesystem path after raw-target validation,
preserving the raw target and root confinement. The scanner's complete
`src/deploy/scan.rs` file is included in mutation acceptance as well.

The first expanded run selected **689 candidates in 11 production files**,
matching the executed inventory exactly. `workspace:check` passed all 13 tasks
before that run. The complete report (14:20:05 to 16:20:17 UTC) records
**25 missed, 608 caught, 53 unviable and three timeouts**. It is not accepted:
follow-up edits were made while its isolated workers continued testing the
earlier snapshot, and the driver records those differing input hashes.

The follow-up consolidates redundant private path guards and directory error
classification. Scanner scenarios now check independent dependency roots,
pruning unreadable caches before traversal, all required LFS fields and its
1024-byte detection boundary, stderr reports and large-file ordering/boundaries.
Streaming file hashing delegates EOF and byte counting to `std::io::copy` with
a bounded SHA-256 sink. TAR state transitions now advance in the parser caller
before entry validation hooks; incomplete bodies return directly. PAX parsing
consumes checked record slices instead of a mutable arithmetic cursor. Timeouts
and exclusions are unchanged. A new empty-entry/unused-regular-linkname
preservation test passes before and after this last refactor. Its before source
was reconstructed from prior source reads, rather than retained verbatim;
the evidence records that limitation and verifies restoration of the after hash.

The source bundle tests now verify native closure custody with file-backed ELF
metadata fixtures rather than an ignored C compiler test. This checks closure
membership and publication ownership; it does not qualify execution by a native
runtime loader. MIME and raw PAX boundary tests also pass on the initial source
commit, independently of the fixes, preserving the existing wire behavior.

The accepted complete run tested **671 candidates in the same 11 production
files** from **16:32:38 to 18:25:01 UTC on 2026-10-08**: **618 caught,
53 unviable, zero survivors and zero timeouts**. Its unmutated baseline passed;
all mutation builds and viable mutation tests used `--workspace`, locked
dependencies and `nrz-contract/codegen`. The executed inventory matches the
selected names exactly. The unchanged-input check covers all 444 tracked and
new source/configuration/test files in its snapshot.

`workspace:check` passed all 13 tasks before that acceptance (one unchanged task was
cached). The accepted hashes identify the first batch snapshot; the subsequent
connected-path review below changes code and requires renewed acceptance. No mutation exclusion,
timeout policy or dependency lock was changed. This is a complete result for the
selected scope, not a full workspace mutation run.

Reports and snapshots live under `.cache/debt-review/source-bundle/`:
`accepted/mutants.out/outcomes.json`, `accepted-mutants.json`,
`accepted-source-snapshot.json`, `accepted-workspace-check.json`,
`accepted-driver-result.json` and `accepted-summary.json`. They are ignored
local evidence; this document is the durable record of the bounded batch.

The JSCPD baseline has been reduced from **64 clones / 875 duplicated lines**
to **52 clones / 679 duplicated lines**. Exactly 12 prior fingerprints were
removed; no fingerprint or multiplicity was added. Shared test setup preserves
scenario-specific wire values, error assertions and filesystem observations.
The accepted workspace gate checked the reduced baseline again.

### Connected-path review

The follow-up review reproduces TAR header-flavor and duplicate PAX-key path
disagreements against the actual `tar::Archive` extractor, plus an empty numeric
size field that the verifier accepted while the extractor rejected it. The
verifier now follows header flavor and rejects ambiguous PAX and empty numeric
fields without changing source wire format or error codes.

Production scan/classify/project/archive tests expose workspace `.bin` traversal
ordering, lost intermediate symlinks during pruning, and application output
removed by workspace dependency projection. These tests use the actual selected
scan instead of the prior test-only full-scan shortcut. Classifier path indexes
replace repeated whole-collection scans; an isolated debug workload with 10,000
regular files and 2,000 aliases improved from 4.537 seconds to 60.969 milliseconds
with the same 3,000 retained entries. The timing workload is not a mutation test.

Publication now binds single and multipart target metadata to the locally
verified source before reading or sending bytes. A regression previously
published replacement bytes under the original artifact identity. Single-object
reads are bounded by the expected size, multipart descriptions recheck the
aggregate source digest, and subsequent reads retain regular-file admission.
Handoff final names are claimed without replacement; a failed descriptor commit
releases and syncs its owned archive. Final acceptance for these follow-up changes
is pending completion of the approved producer and materializer corrections.

The extractor now checks both physical endpoints and intermediate traversal
custody against same-layer extracted dependency trees. Closed-tree EROFS
inspection uses the same physical resolver with actual Unix directories and
filenames, preserving empty-directory traversal, root aliases and backslash
filenames. The source archive's portable path grammar remains unchanged.

Two architectural corrections are approved: runtime-mounted validation needs
actual sibling trees to resolve cross-tree aliases, while the current public
Rust scope carries only mount names; producer filesystem containment can be
invalidated by replacing a parent directory after scan even when immutable
bytes remain correct. Both have small filesystem reproductions. The selected
corrections expand the Rust mount context with actual sibling trees and retain
one filesystem root handle from producer scan through publication preparation.
Both corrections now have frozen implementations and focused checks. Producer
acquires one shared directory handle before scanning and retains it through
pruning, Python/native validation, source planning and archive copying. Projected
archive paths retain their separate original source paths. A real root rename
and pathname replacement still reads the acquired tree; parent replacement by
an external alias is rejected during planning and during copying, with partial
archive cleanup. A 256-file archive succeeds with a 64-descriptor process limit.
The handle does not create a filesystem snapshot or change hardlink/mount policy.

Runtime mount requests now require actual extracted sibling descriptors. The
single physical graph checks endpoints, intermediate layer ownership and a
namespace boundary. Reproductions exposed synthetic-root exit/reentry and a
legacy V2 identity regression; both are corrected. The previous raw-depth marker
is retained only for policy identity after physical admission. It does not grant
path authority. Actual Linux reads and inspection agree on 40 versus 41 symlink
expansions. Final mutation verification remains pending; no complete follow-up
acceptance is claimed yet.

A public multipart publication test now verifies an actual 256 MiB + 4-byte
source, all 17 payloads and offsets, the short final part, reversed target order,
ETags and both completion phases. The test retains observed bytes instead of
asserting only transport calls. Profiling isolated debug SHA-256 cost: the same
test took 36.60 seconds, then 1.86 seconds with only the `sha2` dependency compiled
at optimization level 3. The dev package profile now keeps this optimization for
inherited test builds; production release settings and dependency versions stay
the same. Multipart iteration uses fixed offsets rather than mutable progress
counters; this public test passes before and after that refactor.

The follow-up duplicate baseline shrinks from 52 clones / 679 duplicated lines
to 49 / 632. Three additional fingerprints are removed and none are added.
The independent mutation selection covers shared source parsing, graph and
runtime contracts, scanner, publisher and handoff. It intentionally excludes
the producer custody and runtime-mounted materializer corrections; its result
alone cannot close those paths. This run uses an isolated source snapshot while
the main tree receives follow-up corrections, so it is diagnostic evidence for
that snapshot rather than acceptance of the current tree.

Nine survivors found during that diagnostic run have targeted follow-up checks.
The handoff checks use actual directory permissions and the public publication
path to detect lost preflight I/O errors and false success after directory-sync
admission fails. They also verify the resulting files, including the case where
both final files have been committed before the error. They do not establish
crash durability. An equivalent copy-buffer arithmetic mutation is removed by
using the same literal buffer size as the producer.

Extractor checks distinguish a valid alias to the dependency mount root from an
alias to its parent, which must remain outside the allowed endpoint scope even
when traversal visits an owned ancestor. Unix graph-constructor checks cover
NUL, empty, dot and parent segments for actual entries and explicit directories,
while retaining valid backslash filenames. Each of the six exact extractor/graph
mutations passes the previous owner tests and fails the new ones in an isolated
module harness. Those focused checks do not replace the final cargo-mutants run.


The independent diagnostic run completed all **675 selected candidates** from
19:28:25 to 21:39:35 UTC on 2026-10-08: **561 caught, 60 unviable, 53 missed and
one timeout** (exit 3). Executed names exactly match the selection. Workers used
isolated copies of the starting snapshot; the main tree received corrections
while they ran. The driver records every changed input and does not claim this
result as current-tree acceptance. The publisher survivors and chunk-arithmetic
timeout have focused follow-up corrections. Thirty exact behavioral publisher
mutations now fail the public tests; fourteen equivalent or arithmetic
representations are removed while preserving their exact values. A manual
UTF-8 decrement-loop mutation stalls, so the loop is replaced by standard
`floor_char_boundary` plus `truncate`, preserving the existing byte limit.
Timeout policy and exclusions are unchanged.

The subsequent integration review reproduces optional package metadata changing
after scan and removing its own scan witness through pruning. Pruning now binds
the consumed metadata bytes to their scanned size and hash, including a
`package.json` symlink's scanned regular target. Rooted access alone provides
containment, not this identity guarantee. Related LFS/Python qualification reads
now enforce the same criterion. A shared bounded streaming reader hashes the
bytes consumed by qualification and drains the same stream before allowing
success. It does not hash and then reread mutable data. Regressions first created
and unpacked archives containing an unresolved LFS pointer or native Python
payload that the guards had not qualified; the corrected guards reject these
cases, including in-place content changes while reading. Valid aliases still
pass using their scanned regular target witness. Project health autodetection now
runs before artifact-root acquisition as a planning input; its 15 existing
flag/config/autodetection checks pass before and after the order change.

This iteration's mutation acceptance is limited to the core file selection for
source parsing, scanning and publication: 695 candidates before the scanner
corrections, 692 afterward. The additional
672-candidate custody and 588-candidate planning inventories were enumerated but
have not been run; they belong to later iterations. The implemented producer,
native and materializer corrections retain their targeted regression evidence
and are also checked by the normal full workspace gate. No full-file mutation
acceptance is claimed for those deferred scopes.

The current selection retains full workspace consumer tests and the normal
baseline, features and locked dependencies. The driver runs `workspace:check`
first and records complete workspace input hashes and exact executed inventories.
The diagnostic core run completed all 695 candidates: 617 caught, 62 unviable,
15 missed and one timeout. Both workers match all 449 starting input hashes;
the main tree changed only in the scanner, its two test files and this document.
No deferred mutation batch ran.

Scanner follow-up checks distinguish missing optional roots from invalid
ancestors, actual filesystem entry types, Python src-package pruning, and the
exact 512 KiB metadata boundary. Redundant metadata prechecking is replaced by
one bounded max+1 read and a final length check. Qualification drain is bounded
by a standard caller-side Take; ordinary scanning still hashes through EOF.
The 21 corresponding current mutations are caught in focused replay; 144
connected tests pass with nine pre-existing ignored tests. This targeted proof
does not replace workspace acceptance. A full repeat of the corrected 692-candidate inventory was cancelled during
the workspace gate, before mutation execution: repeating unchanged portions of
the batch is outside the incremental verification needed here. This iteration
retains the complete 695-candidate diagnostic, focused before/after evidence
for its scanner findings, and a final current-tree workspace gate. It does not
claim a fresh zero-survivor run of all 692 current candidates. The bounded iteration is complete under this incremental verification scope.
The final workspace gate passed in 21.402 seconds (13 tasks, five cached),
including fresh workspace tests and CPD. The current report has 49 clones and
632 duplicated lines; relative to HEAD, 15 reviewed fingerprints are removed
and none added. No full workspace mutation run, deferred batch, commit or push
was performed.
