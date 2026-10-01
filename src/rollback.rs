use std::path::Path;

use anyhow::{Context, ensure};
use serde::Serialize;

use crate::api::ApiClient;
use crate::auth;
use crate::cli::RollbackArgs;
use crate::deployments::truncate_id;
use crate::execution_context;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivationOutput {
    environment_id: String,
    operation_id: String,
    release_id: String,
    desired_generation: String,
    status: String,
}

pub async fn run(
    args: RollbackArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    ensure!(
        args.list || args.release_id.is_some(),
        "select a retained release with --release-id, or use --list"
    );
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;
    let context = execution_context::resolve_for_mutation(
        &client,
        &project_id,
        Path::new(&args.dir),
        args.environment.as_deref(),
        None,
    )
    .await
    .context("failed to resolve rollback environment")?;
    let environment_id = context.environment_id;

    if args.list {
        let serving = client.environment_serving(&environment_id).await?;
        let page = client.environment_releases(&environment_id).await?;
        if json {
            output::json_output(&serde_json::json!({
                "environmentId": environment_id,
                "observedReleaseId": serving.observed.release_id,
                "desiredGeneration": serving.desired.generation,
                "releases": page.releases,
                "nextCursor": page.next_cursor,
            }));
        } else {
            eprintln!("Retained releases for {}:", context.environment_name);
            for release in &page.releases {
                let marker = if Some(release.release_id) == serving.observed.release_id {
                    " (observed active)"
                } else {
                    ""
                };
                eprintln!(
                    "  {}  {}{}",
                    release.release_id,
                    release.created_at.to_rfc3339(),
                    marker,
                );
            }
            if page.next_cursor.is_some() {
                eprintln!("  More releases are available through the Environment releases API.");
            }
        }
        return Ok(());
    }

    let release_id = args.release_id.context("missing --release-id")?;
    let activation = activate_release(&client, &environment_id, &release_id).await?;
    let output_row = ActivationOutput {
        environment_id,
        operation_id: activation.operation_id.to_string(),
        release_id: activation.release_id.to_string(),
        desired_generation: activation.desired_generation,
        status: activation.status,
    };
    if json {
        output::json_output(&output_row);
    } else {
        output::success(
            false,
            format!("Release activation requested ({})", output_row.status),
            output::Phase::Rollback,
        );
        eprintln!(
            "  {} {} → {}",
            console::style("Activation:").dim(),
            truncate_id(&output_row.operation_id, 8),
            truncate_id(&output_row.release_id, 8),
        );
    }
    Ok(())
}

pub(crate) async fn activate_release(
    client: &ApiClient,
    environment_id: &str,
    release_id: &str,
) -> anyhow::Result<nrz_api::ActivateRelease202Response> {
    let expected_environment: uuid::Uuid =
        environment_id.parse().context("invalid environment ID")?;
    let requested_release: uuid::Uuid = release_id.parse().context("invalid release ID")?;
    let serving = client.environment_serving(environment_id).await?;
    ensure!(
        serving.environment_id == expected_environment,
        "serving response refers to another environment"
    );
    let result = client
        .activate_environment_release(
            environment_id,
            release_id,
            serving.desired.generation,
            uuid::Uuid::now_v7().to_string(),
        )
        .await
        .context("failed to request environment release activation")?;
    ensure!(
        result.release_id == requested_release,
        "activation response refers to another release"
    );
    Ok(result)
}
