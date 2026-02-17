use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::RollbackArgs;
use crate::deployments::{Deployment, DeploymentStatus, truncate_id};
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RollbackResponse {
    id: String,
    #[serde(default)]
    status: Option<String>,
    message: Option<String>,
    rollback_from: Option<String>,
    rollback_to: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeploymentsResponse {
    deployments: Vec<Deployment>,
}

pub async fn run(
    args: RollbackArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;

    let deployment_id = if let Some(id) = &args.deployment_id {
        id.clone()
    } else {
        let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
        find_live_deployment(&client, &project_id).await?
    };

    let resp: RollbackResponse = client
        .post_empty(&format!("/v1/deployments/{}/rollback", deployment_id))
        .await
        .context("failed to rollback deployment")?;

    if json {
        output::json_output(&resp);
    } else {
        let msg = resp
            .message
            .unwrap_or_else(|| "Rollback initiated".to_string());
        output::success(false, msg);

        if let (Some(from), Some(to)) = (&resp.rollback_from, &resp.rollback_to) {
            eprintln!(
                "  {} {} → {}",
                console::style("Rollback:").dim(),
                truncate_id(from, 8),
                truncate_id(to, 8),
            );
        }
    }

    Ok(())
}

async fn find_live_deployment(client: &ApiClient, project_id: &str) -> anyhow::Result<String> {
    let resp: DeploymentsResponse = client
        .get(&format!("/v1/deployments/project/{}?limit=10", project_id))
        .await
        .context("failed to fetch deployments")?;

    resp.deployments
        .iter()
        .find(|d| d.status == DeploymentStatus::Live)
        .map(|d| d.id.clone())
        .ok_or_else(|| anyhow::anyhow!("no live deployment found to rollback"))
}
