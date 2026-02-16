use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::link::project_ref;
use crate::output;

use super::env::{EnvArgs, EnvCommand};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvListResponse {
    env_vars: Vec<EnvVar>,
    total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvVar {
    key: String,
    value: Option<String>,
    is_secret: bool,
    /// "ALL" or ["PRODUCTION", "PREVIEW", ...]
    target: serde_json::Value,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetEnvBody<'a> {
    key: &'a str,
    value: &'a str,
    is_secret: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetEnvResponse {
    key: String,
    created: bool,
    #[serde(default)]
    warnings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct DeleteEnvResponse {
    #[allow(dead_code)]
    deleted: bool,
}

pub async fn run(
    args: EnvArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;
    let project_id = project_ref::resolve_project_id(args.project_id.as_deref())?;

    match args.command {
        EnvCommand::List => list(&client, &project_id, json).await,
        EnvCommand::Set { key, value, secret } => {
            set(&client, &project_id, &key, &value, secret, json).await
        }
        EnvCommand::Delete { key } => delete(&client, &project_id, &key, json).await,
        EnvCommand::Pull { file } => pull(&client, &project_id, &file, json).await,
    }
}

async fn list(client: &ApiClient, project_id: &str, json: bool) -> anyhow::Result<()> {
    let resp: EnvListResponse = client
        .get(&format!("/v1/projects/{}/env", project_id))
        .await
        .context("failed to fetch environment variables")?;

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
            console::style("Target").bold(),
        );
        eprintln!("  {}", "-".repeat(70));

        for v in &resp.env_vars {
            let display_val = if v.is_secret {
                "*****".to_string()
            } else {
                v.value.as_deref().unwrap_or("-").to_string()
            };
            let target = format_target(&v.target);
            eprintln!("  {:<30} {:<30} {}", v.key, display_val, target);
        }
        eprintln!();
    }

    Ok(())
}

fn format_target(target: &serde_json::Value) -> String {
    match target {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        _ => "ALL".to_string(),
    }
}

async fn set(
    client: &ApiClient,
    project_id: &str,
    key: &str,
    value: &str,
    secret: bool,
    json: bool,
) -> anyhow::Result<()> {
    let body = SetEnvBody {
        key,
        value,
        is_secret: secret,
    };

    let resp: SetEnvResponse = client
        .post(&format!("/v1/projects/{}/env", project_id), &body)
        .await
        .context("failed to set environment variable")?;

    if json {
        output::json_output(&resp);
    } else {
        output::success(false, format!("Set {}", console::style(key).bold()));
    }

    Ok(())
}

async fn delete(client: &ApiClient, project_id: &str, key: &str, json: bool) -> anyhow::Result<()> {
    let _resp: DeleteEnvResponse = client
        .delete(&format!("/v1/projects/{}/env/{}", project_id, key))
        .await
        .context("failed to delete environment variable")?;

    if json {
        output::json_output(&serde_json::json!({
            "key": key,
            "deleted": true,
        }));
    } else {
        output::success(false, format!("Deleted {}", console::style(key).bold()));
    }

    Ok(())
}

async fn pull(client: &ApiClient, project_id: &str, file: &str, json: bool) -> anyhow::Result<()> {
    let content: String = client
        .get(&format!("/v1/projects/{}/env/export-file", project_id))
        .await
        .context("failed to pull environment variables")?;

    std::fs::write(file, &content).with_context(|| format!("failed to write {file}"))?;

    if json {
        output::json_output(&serde_json::json!({
            "file": file,
            "status": "ok",
        }));
    } else {
        output::success(false, format!("Written to {}", console::style(file).bold()));
    }

    Ok(())
}
