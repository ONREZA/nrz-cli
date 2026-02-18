use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

use super::env::{EnvArgs, EnvCommand};

// Secret auto-detection: variables whose name contains any of these
// keywords are automatically marked as secrets.
const SECRET_KEYWORDS: &[&str] = &[
    "KEY",
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PRIVATE",
    "CERT",
    "CREDENTIALS",
];

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BulkEnvVar<'a> {
    key: &'a str,
    value: &'a str,
    is_secret: bool,
}

#[derive(Debug, Serialize)]
struct BulkEnvBody<'a> {
    variables: Vec<BulkEnvVar<'a>>,
    overwrite: bool,
}

#[derive(Debug, Deserialize)]
struct BulkVarResult {
    key: String,
    success: bool,
    /// True when overwrite=false and the key already existed.
    /// Skipped entries are excluded from both "uploaded" and "failed" counts.
    #[serde(default)]
    skipped: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BulkEnvResponse {
    #[allow(dead_code)]
    message: String,
    results: Vec<BulkVarResult>,
}

pub(crate) struct ParsedVar {
    pub(crate) key: String,
    pub(crate) value: String,
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
        EnvCommand::List => list(&client, &project_id, json).await,
        EnvCommand::Set { key, value, secret } => {
            set(&client, &project_id, &key, &value, secret, json).await
        }
        EnvCommand::Delete { key } => delete(&client, &project_id, &key, json).await,
        EnvCommand::Pull { file } => pull(&client, &project_id, &file, json).await,
        EnvCommand::Push {
            file,
            overwrite,
            dry_run,
            secret,
        } => {
            push(
                &client,
                &project_id,
                &file,
                overwrite,
                dry_run,
                secret,
                json,
            )
            .await
        }
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

async fn push(
    client: &ApiClient,
    project_id: &str,
    file: &str,
    overwrite: bool,
    dry_run: bool,
    force_secret: bool,
    json: bool,
) -> anyhow::Result<()> {
    let content =
        std::fs::read_to_string(file).with_context(|| format!("failed to read {file}"))?;

    let result = parse_dotenv(&content);
    let parsed = result.vars;

    if result.skipped_lines > 0 {
        output::warn(
            json,
            format!(
                "{} line(s) in {file} skipped (no '=' found or invalid key)",
                result.skipped_lines
            ),
        );
    }

    if parsed.is_empty() {
        if json {
            output::json_output(&serde_json::json!({
                "file": file,
                "total": 0,
                "message": "No variables found in file"
            }));
        } else {
            eprintln!("  No variables found in {file}");
        }
        return Ok(());
    }

    // Dry-run: pre-fetch existing keys to show an accurate preview
    if dry_run {
        let (vars_to_push, skipped) = if overwrite {
            (parsed.iter().collect::<Vec<_>>(), 0)
        } else {
            let existing: EnvListResponse = client
                .get(&format!("/v1/projects/{}/env", project_id))
                .await
                .context("failed to fetch existing variables")?;
            let existing_keys: std::collections::HashSet<&str> =
                existing.env_vars.iter().map(|v| v.key.as_str()).collect();
            let to_push: Vec<_> = parsed
                .iter()
                .filter(|v| !existing_keys.contains(v.key.as_str()))
                .collect();
            let skipped = parsed.len() - to_push.len();
            (to_push, skipped)
        };

        if json {
            let preview: Vec<_> = vars_to_push
                .iter()
                .map(|v| {
                    let is_secret = force_secret || is_secret_by_name(&v.key);
                    serde_json::json!({ "key": v.key, "isSecret": is_secret })
                })
                .collect();
            output::json_output(&serde_json::json!({
                "dryRun": true,
                "file": file,
                "wouldUpload": preview.len(),
                "wouldSkip": skipped,
                "variables": preview,
            }));
        } else {
            eprintln!();
            eprintln!(
                "  {} Dry run — {} variable(s) would be uploaded, {} skipped",
                console::style("~").cyan().bold(),
                vars_to_push.len(),
                skipped
            );
            for v in &vars_to_push {
                let is_secret = force_secret || is_secret_by_name(&v.key);
                let tag = if is_secret { " (secret)" } else { "" };
                eprintln!("    {} {}{}", console::style("+").green(), v.key, tag);
            }
            eprintln!();
        }
        return Ok(());
    }

    // Actual push: send all vars with overwrite flag, server handles skip/upsert
    let variables: Vec<BulkEnvVar<'_>> = parsed
        .iter()
        .map(|v| BulkEnvVar {
            key: &v.key,
            value: &v.value,
            is_secret: force_secret || is_secret_by_name(&v.key),
        })
        .collect();

    let body = BulkEnvBody {
        variables,
        overwrite,
    };
    let resp: BulkEnvResponse = client
        .post(&format!("/v1/projects/{}/env/bulk", project_id), &body)
        .await
        .context("failed to push environment variables")?;

    let failed: Vec<&BulkVarResult> = resp
        .results
        .iter()
        .filter(|r| !r.success && !r.skipped)
        .collect();
    let skipped = resp.results.iter().filter(|r| r.skipped).count();
    let uploaded = resp.results.iter().filter(|r| r.success).count();

    if json {
        output::json_output(&serde_json::json!({
            "file": file,
            "uploaded": uploaded,
            "skipped": skipped,
            "failed": failed.len(),
            "errors": failed.iter().map(|r| serde_json::json!({
                "key": r.key,
                "error": r.error,
            })).collect::<Vec<_>>(),
        }));
        if !failed.is_empty() {
            anyhow::bail!("{} variable(s) failed to upload", failed.len());
        }
        return Ok(());
    }

    output::success(
        false,
        format!(
            "Pushed {uploaded} variable(s) from {}{}",
            console::style(file).bold(),
            if skipped > 0 {
                format!(", {skipped} skipped (already exist)")
            } else {
                String::new()
            }
        ),
    );
    for r in &failed {
        output::warn(
            false,
            format!(
                "Failed to push {}: {}",
                r.key,
                r.error.as_deref().unwrap_or("unknown error")
            ),
        );
    }

    if !failed.is_empty() {
        anyhow::bail!("{} variable(s) failed to upload", failed.len());
    }

    Ok(())
}

// ── .env parser ──────────────────────────────────────────────

pub(crate) struct ParseResult {
    pub(crate) vars: Vec<ParsedVar>,
    pub(crate) skipped_lines: usize,
}

pub(crate) fn parse_dotenv(content: &str) -> ParseResult {
    let mut vars = Vec::new();
    let mut skipped_lines = 0;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip optional `export ` prefix (only when followed by whitespace)
        let line = if let Some(rest) = line.strip_prefix("export") {
            if rest.starts_with(char::is_whitespace) {
                rest.trim_start()
            } else {
                line
            }
        } else {
            line
        };

        let Some(eq_pos) = line.find('=') else {
            skipped_lines += 1;
            continue;
        };

        let key = line[..eq_pos].trim();
        let raw_val = &line[eq_pos + 1..];

        if key.is_empty() || key.contains(char::is_whitespace) {
            skipped_lines += 1;
            continue;
        }

        let value = parse_dotenv_value(raw_val);
        vars.push(ParsedVar {
            key: key.to_string(),
            value,
        });
    }

    ParseResult {
        vars,
        skipped_lines,
    }
}

pub(crate) fn parse_dotenv_value(raw: &str) -> String {
    let raw = raw.trim();

    // Double-quoted: parse character by character to correctly handle \" inside the value
    if let Some(inner) = raw.strip_prefix('"') {
        let mut result = String::new();
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' => break, // closing quote
                '\\' => match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                },
                other => result.push(other),
            }
        }
        return result;
    }

    // Single-quoted: no escape processing, find closing single quote
    if let Some(inner) = raw.strip_prefix('\'')
        && let Some(end) = inner.find('\'')
    {
        return inner[..end].to_string();
    }

    // Unquoted: strip inline comment
    raw.split(" #")
        .next()
        .unwrap_or(raw)
        .trim_end()
        .to_string()
}

pub(crate) fn is_secret_by_name(key: &str) -> bool {
    let key_upper = key.to_uppercase();
    SECRET_KEYWORDS.iter().any(|kw| key_upper.contains(kw))
}
