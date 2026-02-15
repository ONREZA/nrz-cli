use anyhow::Context;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::LogsArgs;
use crate::link::project_ref;
use crate::output;

#[derive(Debug, Deserialize, Serialize)]
struct LogsResponse {
    logs: Vec<LogEntry>,
    total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct LogEntry {
    timestamp: Option<String>,
    level: Option<String>,
    message: String,
}

pub async fn run(args: LogsArgs, json: bool, token: Option<&str>) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token)
        .ok_or_else(|| anyhow::anyhow!("not logged in. Run `nrz login` first."))?;

    let client = ApiClient::authenticated(&tok)?;

    let project_id = project_ref::resolve_project_id(args.project_id.as_deref())?;

    let mut query = format!("limit={}", args.limit);
    if let Some(search) = &args.search {
        query.push_str(&format!(
            "&search={}",
            utf8_percent_encode(search, NON_ALPHANUMERIC)
        ));
    }
    if let Some(did) = &args.deployment_id {
        query.push_str(&format!(
            "&deploymentId={}",
            utf8_percent_encode(did, NON_ALPHANUMERIC)
        ));
    }

    let resp: LogsResponse = client
        .get(&format!(
            "/v1/projects/{}/runtime-logs?{}",
            project_id, query
        ))
        .await
        .context("failed to fetch logs")?;

    if json {
        output::json_output(&resp);
    } else {
        if resp.logs.is_empty() {
            eprintln!("  No logs found.");
            return Ok(());
        }

        for entry in &resp.logs {
            let ts = entry.timestamp.as_deref().unwrap_or("-");
            let level = entry.level.as_deref().unwrap_or("info");
            eprintln!("[{}] [{}] {}", ts, level, entry.message);
        }
    }

    Ok(())
}
