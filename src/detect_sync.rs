//! Best-effort sync of detection results to the platform API.

use crate::api::ApiClient;
use crate::detect::types::{ComputeType, DetectionResult, PackageManagerType};
use nrz_api::{
    DetectionRequestBody, Project200ResponseProjectDetectedComputeType,
    ProjectRequestBodyPackageManager,
};

/// Send detection result to the platform API (best-effort, errors are silently ignored).
///
/// Called during `init --create`, `deploy`, etc. to enrich the project record
/// in the platform database with locally detected framework info.
pub async fn sync_detection_to_api(client: &ApiClient, project_id: &str, result: &DetectionResult) {
    let pm = result
        .metadata
        .package_manager
        .as_ref()
        .map(|p| detection_package_manager_to_platform(p.pm_type));

    let body = DetectionRequestBody {
        framework: result.framework.clone(),
        framework_name: result.name.clone(),
        framework_version: result.version.clone(),
        suggested_compute: match result.suggested_compute {
            ComputeType::Static => Project200ResponseProjectDetectedComputeType::Static,
            ComputeType::Process => Project200ResponseProjectDetectedComputeType::Process,
        },
        package_manager: pm,
        source: "cli".into(),
    };

    let resp = client.sync_detection(project_id, body).await;
    if let Err(e) = resp {
        tracing::warn!("failed to sync detection to API: {e}");
    }
}

pub(crate) fn detection_package_manager_to_platform(
    pm: PackageManagerType,
) -> ProjectRequestBodyPackageManager {
    match pm {
        PackageManagerType::Npm => ProjectRequestBodyPackageManager::Npm,
        PackageManagerType::Yarn => ProjectRequestBodyPackageManager::Yarn,
        PackageManagerType::Pnpm => ProjectRequestBodyPackageManager::Pnpm,
        PackageManagerType::Bun => ProjectRequestBodyPackageManager::Bun,
        PackageManagerType::Pip => ProjectRequestBodyPackageManager::Pip,
    }
}
