use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::{ApiClient, path_segment, query_value};
use crate::auth;
use crate::cli::LogsArgs;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogsResponse {
    pub(crate) entries: Vec<serde_json::Value>,
    pagination: LogsPagination,
    filters: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogsPagination {
    limit: u32,
    offset: u32,
    has_more: bool,
}

pub async fn run(
    args: LogsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;

    let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;

    let mut query = format!("limit={}", args.limit);
    if let Some(search) = &args.search {
        query.push_str(&format!("&search={}", query_value(search)));
    }
    if let Some(did) = &args.deployment_id {
        query.push_str(&format!("&deploymentId={}", query_value(did)));
    }

    let resp: LogsResponse = client
        .get(&format!(
            "/v1/projects/{}/runtime-logs?{}",
            path_segment(&project_id),
            query
        ))
        .await
        .context("failed to fetch logs")?;

    if json {
        output::json_output(&resp);
    } else {
        if resp.entries.is_empty() {
            eprintln!("  No logs found.");
            return Ok(());
        }

        for entry in &resp.entries {
            eprintln!("{}", format_log_entry(entry));
        }
    }

    Ok(())
}

pub(crate) fn format_log_entry(entry: &serde_json::Value) -> String {
    let ts = string_field(entry, "timestamp").unwrap_or("-");

    let rendered = if let Some(message) = string_field(entry, "message") {
        let level = string_field(entry, "functionLogLevel")
            .or_else(|| string_field(entry, "level"))
            .or_else(|| string_field(entry, "source"))
            .unwrap_or("info");
        format!("[{ts}] [{level}] {message}")
    } else {
        let method = string_field(entry, "method").unwrap_or("-");
        let path = string_field(entry, "path").unwrap_or("-");
        let status = number_field(entry, "status")
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        let duration = duration_field(entry).unwrap_or_default();
        let function = string_field(entry, "functionName")
            .map(|value| format!(" function={value}"))
            .unwrap_or_default();

        format!("[{ts}] [{status}] {method} {path}{duration}{function}")
    };
    output::terminal_line(&rendered)
}

fn string_field<'a>(entry: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    entry.get(key)?.as_str()
}

fn number_field(entry: &serde_json::Value, key: &str) -> Option<i64> {
    entry.get(key)?.as_i64()
}

fn duration_field(entry: &serde_json::Value) -> Option<String> {
    let value = entry.get("durationMs")?.as_f64()?;
    Some(if value.fract() == 0.0 {
        format!(" {:.0}ms", value)
    } else {
        format!(" {:.1}ms", value)
    })
}
