use std::io::{IsTerminal, Read, Write};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::{ApiClient, path_segment};
use crate::auth;
use crate::output;
use nrz::config;
use nrz::config::{EnvVisibility, ProjectConfig};

use super::env::{EnvArgs, EnvCommand};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvListResponse {
    env_vars: Vec<EnvVar>,
    total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvVar {
    key: String,
    #[serde(default)]
    value: Option<String>,
    is_secret: bool,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    scope_type: Option<String>,
    #[serde(default)]
    preview_branch: Option<String>,
    #[serde(default)]
    environments: Vec<EnvVarEnvironment>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvVarEnvironment {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetEnvBody<'a> {
    key: &'a str,
    value: &'a str,
    is_secret: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
    scope_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment_ids: Option<Vec<String>>,
    replace_scope: bool,
    change_category: bool,
    confirmed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetEnvResponse {
    id: String,
    key: String,
    created: bool,
    #[serde(default)]
    is_secret: Option<bool>,
    #[serde(default)]
    scope_type: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    warnings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DeleteEnvResponse {
    deleted: bool,
    message: String,
}

pub async fn run(
    args: EnvArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;
    let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
    match args.command {
        EnvCommand::List { environment } => {
            let environment_id = crate::execution_context::resolve_optional(
                &client,
                &project_id,
                std::path::Path::new("."),
                environment.as_deref(),
                None,
            )
            .await?
            .map(|context| context.environment_id);
            list(&client, &project_id, environment_id.as_deref(), json).await
        }
        EnvCommand::Set {
            key,
            value,
            stdin,
            from_file,
            secret,
            plain,
            note,
            environment,
            all,
            replace_scope,
            change_category,
            yes,
        } => {
            let is_secret = resolve_category(secret, plain, config.env_visibility(&key))?;
            let value = read_set_value(value, stdin, from_file.as_deref(), is_secret)?;
            let (scope_type, environment_ids) = resolve_write_scope(
                &client,
                &project_id,
                std::path::Path::new("."),
                environment.as_deref(),
                all,
            )
            .await?;
            let safety_prompt = describe_safety_change(
                &key,
                is_secret,
                &scope_type,
                environment_ids.as_deref(),
                replace_scope,
                change_category,
            );
            confirm_safety(replace_scope || change_category, yes, stdin, &safety_prompt)?;
            set(
                &client,
                SetEnvRequest {
                    project_id: &project_id,
                    key: &key,
                    value: &value,
                    is_secret,
                    note: note.as_deref(),
                    scope_type: &scope_type,
                    environment_ids,
                    replace_scope,
                    change_category,
                    json,
                },
            )
            .await
        }
        EnvCommand::Delete { key, all, yes } => {
            if !all {
                bail!("legacy environment definitions can only be deleted with explicit --all");
            }
            confirm_safety(
                true,
                yes,
                false,
                &format!("Delete the one legacy definition {key} from every target?"),
            )?;
            delete(&client, &project_id, &key, true, json).await
        }
        EnvCommand::Validate { environment } => {
            let context = crate::execution_context::resolve_for_mutation(
                &client,
                &project_id,
                std::path::Path::new("."),
                environment.as_deref(),
                None,
            )
            .await?;
            let materialized = crate::execution_context::materialize_desired(
                &client,
                &context,
                context.source_ref.as_deref(),
                "EXEC",
            )
            .await?;
            validate(&materialized.variables, json, config)
        }
        EnvCommand::Exec {
            environment,
            command,
        } => {
            crate::execution_context::warn_local_dotenv_drift(std::path::Path::new("."), json)?;
            let context = crate::execution_context::resolve_for_mutation(
                &client,
                &project_id,
                std::path::Path::new("."),
                environment.as_deref(),
                None,
            )
            .await?;
            let materialized = crate::execution_context::materialize_desired(
                &client,
                &context,
                context.source_ref.as_deref(),
                "EXEC",
            )
            .await?;
            exec_with_environment(command, &materialized)
        }
    }
}

async fn list(
    client: &ApiClient,
    project_id: &str,
    environment_id: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let mut resp: EnvListResponse = client
        .get(&format!("/v1/projects/{}/env", path_segment(project_id)))
        .await
        .context("failed to fetch environment variables")?;
    if let Some(environment_id) = environment_id {
        resp.env_vars
            .retain(|variable| env_var_matches_environment(variable, environment_id));
        resp.total = resp.env_vars.len() as u64;
    }

    if json {
        output::json_output(&resp);
    } else if resp.env_vars.is_empty() {
        eprintln!("  No environment variables found.");
        return Ok(());
    } else {
        eprintln!();
        eprintln!(
            "  {:<30} {:<30} {}",
            console::style("Key").bold(),
            console::style("Value").bold(),
            console::style("Scope").bold(),
        );
        eprintln!("  {}", "-".repeat(70));

        for v in &resp.env_vars {
            let display_val = if v.is_secret {
                "*****".to_string()
            } else {
                output::terminal_line(v.value.as_deref().unwrap_or("-"))
            };
            let key = output::terminal_line(&v.key);
            let scope = output::terminal_line(&format_scope(v));
            eprintln!("  {key:<30} {display_val:<30} {scope}");
        }
        eprintln!();
    }

    Ok(())
}

pub(crate) fn env_var_matches_environment(variable: &EnvVar, environment_id: &str) -> bool {
    match variable.scope_type.as_deref() {
        Some("SELECTED") => variable
            .environments
            .iter()
            .any(|environment| environment.id == environment_id),
        _ => true,
    }
}

fn format_scope(v: &EnvVar) -> String {
    match v.scope_type.as_deref() {
        Some("ALL") | None => "ALL".to_string(),
        Some("SELECTED") if !v.environments.is_empty() => v
            .environments
            .iter()
            .map(|e| e.name.as_deref().unwrap_or(&e.id))
            .collect::<Vec<_>>()
            .join(", "),
        Some(other) => other.to_string(),
    }
}

struct SetEnvRequest<'a> {
    project_id: &'a str,
    key: &'a str,
    value: &'a str,
    is_secret: bool,
    note: Option<&'a str>,
    scope_type: &'a str,
    environment_ids: Option<Vec<String>>,
    replace_scope: bool,
    change_category: bool,
    json: bool,
}

async fn set(client: &ApiClient, request: SetEnvRequest<'_>) -> anyhow::Result<()> {
    let body = SetEnvBody {
        key: request.key,
        value: request.value,
        is_secret: request.is_secret,
        note: request.note,
        scope_type: request.scope_type,
        environment_ids: request.environment_ids,
        replace_scope: request.replace_scope,
        change_category: request.change_category,
        confirmed: true,
    };

    let resp: SetEnvResponse = client
        .post(
            &format!("/v1/projects/{}/env", path_segment(request.project_id)),
            &body,
        )
        .await
        .context("failed to set environment variable")?;

    if request.json {
        output::json_output(&resp);
    } else {
        let action = if resp.created { "Created" } else { "Updated" };
        output::success(
            false,
            format!("{action} {}", console::style(request.key).bold()),
            output::Phase::Env,
        );
    }

    Ok(())
}

async fn delete(
    client: &ApiClient,
    project_id: &str,
    key: &str,
    confirmed: bool,
    json: bool,
) -> anyhow::Result<()> {
    let resp: DeleteEnvResponse = client
        .delete(&format!(
            "/v1/projects/{}/env/{}?all=true&confirmed={confirmed}",
            path_segment(project_id),
            path_segment(key)
        ))
        .await
        .context("failed to delete environment variable")?;

    if json {
        output::json_output(&serde_json::json!({
            "key": key,
            "deleted": resp.deleted,
        }));
    } else {
        output::success(
            false,
            format!("Deleted {}", console::style(key).bold()),
            output::Phase::Env,
        );
    }

    Ok(())
}

async fn resolve_write_scope(
    client: &ApiClient,
    project_id: &str,
    project_dir: &std::path::Path,
    environment: Option<&str>,
    all: bool,
) -> anyhow::Result<(String, Option<Vec<String>>)> {
    if all {
        return Ok(("ALL".to_string(), None));
    }
    let context = crate::execution_context::resolve_for_mutation(
        client,
        project_id,
        project_dir,
        environment,
        None,
    )
    .await?;
    Ok(("SELECTED".to_string(), Some(vec![context.environment_id])))
}

pub(crate) fn read_set_value(
    value: Option<String>,
    stdin: bool,
    from_file: Option<&str>,
    secret: bool,
) -> anyhow::Result<String> {
    let mut value = match (value, stdin, from_file) {
        (Some(value), false, None) => {
            if secret {
                bail!("secret values must use --stdin or --from-file, not --value");
            }
            value
        }
        (None, true, None) => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes)?;
            normalize_stdin_value(bytes)?
        }
        (None, false, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read UTF-8 value from {path}"))?,
        _ => bail!("provide exactly one of --value, --stdin, or --from-file"),
    };
    if value.contains('\0') {
        bail!("environment value must not contain NUL");
    }
    value.shrink_to_fit();
    Ok(value)
}

pub(crate) fn normalize_stdin_value(bytes: Vec<u8>) -> anyhow::Result<String> {
    let mut value = String::from_utf8(bytes).context("stdin value must be valid UTF-8")?;
    if value.ends_with("\r\n") {
        value.truncate(value.len() - 2);
    } else if value.ends_with('\n') {
        value.pop();
    }
    Ok(value)
}

pub(crate) fn resolve_category(
    secret: bool,
    plain: bool,
    declared: Option<EnvVisibility>,
) -> anyhow::Result<bool> {
    match (secret, plain, declared) {
        (true, false, _) => Ok(true),
        (false, true, _) => Ok(false),
        (true, true, _) => bail!("choose at most one category: --secret or --plain"),
        (false, false, Some(EnvVisibility::Sensitive)) => Ok(true),
        (false, false, Some(EnvVisibility::Plain)) => Ok(false),
        (false, false, None) => {
            bail!("choose --secret or --plain, or declare the key in [env.declarations]")
        }
    }
}

fn describe_safety_change(
    key: &str,
    is_secret: bool,
    scope_type: &str,
    environment_ids: Option<&[String]>,
    replace_scope: bool,
    change_category: bool,
) -> String {
    let mut changes = Vec::new();
    if replace_scope {
        let target = match (scope_type, environment_ids) {
            ("SELECTED", Some(ids)) => format!("SELECTED ({})", ids.join(", ")),
            _ => scope_type.to_string(),
        };
        changes.push(format!("move the one legacy definition to {target}"));
    }
    if change_category {
        changes.push(format!(
            "change category to {}",
            if is_secret { "SECRET" } else { "PLAIN" }
        ));
    }
    format!(
        "Update {key}: {}? This replaces the existing metadata shown by the previous API remediation",
        changes.join(" and ")
    )
}

fn confirm_safety(required: bool, yes: bool, stdin_used: bool, prompt: &str) -> anyhow::Result<()> {
    if !required || yes {
        return Ok(());
    }
    if stdin_used || !std::io::stdin().is_terminal() {
        bail!("confirmation required; rerun with --yes");
    }
    eprint!("  {} [y/N] ", output::terminal_line(prompt));
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        bail!("operation cancelled");
    }
    Ok(())
}

fn exec_with_environment(
    command: Vec<String>,
    materialized: &crate::execution_context::MaterializedExecutionContext,
) -> anyhow::Result<()> {
    let (program, args) = command.split_first().context("command is required")?;
    let mut child = std::process::Command::new(program);
    child.args(args);
    for (key, value) in crate::execution_context::execution_environment(materialized) {
        child.env(key, value);
    }
    for key in crate::execution_context::private_cli_environment_keys() {
        child.env_remove(key);
    }
    let status = child.status().context("failed to start env exec command")?;
    if !status.success() {
        bail!("env exec command exited with {status}");
    }
    Ok(())
}

// ── env validate ────────────────────────────────────────────

fn validate(
    variables: &std::collections::HashMap<String, String>,
    json: bool,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    if config.env.declarations.is_empty() {
        if json {
            output::json_output(&serde_json::json!({
                "valid": true,
                "missing": [],
                "present": [],
                "undeclared": [],
                "message": "No [env] declarations in onreza.toml"
            }));
        } else {
            eprintln!("  No [env] declarations found in onreza.toml");
        }
        return Ok(());
    }

    let mut missing = Vec::new();
    let mut present = Vec::new();

    for (key, decl) in &config.env.declarations {
        if variables.contains_key(key) {
            present.push(key.as_str());
        } else if decl.required {
            missing.push((key.as_str(), decl.visibility));
        }
    }

    missing.sort_by_key(|(k, _)| *k);
    present.sort();

    let valid = missing.is_empty();

    if json {
        let missing_json: Vec<_> = missing
            .iter()
            .map(|(key, vis)| {
                serde_json::json!({
                    "key": key,
                    "visibility": vis.as_str()
                })
            })
            .collect();

        output::json_output(&serde_json::json!({
            "valid": valid,
            "missing": missing_json,
            "present": present,
        }));
        if !valid {
            std::process::exit(1);
        }
        return Ok(());
    } else {
        eprintln!();
        if valid {
            output::success(
                false,
                format!("All {} required variable(s) are set", present.len()),
                output::Phase::Env,
            );
        } else {
            eprintln!(
                "  {} Missing required environment variables:\n",
                console::style("✗").red().bold()
            );
            for (key, vis) in &missing {
                let tag = if *vis == EnvVisibility::Sensitive {
                    " (sensitive)"
                } else {
                    ""
                };
                eprintln!(
                    "    {} {}{}",
                    console::style("-").red(),
                    output::terminal_line(key),
                    tag
                );
            }
            eprintln!();
            eprintln!("  Set them with:");
            for (key, visibility) in &missing {
                let command = if *visibility == EnvVisibility::Sensitive {
                    format!("printf %s \"$VALUE\" | nrz env set {key} --secret --stdin")
                } else {
                    format!("nrz env set {key} --plain --value <value>")
                };
                eprintln!(
                    "    {}",
                    console::style(output::terminal_line(&command)).dim()
                );
            }
        }
        eprintln!();
    }

    if !valid {
        anyhow::bail!("{} required environment variable(s) missing", missing.len());
    }

    Ok(())
}

pub(crate) fn validate_materialized_env_for_deploy(
    variables: &std::collections::HashMap<String, String>,
    json: bool,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let mut missing = config
        .env
        .declarations
        .iter()
        .filter(|(key, declaration)| declaration.required && !variables.contains_key(key.as_str()))
        .map(|(key, _)| key.as_str())
        .collect::<Vec<_>>();
    missing.sort_unstable();
    if missing.is_empty() {
        return Ok(());
    }
    output::warn(
        json,
        format!(
            "Missing required environment variables in the admitted snapshot: {}",
            missing.join(", ")
        ),
        output::Phase::Env,
    );
    anyhow::bail!(
        "{} required environment variable(s) missing from deployment snapshot",
        missing.len()
    )
}
