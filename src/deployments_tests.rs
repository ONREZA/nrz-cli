use super::deployments::DeploymentStatus;

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
