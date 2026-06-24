use crate::api::ApiClient;
use crate::auth;
use crate::cli::preview::{PreviewArgs, PreviewCommand};
use crate::output;
use anyhow::Context;
use nrz::config;
use nrz::config::ProjectConfig;
use serde::{Deserialize, Serialize};

#[cfg(test)]
const BYPASS_HEADER_NAME: &str = "X-ONREZA-Protection-Bypass";
const DEFAULT_TTL_DISPLAY: &str = "1h";
const MIN_TTL_SECONDS: u64 = 60;
const MAX_TTL_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatePreviewAccessBody {
    note: String,
    ttl_seconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePreviewAccessResponse {
    access: ServerPreviewAccess,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerPreviewAccess {
    project_id: String,
    secret_id: String,
    note: String,
    expires_at: String,
    ttl_seconds: u64,
    header: ServerPreviewAccessHeader,
    query: ServerPreviewAccessQuery,
}

#[derive(Debug, Deserialize)]
struct ServerPreviewAccessHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct ServerPreviewAccessQuery {
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct DeleteBypassSecretResponse {
    success: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewAccessOutput {
    pub(crate) project_id: String,
    pub(crate) secret_id: String,
    pub(crate) note: String,
    pub(crate) header_name: String,
    pub(crate) header_value: String,
    pub(crate) headers: PreviewAccessHeaders,
    pub(crate) query_name: String,
    pub(crate) query_value: String,
    pub(crate) query: PreviewAccessQuery,
    pub(crate) expires_at: String,
    pub(crate) ttl_seconds: u64,
    pub(crate) ttl_enforced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) browser_url: Option<String>,
    pub(crate) curl_command: String,
    pub(crate) revoke_command: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PreviewAccessHeaders {
    pub(crate) x_onreza_protection_bypass: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PreviewAccessQuery {
    pub(crate) name: String,
    pub(crate) value: String,
}

pub async fn run(
    args: PreviewArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;

    match args.command {
        PreviewCommand::Access(args) => {
            let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
            let ttl_seconds = parse_ttl_seconds(&args.ttl)?;
            access(&client, project_id, args.note, args.url, ttl_seconds, json).await
        }
        PreviewCommand::Revoke(args) => {
            let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
            revoke(&client, &project_id, &args.secret_id, json).await
        }
    }
}

async fn access(
    client: &ApiClient,
    project_id: String,
    note: String,
    url: Option<String>,
    ttl_seconds: u64,
    json: bool,
) -> anyhow::Result<()> {
    let resp: CreatePreviewAccessResponse = client
        .post(
            &format!("/v1/preview-access/{project_id}"),
            &CreatePreviewAccessBody { note, ttl_seconds },
        )
        .await
        .context("failed to create preview access")?;
    let output = build_preview_access_output(resp.access, url);

    if json {
        output::json_output(&output);
    } else {
        report_access_human(&output);
    }

    Ok(())
}

async fn revoke(
    client: &ApiClient,
    project_id: &str,
    secret_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    client
        .delete_empty(&format!("/v1/preview-access/{project_id}/{secret_id}"))
        .await
        .context("failed to revoke preview access")?;
    let resp = DeleteBypassSecretResponse { success: true };

    if json {
        output::json_output(&resp);
    } else {
        eprintln!(
            "  {} Revoked preview access secret {}",
            console::style("✓").green().bold(),
            console::style(secret_id).bold(),
        );
    }

    Ok(())
}

fn build_preview_access_output(
    access: ServerPreviewAccess,
    url: Option<String>,
) -> PreviewAccessOutput {
    let curl_command =
        build_curl_command(url.as_deref(), &access.header.name, &access.header.value);
    let browser_url = url
        .as_deref()
        .and_then(|url| build_browser_url(url, &access.query.name, &access.query.value));
    let revoke_command = preview_revoke_command(&access.project_id, &access.secret_id);
    PreviewAccessOutput {
        project_id: access.project_id,
        secret_id: access.secret_id,
        note: access.note,
        header_name: access.header.name,
        header_value: access.header.value.clone(),
        headers: PreviewAccessHeaders {
            x_onreza_protection_bypass: access.header.value,
        },
        query_name: access.query.name.clone(),
        query_value: access.query.value.clone(),
        query: PreviewAccessQuery {
            name: access.query.name,
            value: access.query.value,
        },
        expires_at: access.expires_at,
        ttl_seconds: access.ttl_seconds,
        ttl_enforced: true,
        url,
        browser_url,
        curl_command,
        revoke_command,
    }
}

pub(crate) fn preview_access_hint(project_id: &str, url: Option<&str>) -> String {
    let mut command = format!("nrz preview access --project-id {}", shell_word(project_id));
    if let Some(url) = url.filter(|value| !value.trim().is_empty()) {
        command.push_str(" --url ");
        command.push_str(&shell_word(url));
    }
    command
}

pub(crate) fn print_preview_access_hint(project_id: &str, url: Option<&str>) {
    eprintln!(
        "  {} Protected preview URL. For agents/curl access (default {DEFAULT_TTL_DISPLAY}):",
        console::style("i").cyan().bold(),
    );
    eprintln!("    {}", preview_access_hint(project_id, url));
    eprintln!();
}

fn preview_revoke_command(project_id: &str, secret_id: &str) -> String {
    format!(
        "nrz preview revoke --project-id {} --secret-id {}",
        shell_word(project_id),
        shell_word(secret_id)
    )
}

fn build_curl_command(url: Option<&str>, header_name: &str, token: &str) -> String {
    let header = format!("{header_name}: {token}");
    match url.filter(|value| !value.trim().is_empty()) {
        Some(url) => format!("curl -H {} {}", shell_word(&header), shell_word(url)),
        None => format!("curl -H {} <preview-url>", shell_word(&header)),
    }
}

fn build_browser_url(url: &str, query_name: &str, query_value: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    parsed
        .query_pairs_mut()
        .append_pair(query_name, query_value);
    Some(parsed.to_string())
}

fn report_access_human(output: &PreviewAccessOutput) {
    eprintln!(
        "  {} Preview access created",
        console::style("✓").green().bold(),
    );
    eprintln!(
        "  {} {}",
        console::style("Project:").dim(),
        output.project_id
    );
    eprintln!(
        "  {} {}",
        console::style("Secret ID:").dim(),
        output.secret_id
    );
    eprintln!(
        "  {} {}",
        console::style("Expires:").dim(),
        output.expires_at
    );
    eprintln!();
    eprintln!("  Header:");
    eprintln!("    {}: {}", output.header_name, output.header_value);
    eprintln!();
    eprintln!("  Browser query:");
    eprintln!("    {}={}", output.query_name, output.query_value);
    eprintln!();
    eprintln!("  Curl:");
    eprintln!("    {}", output.curl_command);
    if let Some(browser_url) = &output.browser_url {
        eprintln!();
        eprintln!("  Browser URL (contains the secret):");
        eprintln!("    {browser_url}");
    }
    eprintln!();
    eprintln!("  Revoke:");
    eprintln!("    {}", output.revoke_command);
}

fn parse_ttl_seconds(value: &str) -> anyhow::Result<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("--ttl must not be empty");
    }

    let (number, multiplier) = match trimmed.as_bytes().last().copied() {
        Some(b's' | b'S') => (&trimmed[..trimmed.len() - 1], 1),
        Some(b'm' | b'M') => (&trimmed[..trimmed.len() - 1], 60),
        Some(b'h' | b'H') => (&trimmed[..trimmed.len() - 1], 60 * 60),
        Some(b'd' | b'D') => (&trimmed[..trimmed.len() - 1], 24 * 60 * 60),
        Some(_) => (trimmed, 1),
        None => unreachable!("empty value returned earlier"),
    };
    let amount: u64 = number
        .parse()
        .with_context(|| format!("invalid --ttl value: {value}"))?;
    let seconds = amount
        .checked_mul(multiplier)
        .with_context(|| format!("--ttl value is too large: {value}"))?;

    if !(MIN_TTL_SECONDS..=MAX_TTL_SECONDS).contains(&seconds) {
        anyhow::bail!("--ttl must be between 60s and 24h");
    }

    Ok(seconds)
}

fn shell_word(value: &str) -> String {
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access() -> ServerPreviewAccess {
        ServerPreviewAccess {
            project_id: "project-1".to_string(),
            secret_id: "secret-1".to_string(),
            note: "test".to_string(),
            expires_at: "2026-06-24T17:00:00.000Z".to_string(),
            ttl_seconds: 3600,
            header: ServerPreviewAccessHeader {
                name: BYPASS_HEADER_NAME.to_string(),
                value: "token-value".to_string(),
            },
            query: ServerPreviewAccessQuery {
                name: "_bypass".to_string(),
                value: "token-value".to_string(),
            },
        }
    }

    #[test]
    fn preview_access_output_builds_agent_and_revoke_snippets() {
        let output = build_preview_access_output(
            access(),
            Some("https://preview.onreza.app/docs?x=1".to_string()),
        );

        assert_eq!(output.header_name, BYPASS_HEADER_NAME);
        assert_eq!(output.header_value, "token-value");
        assert_eq!(output.query_name, "_bypass");
        assert_eq!(output.query_value, "token-value");
        assert_eq!(output.expires_at, "2026-06-24T17:00:00.000Z");
        assert_eq!(output.ttl_seconds, 3600);
        assert!(output.ttl_enforced);
        assert_eq!(
            output.browser_url.as_deref(),
            Some("https://preview.onreza.app/docs?x=1&_bypass=token-value")
        );
        assert_eq!(
            output.curl_command,
            "curl -H 'X-ONREZA-Protection-Bypass: token-value' 'https://preview.onreza.app/docs?x=1'"
        );
        assert_eq!(
            output.revoke_command,
            "nrz preview revoke --project-id project-1 --secret-id secret-1"
        );
    }

    #[test]
    fn preview_access_hint_includes_url_only_when_available() {
        assert_eq!(
            preview_access_hint("project-1", Some("https://preview.onreza.app")),
            "nrz preview access --project-id project-1 --url https://preview.onreza.app"
        );
        assert_eq!(
            preview_access_hint("project-1", None),
            "nrz preview access --project-id project-1"
        );
    }

    #[test]
    fn parse_ttl_accepts_common_units() {
        assert_eq!(parse_ttl_seconds("60").unwrap(), 60);
        assert_eq!(parse_ttl_seconds("15m").unwrap(), 900);
        assert_eq!(parse_ttl_seconds("1h").unwrap(), 3600);
        assert_eq!(parse_ttl_seconds("1d").unwrap(), 86_400);
    }
}
