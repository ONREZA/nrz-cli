//! Report the policy of the returned URL, independently of deployment environment.
use anyhow::Context;
use nrz_api::Deployment200ResponseDeploymentUrlAliasType;

use crate::api::ApiClient;

pub(super) async fn read(
    client: &ApiClient,
    deployment_id: &str,
    url: &str,
) -> anyhow::Result<bool> {
    let deployment = client.deployment(deployment_id).await?;
    let returned = deployment
        .deployment_urls
        .iter()
        .find(|candidate| candidate.full_url.trim_end_matches('/') == url.trim_end_matches('/'))
        .context("returned URL is not present in deployment routes")?;
    Ok(protection(
        &returned.alias_type,
        deployment.project.preview_protection_enabled,
    ))
}

pub(super) fn protection(
    alias: &Deployment200ResponseDeploymentUrlAliasType,
    preview_protection_enabled: bool,
) -> bool {
    match alias {
        Deployment200ResponseDeploymentUrlAliasType::ProductionAlias => false,
        Deployment200ResponseDeploymentUrlAliasType::BranchAlias
        | Deployment200ResponseDeploymentUrlAliasType::UniqueUrl => preview_protection_enabled,
    }
}
