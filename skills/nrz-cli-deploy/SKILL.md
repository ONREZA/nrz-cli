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
1. Detect framework and persist it:
```bash
nrz detect --save
```
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
nrz rollback --project-id <project_id> --deployment-id <deployment_id>
```
