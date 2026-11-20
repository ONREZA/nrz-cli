use std::io::{IsTerminal, Read, Write};
use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{
    ApiClient, ServiceConfigDeleteBody, ServiceConfigResponse, ServiceConfigSaveBody,
    ServiceConfigVariable,
};
use crate::execution_context::{self, ExecutionContext};
use crate::output;
use nrz::config::{EnvVisibility, ProjectConfig};

use super::env::{EnvArgs, EnvCommand};

#[cfg(test)]
#[path = "env_handler_wire_tests.rs"]
mod wire_tests;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvListResponse {
    env_vars: Vec<nrz_api::Config200ResponseEffective>,
    total: usize,
    service_id: Uuid,
    environment_id: Option<Uuid>,
}

impl From<ServiceConfigResponse> for EnvListResponse {
    fn from(value: ServiceConfigResponse) -> Self {
        let mut env_vars = if value.context.environment.is_some() {
            value.effective
        } else {
            value
                .variables
                .into_iter()
                .filter(|row| row.environment_id.is_none())
                .map(|row| nrz_api::Config200ResponseEffective {
                    id: row.id,
                    key: row.key,
                    value: row.value,
                    is_secret: row.is_secret,
                    note: row.note,
                    environment_id: row.environment_id,
                    workspace_env_var_id: row.workspace_env_var_id,
                    scope_type: row.scope_type,
                    environment_ids: row.environment_ids,
                    environment_types: row.environment_types,
                    preview_branch: row.preview_branch,
                    updated_at: row.updated_at,
                    inherited: false,
                    additional_properties: Default::default(),
                })
                .collect()
        };
        for variable in &mut env_vars {
            if variable.is_secret {
                variable.value = None;
            }
        }
        Self {
            total: env_vars.len(),
            env_vars,
            service_id: value.context.service_id,
            environment_id: value
                .context
                .environment
                .map(|environment| environment.product_environment_id),
        }
    }
}

pub async fn run(
    args: EnvArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let (client, project_id) =
        crate::cli::remote::project_client(token, workspace, args.project_id.as_deref(), config)?;
    run_for_client(&client, &project_id, args, json, config, Path::new(".")).await
}

