---
name: nrz-cli-env-db-kv
description: Use when users manage ONREZA environment variables, D1-compatible database operations, migrations, or local KV data through nrz commands.
---

# nrz CLI Env, DB, and KV

## When to use
- Syncing `.env.local` with platform variables
- Running migrations and SQL locally or remotely
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

## Database and migrations
```bash
# local db
nrz db info
nrz db execute "SELECT name FROM sqlite_master WHERE type='table';"

# migration workflow
nrz db migrate create add_users_table
nrz db migrate status
nrz db migrate apply

# remote
nrz db migrate status --remote --project-id <project_id>
nrz db migrate apply --remote --project-id <project_id>
```

## KV operations
```bash
nrz kv set feature_flag enabled --ttl 3600
nrz kv get feature_flag
nrz kv list --prefix feature_ --limit 100
nrz kv delete feature_flag
```

## Safety checklist
- Confirm local vs remote target before destructive actions.
- Use `--force` only when command is explicitly approved.
- Prefer `--json` in automation for reliable parsing.
