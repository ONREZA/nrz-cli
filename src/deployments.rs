use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::{ApiClient, path_segment};
use crate::auth;
use crate::cli::DeploymentsArgs;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentsResponse {
    deployments: Vec<Deployment>,
    total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub id: String,
    pub status: DeploymentStatus,
    #[serde(default)]
    pub is_preview: Option<bool>,
    #[serde(default)]
    pub is_rollback: Option<bool>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub commit_sha: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub deployed_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Pending,
    Queued,
    Building,
    Uploading,
    Ingesting,
    Skipped,
    SmokeTesting,
    Live,
    Stopped,
    Failed,
    Deploying,
    Cancelled,
    Unknown,
}

impl<'de> Deserialize<'de> for DeploymentStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.to_ascii_lowercase().as_str() {
            "pending" => Self::Pending,
            "queued" => Self::Queued,
            "building" => Self::Building,
            "uploading" => Self::Uploading,
            "ingesting" => Self::Ingesting,
            "skipped" => Self::Skipped,
            "smoke_testing" => Self::SmokeTesting,
            "live" => Self::Live,
            "stopped" => Self::Stopped,
            "failed" => Self::Failed,
            "deploying" => Self::Deploying,
            "cancelled" => Self::Cancelled,
            _ => Self::Unknown,
        })
    }
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Queued => write!(f, "queued"),
            Self::Building => write!(f, "building"),
            Self::Uploading => write!(f, "uploading"),
            Self::Ingesting => write!(f, "ingesting"),
            Self::Skipped => write!(f, "skipped"),
            Self::SmokeTesting => write!(f, "smoke_testing"),
            Self::Live => write!(f, "live"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
            Self::Deploying => write!(f, "deploying"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

pub async fn run(
    args: DeploymentsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;

    let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;

    let resp: DeploymentsResponse = client
        .get(&format!(
            "/v1/deployments/project/{}?limit={}",
            path_segment(&project_id),
            args.limit
        ))
        .await
        .context("failed to fetch deployments")?;

    if json {
        output::json_output(&resp);
    } else {
        if resp.deployments.is_empty() {
            eprintln!("  No deployments found.");
            return Ok(());
        }

        eprintln!();
        eprintln!(
            "  {:<12} {:<12} {:<15} {:<35} {}",
            console::style("ID").bold(),
            console::style("Status").bold(),
            console::style("Branch").bold(),
            console::style("URL").bold(),
            console::style("Created").bold(),
        );
        eprintln!("  {}", "-".repeat(90));

        for d in &resp.deployments {
            let id = output::terminal_line(&d.id);
            let short_id = truncate_id(&id, 8);
            let branch = output::terminal_line(d.branch.as_deref().unwrap_or("-"));
            let url = output::terminal_line(d.url.as_deref().unwrap_or("-"));
            let created = output::terminal_line(d.created_at.as_deref().unwrap_or("-"));
            eprintln!(
                "  {:<12} {:<12} {:<15} {:<35} {}",
                short_id, d.status, branch, url, created
            );
        }

        eprintln!();
        eprintln!(
            "  {} {} deployment(s)",
            console::style("Total:").dim(),
            resp.total,
        );
        if let Some(url) = first_preview_url(&resp.deployments) {
            eprintln!();
            crate::preview::print_preview_access_hint(&project_id, Some(url));
        }
    }

    Ok(())
}

pub(crate) fn first_preview_url(deployments: &[Deployment]) -> Option<&str> {
    deployments
        .iter()
        .find(|d| d.is_preview == Some(true) && d.url.is_some())
        .and_then(|d| d.url.as_deref())
}

pub fn truncate_id(s: &str, max: usize) -> &str {
    s.get(..max).unwrap_or(s)
}
