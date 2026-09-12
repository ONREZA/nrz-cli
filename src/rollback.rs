use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::cli::RollbackArgs;
use crate::deployments::truncate_id;
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

    let result = client
        .rollback_deployment(&deployment_id)
        .await
        .context("failed to rollback deployment")?;
    anyhow::ensure!(
        result.rollback_from == deployment_id.parse::<uuid::Uuid>()?,
        "rollback response refers to another deployment"
    );
    let resp = RollbackResponse {
        id: result.id.to_string(),
        status: Some(result.status.to_string()),
        message: Some(result.message),
        rollback_from: Some(result.rollback_from.to_string()),
        rollback_to: Some(result.rollback_to.to_string()),
    };

    if json {
        output::json_output(&resp);
    } else {
        let msg = resp
            .message
            .unwrap_or_else(|| "Rollback initiated".to_string());
        output::success(false, msg, output::Phase::Rollback);

        if let (Some(from), Some(to)) = (&resp.rollback_from, &resp.rollback_to) {
            let from = output::terminal_line(from);
            let to = output::terminal_line(to);
            eprintln!(
                "  {} {} → {}",
                console::style("Rollback:").dim(),
                truncate_id(&from, 8),
                truncate_id(&to, 8),
            );
        }
    }

    Ok(())
}

pub(crate) async fn find_live_deployment(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<String> {
    let mut offset = 0u32;
    let mut candidate = None;
    let mut seen = std::collections::HashSet::new();
    loop {
        let page = client
            .project_deployments(project_id, 100, offset)
            .await
            .context("failed to fetch deployments")?;
        anyhow::ensure!(page.total >= 0, "invalid deployment count");
        let count = u32::try_from(page.deployments.len())?;
        let previous_count = seen.len();
        for deployment in page.deployments {
            if !seen.insert(deployment.id) {
                continue;
            }
            if deployment.is_active
                && deployment.status
                    == nrz_api::Project200ResponseProjectLatestDeploymentStatus::Live
            {
                anyhow::ensure!(
                    candidate.is_none(),
                    "multiple active deployments found; specify --deployment-id to select the environment to rollback"
                );
                candidate = Some(deployment.id.to_string());
            }
        }
        offset = offset
            .checked_add(count)
            .context("deployment pagination overflow")?;
        if i64::from(offset) >= page.total {
            break;
        }
        anyhow::ensure!(
            seen.len() > previous_count,
            "deployment listing changed while selecting rollback; specify --deployment-id"
        );
    }
    candidate.ok_or_else(|| anyhow::anyhow!("no active live deployment found to rollback"))
}
