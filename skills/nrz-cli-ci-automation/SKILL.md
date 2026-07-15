---
name: nrz-cli-ci-automation
description: Use when users build CI pipelines around nrz and need fully non-interactive, JSON-first automation for detect, build, env checks, and deploy.
---

# nrz CLI CI Automation

## Goal
Produce deterministic CI steps for `nrz` with machine-readable output and no interactive prompts.

## Non-interactive rules
- Prefer `--json` for command output.
- Pass auth/context explicitly via flags or env:
  - `NRZ_TOKEN`
  - `NRZ_WORKSPACE`
- Use `--environment`/`NRZ_ENVIRONMENT` for platform execution context.
- Legacy `--env`/`NRZ_ENV` selects only a local KV namespace.
- Pass `--project-id` when available to skip project selection prompts.

## Recommended CI sequence
```bash
set -euo pipefail

nrz detect --save --json
nrz build --json
nrz env validate --environment production --project-id "$NRZ_PROJECT_ID" --json
nrz deploy \
  --environment production \
  --project-id "$NRZ_PROJECT_ID" \
  --json \
  --token "$NRZ_TOKEN" \
  --workspace "$NRZ_WORKSPACE"
```

## CI diagnostics
On failures, fetch deployment and logs:
```bash
nrz deployments --project-id "$NRZ_PROJECT_ID" --limit 10 --json
nrz logs --project-id "$NRZ_PROJECT_ID" --limit 200 --json
```

## Guardrails
- Do not use `nrz login` in CI.
- Avoid interactive destructive operations without `--force`.
- Keep `onreza.toml` as the source of truth for `build`, `deploy`, `env` declarations.
