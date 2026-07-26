use anyhow::Context;
use nrz::config::ProjectBuildSettings;

use crate::api::{ApiClient, path_segment};

pub(crate) enum ProjectSettingsFetch {
    Applied(ProjectBuildSettings),
    TransientFailure { message: String },
}

pub(crate) async fn fetch(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<ProjectBuildSettings> {
    client
        .get(&format!("/v1/projects/{}", path_segment(project_id)))
        .await
        .context("failed to fetch project settings")
}

pub(crate) async fn fetch_for_effective_config(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<ProjectSettingsFetch> {
    match fetch(client, project_id).await {
        Ok(settings) => Ok(ProjectSettingsFetch::Applied(settings)),
        Err(error) if is_client_error(&error) => Err(error.context(format!(
            "failed to fetch settings for project '{project_id}'. \
             Verify the project ID is correct"
        ))),
        Err(error) => Ok(ProjectSettingsFetch::TransientFailure {
            message: error.to_string(),
        }),
    }
}

fn is_client_error(error: &anyhow::Error) -> bool {
    format!("{error:#}").contains("API error (4")
}
