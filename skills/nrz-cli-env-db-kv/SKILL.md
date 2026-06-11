---
name: nrz-cli-env-db-kv
description: Use when users manage ONREZA environment variables, managed PostgreSQL operations, or local KV data through nrz commands.
---

# nrz CLI Env, DB, and KV

## When to use
- Syncing `.env.local` with platform variables
- Running SQL against managed PostgreSQL from the local CLI
- Managing local KV data during development

## Environment variables
```bash
# inspect
nrz env list --project-id <project_id>

# pull platform vars to file
nrz env pull .env.local --project-id <project_id>

# push local vars (declared vars only)
nrz env push .env.local --declared-only --project-id <project_id>

# validate required vars from [env.declarations]
nrz env validate --project-id <project_id>
```

## Database
```bash
nrz db info
nrz db query "SELECT 1;" --project-id <project_id>
nrz db schema --project-id <project_id>
nrz db branches --project-id <project_id>
```

## KV operations
```bash
nrz kv set feature_flag enabled --ttl 3600 --env development
nrz kv get feature_flag --env development
nrz kv list --prefix feature_ --limit 100 --env preview
nrz kv delete feature_flag --env preview
```

## Safety checklist
- Confirm local vs remote target before destructive actions.
- Use `--force` only when command is explicitly approved.
- Prefer `--json` in automation for reliable parsing.
