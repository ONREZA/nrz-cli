use super::*;

// ── resolve_health_check ─────────────────────────────────────

#[test]
fn health_check_flag_wins_over_config_and_autodetect() {
    let dir = tempdir().unwrap();
    // Create a detectable endpoint that should be ignored
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Http(
        "/from-config".to_string(),
    ));

    let detection = make_detection("nextjs", None);
    let result = resolve_health_check(
        Some("/from-flag"),
        &config,
        dir.path(),
        &detection,
        dir.path(),
        true, // json mode suppresses output
    )
    .unwrap();

    assert_eq!(result.path, Some("/from-flag".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Flag));
}

#[test]
fn health_check_config_wins_over_autodetect() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Http(
        "/from-config".to_string(),
    ));

    let detection = make_detection("nextjs", None);
    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert_eq!(result.path, Some("/from-config".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Config));
}

#[test]
fn health_check_autodetect_when_no_flag_or_config() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("nextjs", None);

    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert_eq!(result.path, Some("/api/health".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Detected));
}

#[test]
fn health_check_default_tcp_when_nothing_found() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("other", None);

    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert!(result.path.is_none());
    assert!(matches!(result.source, HealthCheckSource::Default));
}

#[test]
fn health_check_flag_none_gives_tcp() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("other", None);

    for alias in &["none", "NONE", "false", "tcp", "TCP", "None"] {
        let result = resolve_health_check(
            Some(alias),
            &config,
            dir.path(),
            &detection,
            dir.path(),
            true,
        )
        .unwrap();

        assert!(
            result.path.is_none(),
            "expected TCP for alias \"{alias}\", got: {:?}",
            result.path
        );
        assert!(matches!(result.source, HealthCheckSource::Flag));
    }
}

#[test]
fn health_check_config_tcp_overrides_autodetect() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Tcp);

    let detection = make_detection("nextjs", None);
    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert!(result.path.is_none());
    assert!(matches!(result.source, HealthCheckSource::Config));
}

// ── validate_health_path ─────────────────────────────────────

#[test]
fn validate_health_path_rejects_no_slash() {
    let result = validate_health_path("health", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must start with '/'")
    );
}

#[test]
fn validate_health_path_rejects_parent_traversal() {
    let result = validate_health_path("/../../etc", "--health-check-path");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("'..'"));
}

#[test]
fn validate_health_path_rejects_query() {
    let result = validate_health_path("/health?v=1", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("query or fragment")
    );
}

#[test]
fn validate_health_path_rejects_fragment() {
    let result = validate_health_path("/health#section", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("query or fragment")
    );
}

#[test]
fn validate_health_path_accepts_valid_path() {
    assert!(validate_health_path("/health", "--health-check-path").is_ok());
    assert!(validate_health_path("/api/health", "--health-check-path").is_ok());
    assert!(validate_health_path("/v1/healthz", "--health-check-path").is_ok());
}

// ── ComputeConfigBody serialization ─────────────────────────

#[test]
fn compute_config_body_with_health_check_path_serializes_camel_case() {
    let body = compute_config_body(Some("/api/health".to_string()));
    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(
        value.get("healthCheckPath").and_then(|v| v.as_str()),
        Some("/api/health")
    );
    assert!(value.get("health_check_path").is_none());
}

#[test]
fn compute_config_body_without_path_clears_previous_http_check() {
    let body = compute_config_body(None);
    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value, serde_json::json!({"healthCheckPath":null}));
}

#[test]
fn deploy_output_serializes_public_json_as_camel_case() {
    let output = DeployOutput {
        deployment_id: "dep_123".to_string(),
        url: "https://example.test".to_string(),
        status: "live".to_string(),
        target: deploy_target_output(Some(false)),
        preview_protected: true,
        runtime_artifact_files: file_breakdown(),
        warnings: vec![],
        health_check: Some(HealthCheckInfo::Http {
            path: "/health".to_string(),
            source: HealthCheckSourceTag::Config,
        }),
        verification: Some(verify::DeployVerificationOutput {
            status: "passed",
            url: "https://example.test/health".to_string(),
            path: "/health".to_string(),
            status_code: 200,
            used_preview_bypass: true,
            preview_access_revoked: Some(true),
        }),
    };

    let value = serde_json::to_value(&output).unwrap();

    assert_eq!(value["deploymentId"], "dep_123");
    assert_eq!(value["target"]["environment"], "preview");
    assert_eq!(value["previewProtected"], true);
    assert_eq!(value["runtimeArtifactFiles"]["total"], 20_679);
    assert_eq!(value["healthCheck"]["path"], "/health");
    assert_eq!(value["verification"]["usedPreviewBypass"], true);
    assert_eq!(value["verification"]["previewAccessRevoked"], true);
    assert!(value.get("deployment_id").is_none());
    assert!(value.get("health_check").is_none());
}

#[test]
fn skipped_deploy_output_is_machine_readable() {
    let value = serde_json::to_value(SkippedDeployOutput {
        deployment_id: "dep_123",
        status: "skipped",
    })
    .unwrap();

    assert_eq!(
        value,
        serde_json::json!({
            "deploymentId": "dep_123",
            "status": "skipped",
        })
    );
}
