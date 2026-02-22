---
name: nrz-cli-project-bootstrap
description: Use when users initialize, link, and configure ONREZA projects with nrz, including workspace selection and project metadata setup.
---

# nrz CLI Project Bootstrap

## Use cases
- New repository onboarding to ONREZA
- Linking an existing local directory to a platform project
- Creating projects and switching workspaces

## Bootstrap flow
```bash
# initialize local scaffold (onreza.toml + .onreza/)
nrz init

# create on platform and link immediately
nrz init --create --name my-app

# or link an existing platform project
nrz init --project-id proj_abc123
# alternatively:
nrz link --project-id proj_abc123
```

## Workspace and project management
```bash
nrz workspace list
nrz workspace switch <workspace_slug>

nrz projects list --limit 20
nrz projects create --name my-app --framework astro --link
nrz projects info <project_id>
```

## Recommended follow-up
```bash
nrz detect --save
nrz build
nrz deploy
```

## Automation notes
- In CI/non-interactive contexts, pass `--project-id`, `--token`, `--workspace`, and `--json`.
- Keep `onreza.toml` committed; keep `.onreza/` local and gitignored.
