use super::deployments::{Deployment, DeploymentStatus, first_preview_url};

#[test]
fn deployment_status_accepts_server_enum_case() {
    let status: DeploymentStatus = serde_json::from_str(r#""LIVE""#).unwrap();
    assert_eq!(status, DeploymentStatus::Live);

    let status: DeploymentStatus = serde_json::from_str(r#""SMOKE_TESTING""#).unwrap();
    assert_eq!(status, DeploymentStatus::SmokeTesting);
}

#[test]
fn deployment_status_still_accepts_cli_lowercase_case() {
    let status: DeploymentStatus = serde_json::from_str(r#""live""#).unwrap();
    assert_eq!(status, DeploymentStatus::Live);

    let status: DeploymentStatus = serde_json::from_str(r#""smoke_testing""#).unwrap();
    assert_eq!(status, DeploymentStatus::SmokeTesting);
}

#[test]
fn first_preview_url_uses_preview_deployment_url() {
    let deployments = vec![
        deployment(Some(false), Some("https://production.example.com")),
        deployment(Some(true), Some("https://preview.example.com")),
    ];

    assert_eq!(
        first_preview_url(&deployments),
        Some("https://preview.example.com")
    );
}

#[test]
fn first_preview_url_ignores_preview_without_url() {
    let deployments = vec![
        deployment(Some(true), None),
        deployment(Some(false), Some("https://production.example.com")),
    ];

    assert_eq!(first_preview_url(&deployments), None);
}

fn deployment(is_preview: Option<bool>, url: Option<&str>) -> Deployment {
    Deployment {
        id: "deployment-id".to_string(),
        status: DeploymentStatus::Live,
        is_preview,
        is_rollback: None,
        is_active: None,
        commit_sha: None,
        branch: None,
        url: url.map(str::to_string),
        created_at: None,
        deployed_at: None,
        finished_at: None,
    }
}
