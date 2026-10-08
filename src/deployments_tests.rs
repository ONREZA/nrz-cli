use super::deployments::{Deployment, DeploymentStatus, first_preview_url};

#[test]
fn generated_deployment_statuses_keep_cli_json_and_human_names() {
    use nrz_api::Project200ResponseProjectLatestDeploymentStatus as Wire;
    for (wire, expected) in [
        (Wire::Pending, "pending"),
        (Wire::Queued, "queued"),
        (Wire::Building, "building"),
        (Wire::Uploading, "uploading"),
        (Wire::Ingesting, "ingesting"),
        (Wire::Skipped, "skipped"),
        (Wire::SmokeTesting, "smoke_testing"),
        (Wire::Live, "live"),
        (Wire::Stopped, "stopped"),
        (Wire::Failed, "failed"),
        (Wire::Cancelled, "cancelled"),
    ] {
        let status = DeploymentStatus::from(wire);
        assert_eq!(serde_json::to_value(&status).unwrap(), expected);
        assert_eq!(status.to_string(), expected);
    }
}

#[test]
fn shortened_ids_preserve_short_inputs_and_utf8_boundaries() {
    use super::deployments::truncate_id;
    for (input, limit, expected) in [
        ("89abcdef-0123", 8, "89abcdef"),
        ("short", 8, "short"),
        ("", 8, ""),
        ("short", 0, ""),
        ("é🙂", 2, "é"),
        ("é🙂", 3, "é🙂"),
    ] {
        assert_eq!(truncate_id(input, limit), expected);
    }
}

#[test]
fn first_preview_url_uses_preview_environment_address() {
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
