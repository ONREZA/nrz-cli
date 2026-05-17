# Build Settings Source Contract

`installCommand`, `buildCommand`, and `outputDirectory` are source-aware. The
value string alone is not enough to decide whether the CLI may apply local
fallbacks or framework analysis. At runtime the CLI first builds one
`onreza.toml`-shaped effective config in memory, then runs build/deploy from
that model only.

All three settings use the same source enum:

| Source | Meaning |
| --- | --- |
| `USER` | User-entered override in UI/API; explicit and authoritative. |
| `DETECTED` | Derived from repository/framework config; preferred, but non-user. |
| `PRESET` | Framework preset/default; fallback-compatible. |

## Output Directory

| Source | Meaning | CLI precedence | Missing path |
| --- | --- | --- | --- |
| `USER` | User-entered override in UI/API | exactly `outputDirectory` only | fail fast with `MISSING_BUILD_OUTPUT` |
| `DETECTED` | Derived from repository/framework config | current framework/SSR refinements of the detected hint, then detected path, then framework/SSR candidates, then local config defaults | fall back to other non-user candidates |
| `PRESET` | Framework preset/default | framework/SSR candidates, then preset path, then local config defaults | fall back to other non-user candidates |

The CLI evaluates this as ordered precedence tiers. A `.onreza/` manifest is
preferred only within the first tier that contains an existing directory; a
manifest in a lower-priority fallback directory must not override a higher-
priority existing output.

`USER` is the only exact output path. `DETECTED` is a pre-build hint, so a
generic framework container may be narrowed or superseded by the current
framework artifact analysis. Examples include `.next` refining to
`.next/standalone/` or `out/`, `.output` refining to `.output/public/`, and
framework client subdirectories such as `build/client/` or `dist/client/`.
As a compatibility exception, a user-sourced Next.js `.next` setting also
uses the concrete Next artifact when it exists (`.next/standalone/` for
PROCESS or `out/` for static export), because `.next` is the framework
container rather than a stable deploy root.
For monorepo Next.js standalone builds, `.next/standalone/` remains the artifact
root even when `server.js` lives in a nested app directory; only the generated
manifest entry is nested. Preparation keeps the same split: static/public assets
are copied beside the app server, while root-level runtime dependencies such as
missing Prisma client hash packages are copied into the standalone bundle root.
Nested server inference must prefer the generated Next.js entry or app-shaped
directory and ignore traced support files named `server.js`.

`frameworkPreset` selects the framework detector and compute heuristics. It does
not make the preset `outputDirectory` authoritative unless the source is `USER`.

## Commands

Command precedence is:

| Setting | CLI precedence |
| --- | --- |
| `buildCommand` | `--build-command` > local `[build].command` > server command/source > package.json auto-detect |
| `installCommand` | local `[build].install_command` > server command/source > package-manager auto-detect |

For output directories, local `[build].output_directory` is a `USER` source and
therefore exact/authoritative. `[build].output_dirs` remains only the fallback
search list used after framework/source-aware candidates.

Server command source affects empty or missing server values:

| Source | Empty command behavior |
| --- | --- |
| `USER` | Explicitly skip that command; do not auto-detect. |
| `DETECTED` | Treat detector absence as meaningful; do not auto-detect. |
| `PRESET` or missing source | Backward-compatible fallback to local auto-detect. |

Non-empty server commands are used regardless of source after CLI/local config
overrides. Older APIs that omit source metadata are treated as `PRESET`.

## Compute

`compute` is resolved after the output directory is selected:

| compute source | Behavior |
| --- | --- |
| manifest layers | Manifest targets are authoritative. |
| CLI/config override | Parsed `--compute`/`[deploy].compute` wins when no manifest target overrides it. |
| framework detection | Used when neither manifest nor explicit compute is present. |

Examples:

- Vite static with preset `dist` uses `dist/` and generates a STATIC manifest.
- Next.js preset `.next` may refine to `.next/standalone/` for PROCESS or `out/`
  for static export.
- Generic PROCESS with a `USER` output directory deploys only that directory; if
  it is missing, CLI must not silently fall back to `.` or `dist/`.
