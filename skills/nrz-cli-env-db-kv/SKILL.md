---
name: nrz-cli-env-db-kv
description: Use when users manage ONREZA environment variables, managed PostgreSQL operations, or local KV data through nrz commands.
---

# nrz CLI Env, DB, and KV

## When to use
- Managing platform variables without local plaintext copies
- Running SQL against managed PostgreSQL from the local CLI
- Managing local KV data during development

## Environment variables
```bash
# select one exact Environment for this checkout
nrz context use <environment>

# inspect metadata (secret plaintext is never returned)
nrz env list --environment <environment> --project-id <project_id>

# write one plain or secret value
nrz env set PUBLIC_URL --value https://example.com --plain --environment <environment> --project-id <project_id>
printf %s "$API_TOKEN" | nrz env set API_TOKEN --stdin --secret --environment <environment> --project-id <project_id>
nrz env delete OLD_KEY --all --yes --project-id <project_id>

# validate or execute with an ephemeral materialized snapshot
nrz env validate --environment <environment> --project-id <project_id>
nrz env exec --environment <environment> --project-id <project_id> -- <command>
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
- Never copy secret values into argv or dotenv files; use `--stdin` or `--from-file`.
- Use `--force` only when command is explicitly approved.
- Prefer `--json` in automation for reliable parsing.
