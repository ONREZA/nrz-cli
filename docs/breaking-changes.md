# Breaking Changes and Migration Guide

This document is the canonical, machine-readable migration guide for breaking
`nrz-cli` changes. `CHANGELOG.md` remains the release history; this file explains
required user actions. Sections marked `UNRELEASED` describe planned behavior
and must receive exact `from`/`to` versions before release.

## UNRELEASED: execution-context-v1

| Field | Value |
| --- | --- |
| Status | `PLANNED` |
| From version | `TBD` |
| To version | `TBD` |
| Required action | Upgrade `nrz-cli` and select an Environment context |
| Compatibility error | `CLI_UPDATE_REQUIRED` |
| Platform rollout | Coordinated Server, Builder and CLI cutover |

### Who must migrate

- users of `nrz deploy`, Environment-backed `nrz dev` or `nrz env`;
- CI using `--env` or `NRZ_ENV`;
- custom roles/API keys used for deploy or configuration materialization.

Built-in `Owner`, `Admin` and `Developer` roles receive the new execution
permission. Custom roles and API keys are not expanded automatically because
that would silently grant plaintext secret-use authority.

### Environment selection

Before:

```bash
nrz deploy --env preview
NRZ_ENV=preview nrz deploy
```

After:

```bash
nrz context use preview
nrz deploy

nrz deploy --environment preview
NRZ_ENVIRONMENT=preview nrz deploy
```

`deploy`, Environment-backed `dev` and `env exec` do not silently default to
production. Repository selection is stored in gitignored
`.onreza/environment.json`. Use `nrz dev --local` for an explicit local-only
development session without Server configuration.

### Secret input

Before:

```bash
nrz env set API_TOKEN secret-value --secret --env preview
```

After:

```bash
printf %s "$API_TOKEN" | nrz env set API_TOKEN --stdin --secret --environment preview
nrz env set API_TOKEN --from-file ./api-token.txt --secret --environment preview
```

Secret values are no longer accepted as positional arguments. Changing
secret/plain category or moving an existing legacy scope requires the explicit
safety flags reported by CLI remediation.

### Permission migration

Automation that materializes configuration must receive `env.materialize` in
addition to its existing deploy permissions. Generic `env.read` continues to
return public values and secret metadata, never secret plaintext.

Migration checklist:

1. Upgrade CLI to the exact version recorded in this section.
2. Replace `--env` with `--environment` and `NRZ_ENV` with
   `NRZ_ENVIRONMENT` in automation.
3. Run `nrz context use <environment>` in linked interactive repositories.
4. Grant `env.materialize` only to custom roles/API keys that execute code with
   platform configuration.
5. Replace positional secret values with `--stdin` or `--from-file`.
6. Run the affected command once in a non-production Environment.

### Stable failure behavior

| Error | Meaning | Remediation |
| --- | --- | --- |
| `CLI_UPDATE_REQUIRED` | CLI uses the retired execution protocol. | Upgrade to the recorded target version. |
| `ENV_MATERIALIZATION_FORBIDDEN` | Principal lacks plaintext execution permission. | Grant `env.materialize` or use a different principal. |
| `ENVIRONMENT_CONTEXT_STALE` | Saved Environment no longer belongs to the linked project/workspace. | Run `nrz context use` again. |
| `ENV_CONFIG_OVERRIDE_UNSUPPORTED` | Legacy model cannot store different values for the same key per Environment. | Use one legacy value/scope or wait for the Environment-first config model. |

