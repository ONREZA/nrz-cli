use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::output;
use nrz::config;
use nrz::config::{EnvVisibility, ProjectConfig};

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
struct EnvVarEnvironment {
    id: String,
    name: String,
    #[serde(rename = "type")]
    env_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetEnvBody<'a> {
    key: &'a str,
    value: &'a str,
    is_secret: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<Vec<String>>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BulkEnvVar<'a> {
    key: &'a str,
    value: &'a str,
    is_secret: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<Vec<String>>,
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
    #[serde(default)]
    skipped: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    warnings: Option<Vec<String>>,
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
    env: &[String],
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;
    let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
    let targets = resolve_targets(env);

    match args.command {
        EnvCommand::List => list(&client, &project_id, &targets, json).await,
        EnvCommand::Set { key, value, secret } => {
            set(&client, &project_id, &key, &value, secret, &targets, json).await
        }
        EnvCommand::Delete { key } => delete(&client, &project_id, &key, json).await,
        EnvCommand::Pull { file } => pull(&client, &project_id, &file, json).await,
        EnvCommand::Push {
            file,
            overwrite,
            dry_run,
            secret,
            declared_only,
        } => {
            push(
                &client,
                &project_id,
                &file,
                overwrite,
                dry_run,
                secret,
                declared_only,
                &targets,
                json,
                config,
            )
            .await
        }
        EnvCommand::Validate => validate(&client, &project_id, json, config).await,
    }
}

/// Resolve `--env` values into normalized target environment types.
/// Only known environment types are kept; UUIDs and custom names are ignored.
fn resolve_targets(env: &[String]) -> Vec<String> {
    env.iter()
        .filter_map(|e| {
            match e.to_uppercase().as_str() {
                "PRODUCTION" | "PREVIEW" | "DEVELOPMENT" => Some(e.to_uppercase()),
                _ => None, // ignore UUIDs, custom names
            }
        })
        .collect()
}

async fn list(
    client: &ApiClient,
    project_id: &str,
    targets: &[String],
    json: bool,
) -> anyhow::Result<()> {
    let mut url = format!("/v1/projects/{}/env", project_id);
    if !targets.is_empty() {
        let params: Vec<String> = targets.iter().map(|t| format!("target={t}")).collect();
        url = format!("{}?{}", url, params.join("&"));
    }

    let resp: EnvListResponse = client
        .get(&url)
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
            console::style("Scope").bold(),
        );
        eprintln!("  {}", "-".repeat(70));

        for v in &resp.env_vars {
            let display_val = if v.is_secret {
                "*****".to_string()
            } else {
                v.value.as_deref().unwrap_or("-").to_string()
            };
            let scope = format_scope(v);
            eprintln!("  {:<30} {:<30} {}", v.key, display_val, scope);
        }
        eprintln!();
    }

    Ok(())
}

fn format_scope(v: &EnvVar) -> String {
    match v.scope_type.as_deref() {
        Some("ALL") | None => "ALL".to_string(),
        Some("SELECTED") if !v.environments.is_empty() => v
            .environments
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        Some(other) => other.to_string(),
    }
}

async fn set(
    client: &ApiClient,
    project_id: &str,
    key: &str,
    value: &str,
    secret: bool,
    targets: &[String],
    json: bool,
) -> anyhow::Result<()> {
    let target = if targets.is_empty() {
        None
    } else {
        Some(targets.to_vec())
    };
    let body = SetEnvBody {
        key,
        value,
        is_secret: secret,
        target,
    };

    let resp: SetEnvResponse = client
        .post(&format!("/v1/projects/{}/env", project_id), &body)
        .await
        .context("failed to set environment variable")?;

    if json {
        output::json_output(&resp);
    } else {
        let action = if resp.created { "Created" } else { "Updated" };
        output::success(false, format!("{action} {}", console::style(key).bold()));
    }

    Ok(())
}

