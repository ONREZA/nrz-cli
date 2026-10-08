use nrz::config::{self, ProjectConfig};

use crate::{api::ApiClient, auth};

pub(crate) fn project_client(
    token: Option<&str>,
    workspace: Option<&str>,
    project_id: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<(ApiClient, String)> {
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    let project_id = config::resolve_project_id(project_id, config)?;
    Ok((client, project_id))
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
