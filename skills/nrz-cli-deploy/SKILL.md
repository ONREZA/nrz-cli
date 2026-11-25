---
name: nrz-cli-deploy
description: Use when users need to deploy ONREZA projects with nrz, troubleshoot failed deployments, or choose correct compute mode and entrypoint settings.
---

# nrz CLI Deploy

## When to use
- First deployment of a project
- Failed `nrz deploy` in local or CI
- Compute mismatch (`static`, `process`)

## Standard workflow
1. Inspect the local build/deploy configuration without fetching platform settings:
```bash
nrz config explain --local
```
Use `nrz detect` to see what the CLI auto-detects. Use `nrz detect --save` only if you intend to update `[project].framework` in an existing `onreza.toml`; review the diff before deploying.
2. Validate build output:
```bash
nrz build
```
3. Deploy:
```bash
# local
nrz deploy

# CI/non-interactive
nrz deploy --environment production --project-id <project_id> --json --token "$NRZ_TOKEN" --workspace <workspace_slug>
```

To validate a deployment plan without publishing, run `nrz deploy --dry --json`. Unlike `nrz config explain --local`, dry deploy can run the project's build and ONREZA Functions preflight.

## Troubleshooting map
- Build output or manifest issue:
  - For static/process deployments, set `[deploy].compute` in `onreza.toml`.
- Process entrypoint error (common for Next.js/Nuxt):
  - Set `[deploy].entry` in `onreza.toml`.
  - Retry with `nrz deploy --compute process`.
- Environment validation error:
```bash
nrz env validate --environment <environment> --project-id <project_id>
nrz env list --environment <environment> --project-id <project_id>
```
## Post-deploy checks
```bash
nrz deployments --project-id <project_id> --limit 5
nrz logs --project-id <project_id> --limit 200
nrz rollback --project-id <project_id> --environment <environment> --list
nrz rollback --project-id <project_id> --environment <environment> --release-id <release_id>
```