async fn delete(client: &ApiClient, project_id: &str, key: &str, json: bool) -> anyhow::Result<()> {
    let resp: DeleteEnvResponse = client
        .delete(&format!("/v1/projects/{}/env/{}", project_id, key))
        .await
        .context("failed to delete environment variable")?;

    if json {
        output::json_output(&serde_json::json!({
            "key": key,
            "deleted": resp.deleted,
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

#[allow(clippy::too_many_arguments)]
async fn push(
    client: &ApiClient,
    project_id: &str,
    file: &str,
    overwrite: bool,
    dry_run: bool,
    force_secret: bool,
    declared_only: bool,
    targets: &[String],
    json: bool,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let content =
        std::fs::read_to_string(file).with_context(|| format!("failed to read {file}"))?;

    let result = parse_dotenv(&content);
    let mut parsed = result.vars;

    // Filter to declared-only vars if requested (flag or config)
    let strict = declared_only || config.env_strict();
    if strict {
        if config.env.declarations.is_empty() {
            anyhow::bail!(
                "strict mode is enabled but [env.declarations] is empty in onreza.toml. \
                 Declare variables or disable strict mode."
            );
        }
        let before = parsed.len();
        parsed.retain(|v| config.env.declarations.contains_key(&v.key));
        let filtered = before - parsed.len();
        if filtered > 0 {
            output::warn(
                json,
                format!("{filtered} variable(s) skipped (not declared in [env.declarations])"),
            );
        }
    }

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
                    let is_secret = resolve_sensitivity(&v.key, force_secret, config);
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
                let is_secret = resolve_sensitivity(&v.key, force_secret, config);
                let tag = if is_secret { " (secret)" } else { "" };
                eprintln!("    {} {}{}", console::style("+").green(), v.key, tag);
            }
            eprintln!();
        }
        return Ok(());
    }

    // Actual push: send all vars with overwrite flag, server handles skip/upsert
    let target = if targets.is_empty() {
        None
    } else {
        Some(targets.to_vec())
    };
    let variables: Vec<BulkEnvVar<'_>> = parsed
        .iter()
        .map(|v| BulkEnvVar {
            key: &v.key,
            value: &v.value,
            is_secret: resolve_sensitivity(&v.key, force_secret, config),
            target: target.clone(),
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
        let warnings: Vec<_> = resp
            .results
            .iter()
            .filter_map(|r| {
                r.warnings
                    .as_ref()
                    .map(|w| serde_json::json!({ "key": r.key, "warnings": w }))
            })
            .collect();
        let mut obj = serde_json::json!({
            "file": file,
            "uploaded": uploaded,
            "skipped": skipped,
            "failed": failed.len(),
            "errors": failed.iter().map(|r| serde_json::json!({
                "key": r.key,
                "error": r.error,
            })).collect::<Vec<_>>(),
        });
        if !warnings.is_empty() {
            obj["warnings"] = serde_json::json!(warnings);
        }
        output::json_output(&obj);
        if !failed.is_empty() {
            std::process::exit(1);
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
    for r in &resp.results {
        if let Some(warnings) = &r.warnings {
            for w in warnings {
                output::warn(false, format!("{}: {w}", r.key));
            }
        }
    }
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
    raw.split(" #").next().unwrap_or(raw).trim_end().to_string()
}

pub(crate) fn is_secret_by_name(key: &str) -> bool {
    let key_upper = key.to_uppercase();
    SECRET_KEYWORDS.iter().any(|kw| key_upper.contains(kw))
}

/// Resolve whether a variable should be marked as secret.
/// Priority: --secret flag > [env] config > heuristic by name.
pub(crate) fn resolve_sensitivity(key: &str, force_secret: bool, config: &ProjectConfig) -> bool {
    if force_secret {
        return true;
    }
    if let Some(vis) = config.env_visibility(key) {
        return vis == EnvVisibility::Sensitive;
    }
    is_secret_by_name(key)
}

// ── env validate ────────────────────────────────────────────

async fn validate(
    client: &ApiClient,
    project_id: &str,
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

    let resp: EnvListResponse = client
        .get(&format!("/v1/projects/{}/env", project_id))
        .await
        .context("failed to fetch environment variables")?;

    let platform_keys: std::collections::HashSet<&str> =
        resp.env_vars.iter().map(|v| v.key.as_str()).collect();

    let mut missing = Vec::new();
    let mut present = Vec::new();

    for (key, decl) in &config.env.declarations {
        if platform_keys.contains(key.as_str()) {
            present.push(key.as_str());
        } else if decl.required {
            missing.push((key.as_str(), decl.visibility));
        }
    }

    let declared_keys: std::collections::HashSet<&str> =
        config.env.declarations.keys().map(|k| k.as_str()).collect();
    let undeclared: Vec<&str> = resp
        .env_vars
        .iter()
        .map(|v| v.key.as_str())
        .filter(|k| !declared_keys.contains(k))
        .collect();

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
            "undeclared": undeclared,
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
                eprintln!("    {} {}{}", console::style("-").red(), key, tag);
            }
            eprintln!();
            eprintln!("  Set them with:");
            for (key, vis) in &missing {
                let flag = if *vis == EnvVisibility::Sensitive {
                    " --secret"
                } else {
                    ""
                };
                eprintln!(
                    "    {}",
                    console::style(format!("nrz env set {key} <value>{flag}")).dim()
                );
            }
            eprintln!();
            eprintln!(
                "  Or push from file: {}",
                console::style("nrz env push .env.local").dim()
            );
        }

        if !undeclared.is_empty() {
            eprintln!();
            output::warn(
                false,
                format!(
                    "{} variable(s) on platform not declared in onreza.toml: {}",
                    undeclared.len(),
                    undeclared.join(", ")
                ),
            );
        }
        eprintln!();
    }

    if !valid {
        anyhow::bail!("{} required environment variable(s) missing", missing.len());
    }

    Ok(())
}

/// Pre-flight check for deploy: verifies all required env vars are set on platform.
/// Returns Ok(()) if no [env] declarations exist or all required vars are present.
pub(crate) async fn validate_env_for_deploy(
    client: &ApiClient,
    project_id: &str,
    json: bool,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    if config.env.declarations.is_empty() {
        return Ok(());
    }

    output::status(json, "~", "Checking environment variables...");

    // Only fetch the keys we care about (declared required vars)
    let required_keys: Vec<&str> = config
        .env
        .declarations
        .iter()
        .filter(|(_, decl)| decl.required)
        .map(|(key, _)| key.as_str())
        .collect();

    if required_keys.is_empty() {
        return Ok(());
    }

    let keys_param = required_keys.join(",");
    let resp: EnvListResponse = client
        .get(&format!(
            "/v1/projects/{}/env?keys={}",
            project_id, keys_param
        ))
        .await
        .context("failed to fetch environment variables")?;

    let platform_keys: std::collections::HashSet<&str> =
        resp.env_vars.iter().map(|v| v.key.as_str()).collect();

    let mut missing: Vec<(&str, EnvVisibility)> = config
        .env
        .declarations
        .iter()
        .filter(|(key, decl)| decl.required && !platform_keys.contains(key.as_str()))
        .map(|(key, decl)| (key.as_str(), decl.visibility))
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    missing.sort_by_key(|(k, _)| *k);

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
            "error": "missing_env_vars",
            "missing": missing_json,
        }));
        std::process::exit(1);
    } else {
        eprintln!();
        eprintln!(
            "  {} Missing required environment variables:\n",
            console::style("✗").red().bold()
        );
        for (key, vis) in &missing {
            let tag = match vis {
                EnvVisibility::Sensitive => " (sensitive)",
                EnvVisibility::Plain => "",
            };
            eprintln!("    {} {}{}", console::style("-").red(), key, tag);
        }
        eprintln!();
        eprintln!("  Set them with:");
        for (key, vis) in &missing {
            let flag = match vis {
                EnvVisibility::Sensitive => " --secret",
                EnvVisibility::Plain => "",
            };
            eprintln!(
                "    {}",
                console::style(format!("nrz env set {key} <value>{flag}")).dim()
            );
        }
        eprintln!();
        eprintln!(
            "  Or push from file: {}",
            console::style("nrz env push .env.local").dim()
        );
        eprintln!();
        eprintln!(
            "  Skip this check with: {}",
            console::style("nrz deploy --skip-env-check").dim()
        );
    }

    anyhow::bail!(
        "{} required environment variable(s) missing. Set them or use --skip-env-check",
        missing.len()
    );
}
