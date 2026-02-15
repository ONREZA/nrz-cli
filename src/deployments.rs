use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::DeploymentsArgs;
use crate::link::project_ref;
use crate::output;

#[derive(Debug, Deserialize, Serialize)]
struct DeploymentsResponse {
    deployments: Vec<Deployment>,
    total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Deployment {
    pub id: String,
    pub status: DeploymentStatus,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStatus {
    Live,
    Failed,
    Deploying,
    Building,
    Queued,
    Cancelled,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Live => write!(f, "live"),
            Self::Failed => write!(f, "failed"),
            Self::Deploying => write!(f, "deploying"),
            Self::Building => write!(f, "building"),
            Self::Queued => write!(f, "queued"),
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
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;

    let project_id = project_ref::resolve_project_id(args.project_id.as_deref())?;

    let resp: DeploymentsResponse = client
        .get(&format!(
            "/v1/deployments/project/{}?limit={}",
            project_id, args.limit
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
            let short_id = truncate_id(&d.id, 8);
            let branch = d.branch.as_deref().unwrap_or("-");
            let url = d.url.as_deref().unwrap_or("-");
            let created = d.created_at.as_deref().unwrap_or("-");
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
    }

    Ok(())
}

pub fn truncate_id(s: &str, max: usize) -> &str {
    s.get(..max).unwrap_or(s)
}