async fn run_for_client(
    client: &ApiClient,
    project_id: &str,
    args: EnvArgs,
    json: bool,
    config: &ProjectConfig,
    project_dir: &Path,
) -> anyhow::Result<()> {
    let service_id =
        execution_context::select_service(client, project_id, project_dir, args.service).await?;
    let product_id = project_id.parse().context("invalid Product ID")?;
    let service = service_id.to_string();
    match args.command {
        EnvCommand::List { environment, all } => {
            let context = resolve_scope(
                client,
                project_id,
                project_dir,
                &service,
                environment.as_deref(),
                all,
                false,
            )
            .await?;
            list(client, product_id, service_id, context.as_ref(), json).await
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
            let context = resolve_scope(
                client,
                project_id,
                project_dir,
                &service,
                environment.as_deref(),
                all,
                true,
            )
            .await?;
            let environment_id = context
                .as_ref()
                .map(|context| context.environment_id.parse::<Uuid>())
                .transpose()
                .context("invalid environment ID")?;
            let safety_prompt = describe_safety_change(
                &key,
                is_secret,
                environment_id,
                replace_scope,
                change_category,
            );
            confirm_safety(replace_scope || change_category, yes, stdin, &safety_prompt)?;
            set(
                client,
                SetEnvRequest {
                    project_id: product_id,
                    service_id,
                    key: &key,
                    value: &value,
                    is_secret,
                    note: note.as_deref(),
                    environment_id,
                    branch: context
                        .as_ref()
                        .and_then(|context| context.source_ref.as_deref()),
                    replace_scope,
                    change_category,
                    confirmed: yes || replace_scope || change_category,
                    json,
                },
            )
            .await
        }
        EnvCommand::Delete {
            key,
            environment,
            all,
            yes,
        } => {
            let context = resolve_scope(
                client,
                project_id,
                project_dir,
                &service,
                environment.as_deref(),
                all,
                true,
            )
            .await?;
            let environment_id = context
                .as_ref()
                .map(|context| context.environment_id.parse::<Uuid>())
                .transpose()
                .context("invalid environment ID")?;
            confirm_safety(
                true,
                yes,
                false,
                &format!("Delete {key} from {}?", scope_label(environment_id)),
            )?;
            delete(client, product_id, service_id, &key, context.as_ref(), json).await
        }
        EnvCommand::Validate { environment } => {
            let context = execution_context::resolve_for_service_mutation(
                client,
                project_id,
                project_dir,
                environment.as_deref(),
                None,
                Some(&service),
            )
            .await?;
            let materialized = execution_context::materialize_desired(
                client,
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
            execution_context::warn_local_dotenv_drift(project_dir, json)?;
            let context = execution_context::resolve_for_service_mutation(
                client,
                project_id,
                project_dir,
                environment.as_deref(),
                None,
                Some(&service),
            )
            .await?;
            let materialized = execution_context::materialize_desired(
                client,
                &context,
                context.source_ref.as_deref(),
                "EXEC",
            )
            .await?;
            exec_with_environment(command, &materialized)
        }
    }
}

async fn resolve_scope(
    client: &ApiClient,
    project_id: &str,
    project_dir: &Path,
    service: &str,
    environment: Option<&str>,
    all: bool,
    required: bool,
) -> anyhow::Result<Option<ExecutionContext>> {
    if all {
        return Ok(None);
    }
    if required {
        execution_context::resolve_for_service_mutation(
            client,
            project_id,
            project_dir,
            environment,
            None,
            Some(service),
        )
        .await
        .map(Some)
    } else {
        execution_context::resolve_optional_for_service(
            client,
            project_id,
            project_dir,
            environment,
            None,
            Some(service),
        )
        .await
    }
}

async fn list(
    client: &ApiClient,
    project_id: Uuid,
    service_id: Uuid,
    context: Option<&ExecutionContext>,
    json: bool,
) -> anyhow::Result<()> {
    let environment_id = context
        .map(|context| context.environment_id.parse::<Uuid>())
        .transpose()
        .context("invalid environment ID")?;
    let resp: EnvListResponse = client
        .service_config(
            project_id,
            service_id,
            environment_id,
            context.and_then(|context| context.source_ref.as_deref()),
        )
        .await
        .context("failed to fetch Service environment variables")?
        .into();
    if json {
        output::json_output(&resp);
    } else if resp.env_vars.is_empty() {
        eprintln!("  No environment variables found.");
    } else {
        eprintln!();
        eprintln!(
            "  {:<30} {:<30} {}",
            console::style("Key").bold(),
            console::style("Value").bold(),
            console::style("Scope").bold()
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

fn format_scope(variable: &nrz_api::Config200ResponseEffective) -> String {
    let mut scope = variable.environment_id.map_or_else(
        || {
            if variable.scope_type == nrz_api::Project200Response3EnvVarScopeType::Selected {
                format!(
                    "DEFAULT ({})",
                    variable
                        .environment_ids
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                "DEFAULT".to_string()
            }
        },
        |id| id.to_string(),
    );
    if variable.inherited {
        scope.push_str(" (inherited)");
    }
    scope
}

fn scope_label(environment_id: Option<Uuid>) -> String {
    environment_id.map_or_else(
        || "the Service default".to_string(),
        |id| format!("Environment override {id}"),
    )
}

struct SetEnvRequest<'a> {
    project_id: Uuid,
    service_id: Uuid,
    key: &'a str,
    value: &'a str,
    is_secret: bool,
    note: Option<&'a str>,
    environment_id: Option<Uuid>,
    branch: Option<&'a str>,
    replace_scope: bool,
    change_category: bool,
    confirmed: bool,
    json: bool,
}

async fn set(client: &ApiClient, request: SetEnvRequest<'_>) -> anyhow::Result<()> {
    let body = ServiceConfigSaveBody {
        environment_id: request.environment_id,
        branch: request.branch.map(str::to_owned),
        variables: vec![ServiceConfigVariable {
            key: request.key.to_string(),
            value: Some(request.value.to_string()),
            is_secret: request.is_secret,
            note: request.note.map(|note| Some(note.to_string())),
            ..Default::default()
        }],
        replace_scope: Some(request.replace_scope),
        change_category: Some(request.change_category),
        confirmed: Some(request.confirmed),
        ..Default::default()
    };
    let resp = client
        .save_service_config(request.project_id, request.service_id, body)
        .await
        .context("failed to set Service environment variable")?;
    if request.json {
        output::json_output(
            &serde_json::json!({"key":request.key,"serviceId":request.service_id,"environmentId":request.environment_id,"message":resp.message}),
        );
    } else {
        output::success(
            false,
            format!("Saved {}", console::style(request.key).bold()),
            output::Phase::Env,
        );
    }
    Ok(())
}

async fn delete(
    client: &ApiClient,
    project_id: Uuid,
    service_id: Uuid,
    key: &str,
    context: Option<&ExecutionContext>,
    json: bool,
) -> anyhow::Result<()> {
    let environment_id = context
        .map(|context| context.environment_id.parse::<Uuid>())
        .transpose()
        .context("invalid environment ID")?;
    let resp = client
        .delete_service_config(
            project_id,
            service_id,
            key,
            ServiceConfigDeleteBody {
                environment_id,
                branch: context.and_then(|context| context.source_ref.clone()),
                confirmed: true,
                ..Default::default()
            },
        )
        .await
        .context("failed to delete Service environment variable")?;
    if json {
        output::json_output(
            &serde_json::json!({"key":key,"deleted":true,"serviceId":service_id,"environmentId":environment_id,"message":resp.message}),
        );
    } else {
        output::success(
            false,
            format!("Deleted {}", console::style(key).bold()),
            output::Phase::Env,
        );
    }
    Ok(())
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
    environment_id: Option<Uuid>,
    replace_scope: bool,
    change_category: bool,
) -> String {
    let mut changes = Vec::new();
    if replace_scope {
        changes.push("replace the existing legacy scope".to_string());
    }
    if change_category {
        changes.push(format!(
            "change category to {}",
            if is_secret { "SECRET" } else { "PLAIN" }
        ));
    }
    format!(
        "Update {key} in {}: {}?",
        scope_label(environment_id),
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
                format!(
                    "All {} required variable(s) are set",
                    config
                        .env
                        .declarations
                        .values()
                        .filter(|decl| decl.required)
                        .count()
                ),
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
    Err(crate::output::coded_error(
        "INVALID_CONFIG",
        format!(
            "{} required environment variable(s) missing from deployment snapshot",
            missing.len()
        ),
    ))
}
