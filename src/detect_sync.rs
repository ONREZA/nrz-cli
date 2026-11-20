//! Best-effort sync of detection results to the platform API.

use crate::api::ApiClient;
use crate::detect::types::{ComputeType, DetectionResult, PackageManagerType};
use nrz_api::{
    DetectionRequestBody, ServiceRequestBodyDetectedComputeType, ServiceRequestBodyPackageManager,
};

/// Send detection result to the platform API (best-effort, errors are silently ignored).
///
/// Called during `init --create`, `deploy`, etc. to enrich the project record
/// in the platform database with locally detected framework info.
pub async fn sync_detection_to_api(
    client: &ApiClient,
    project_id: &str,
    service_id: Option<&str>,
    result: &DetectionResult,
) {
    let pm = result
        .metadata
        .package_manager
        .as_ref()
        .map(|p| detection_package_manager_to_platform(p.pm_type));

    let service_id = match service_id.map(str::parse).transpose() {
        Ok(id) => id,
        Err(error) => {
            tracing::warn!("failed to sync detection: invalid Service ID: {error}");
            return;
        }
    };
    let body = DetectionRequestBody {
        service_id,
        additional_properties: Default::default(),
        framework: result.framework.clone(),
        framework_name: result.name.clone(),
        framework_version: result.version.clone(),
        suggested_compute: match result.suggested_compute {
            ComputeType::Static => ServiceRequestBodyDetectedComputeType::Static,
            ComputeType::Process => ServiceRequestBodyDetectedComputeType::Process,
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
) -> ServiceRequestBodyPackageManager {
    match pm {
        PackageManagerType::Npm => ServiceRequestBodyPackageManager::Npm,
        PackageManagerType::Yarn => ServiceRequestBodyPackageManager::Yarn,
        PackageManagerType::Pnpm => ServiceRequestBodyPackageManager::Pnpm,
        PackageManagerType::Bun => ServiceRequestBodyPackageManager::Bun,
        PackageManagerType::Pip => ServiceRequestBodyPackageManager::Pip,
        PackageManagerType::Uv => ServiceRequestBodyPackageManager::Uv,
        PackageManagerType::Poetry => ServiceRequestBodyPackageManager::Poetry,
        PackageManagerType::Pub => ServiceRequestBodyPackageManager::Pub,
        PackageManagerType::Go => ServiceRequestBodyPackageManager::Go,
    }
}
