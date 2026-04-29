# Output Directory Contract

`outputDirectory` is source-aware. The path string alone is not enough to decide
whether the CLI may apply framework analysis.

| Source | Meaning | CLI precedence | Missing path |
| --- | --- | --- | --- |
| `USER` | User-entered override in UI/API | exactly `outputDirectory` only | fail fast with `MISSING_BUILD_OUTPUT` |
| `DETECTED` | Derived from repository/framework config | detected path, then framework/SSR candidates, then local config defaults | fall back to other non-user candidates |
| `PRESET` | Framework preset/default | framework/SSR candidates, then preset path, then local config defaults | fall back to other non-user candidates |

The CLI evaluates this as ordered precedence tiers. A `.onreza/` manifest is
preferred only within the first tier that contains an existing directory; a
manifest in a lower-priority fallback directory must not override a higher-
priority existing output.

`frameworkPreset` selects the framework detector and compute heuristics. It does
not make the preset `outputDirectory` authoritative unless the source is `USER`.

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
