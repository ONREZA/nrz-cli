//! Best-effort sync of detection results to the platform API.

use crate::detect::types::{ComputeType, DetectionResult};
use serde::Serialize;

use crate::api::ApiClient;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DetectionSyncBody<'a> {
    framework: &'a str,
    framework_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    framework_version: Option<&'a str>,
    suggested_compute: &'a ComputeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_manager: Option<&'a str>,
    source: &'a str,
}

/// Send detection result to the platform API (best-effort, errors are silently ignored).
///
/// Called during `init --create`, `deploy`, etc. to enrich the project record
/// in the platform database with locally detected framework info.
pub async fn sync_detection_to_api(client: &ApiClient, project_id: &str, result: &DetectionResult) {
    let pm = result
        .metadata
        .package_manager
        .as_ref()
        .map(|p| p.pm_type.as_str());

    let body = DetectionSyncBody {
        framework: &result.framework,
        framework_name: &result.name,
        framework_version: result.version.as_deref(),
        suggested_compute: &result.suggested_compute,
        package_manager: pm,
        source: "cli",
    };

    let path = format!("/v1/projects/{project_id}/detection");
    // Best-effort: log warning on failure, don't fail the overall command
    let resp: Result<serde_json::Value, _> = client.post(&path, &body).await;
    if let Err(e) = resp {
        eprintln!(
            "  {} failed to sync detection to API: {e}",
            console::style("warn").yellow()
        );
    }
}
