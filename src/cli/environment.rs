use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::output;

const DEFAULT_ENVIRONMENT_TARGET: &str = "production";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvironmentsResponse {
    pub(crate) environments: Vec<ProjectEnvironment>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectEnvironment {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) env_type: String,
    pub(crate) name: String,
}

pub(crate) async fn resolve_environment_id(
    client: &ApiClient,
    project_id: &str,
    target: Option<&str>,
) -> anyhow::Result<String> {
    let resp: EnvironmentsResponse = client
        .get(&format!("/v1/environments/{project_id}"))
        .await
        .context("failed to fetch environments")?;
    resolve_environment_from_list(&resp.environments, target).map(|environment| environment.id)
}

pub(crate) fn resolve_environment_from_list(
    environments: &[ProjectEnvironment],
    target: Option<&str>,
) -> anyhow::Result<ProjectEnvironment> {
    let target = target
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_ENVIRONMENT_TARGET);

    if let Some(environment) = environments
        .iter()
        .find(|environment| environment.id == target)
    {
        return Ok(environment.clone());
    }

    let normalized_type = normalize_environment_type_target(target);
    let matches: Vec<&ProjectEnvironment> = environments
        .iter()
        .filter(|environment| {
            normalized_type
                .is_some_and(|env_type| environment.env_type.eq_ignore_ascii_case(env_type))
                || environment.name.eq_ignore_ascii_case(target)
        })
        .collect();

    match matches.as_slice() {
        [environment] => Ok((*environment).clone()),
        [] => Err(output::coded_error(
            "ENVIRONMENT_NOT_FOUND",
            format!("environment '{target}' not found for this project"),
        )),
        _ => Err(output::coded_error(
            "ENVIRONMENT_AMBIGUOUS",
            format!("environment '{target}' is ambiguous; pass the environment ID instead"),
        )),
    }
}

fn normalize_environment_type_target(target: &str) -> Option<&'static str> {
    match target.to_ascii_uppercase().as_str() {
        "PRODUCTION" | "PROD" => Some("PRODUCTION"),
        "PREVIEW" => Some("PREVIEW"),
        "DEVELOPMENT" | "DEV" => Some("DEVELOPMENT"),
        _ => None,
    }
}
