//! Report the protection policy for the deployment's Environment address.
use anyhow::Context;

use crate::api::ApiClient;

pub(super) async fn read(
    client: &ApiClient,
    deployment_id: &str,
    url: &str,
) -> anyhow::Result<bool> {
    let deployment = client.deployment(deployment_id).await?;
    let returned = deployment
        .url
        .as_deref()
        .context("environment address is unavailable")?;
    anyhow::ensure!(
        returned.trim_end_matches('/') == url.trim_end_matches('/'),
        "returned URL does not match the deployment environment address"
    );
    Ok(protection(
        deployment.is_preview,
        deployment.project.preview_protection_enabled,
    ))
}

pub(super) fn protection(is_preview: bool, preview_protection_enabled: bool) -> bool {
    is_preview && preview_protection_enabled
}
