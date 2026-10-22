use super::edge_handoff::{
    EDGE_BUILD_HANDOFF_MODE_V2, EdgeBuildHandoffOutput, validate_resume_arguments,
};
use crate::cli::DeployArgs;
use uuid::Uuid;

#[test]
fn edge_handoff_mode_is_explicit_and_runner_scoped() {
    let deployment_id = Uuid::now_v7();
    assert!(
        EdgeBuildHandoffOutput::from_values(None, None, false, None)
            .expect("mode resolution")
            .is_none()
    );

    let error = EdgeBuildHandoffOutput::from_values(None, None, true, Some(deployment_id))
        .err()
        .expect("resume without handoff mode must fail");
    assert!(
        error
            .to_string()
            .contains("requires NRZ_EDGE_BUILD_HANDOFF=V2")
    );

    let error = EdgeBuildHandoffOutput::from_values(
        Some(EDGE_BUILD_HANDOFF_MODE_V2),
        Some(std::path::Path::new("/workspace/output")),
        false,
        Some(deployment_id),
    )
    .err()
    .expect("non-platform runner must fail");
    assert!(error.to_string().contains("requires NRZ_RUNNER=PLATFORM"));

    let error = EdgeBuildHandoffOutput::from_values(
        Some(EDGE_BUILD_HANDOFF_MODE_V2),
        Some(std::path::Path::new("relative-output")),
        true,
        Some(deployment_id),
    )
    .err()
    .expect("relative output must fail");
    assert!(error.to_string().contains("must be an absolute path"));
}

#[test]
fn platform_resume_rejects_every_mutable_override() {
    let args = DeployArgs {
        dir: "/workspace/source".to_string(),
        prod: true,
        dry: true,
        verify: true,
        wait_timeout: 120,
        environment: Some("production".to_string()),
        service_id: None,
        project_id: Some("project-1".to_string()),
        skip_build: true,
        skip_install: true,
        no_log_upload: true,
        log_upload_debug: true,
        build_command: Some("npm run build".to_string()),
        skip_env_check: true,
        resume_deployment: Some(Uuid::now_v7().to_string()),
        compute: Some("static".to_string()),
        health_check_path: Some("/health".to_string()),
        app: Some("web".to_string()),
        force_rules: true,
    };

    let error = validate_resume_arguments(&args).expect_err("overrides must be rejected");
    let message = error.to_string();
    for flag in [
        "--prod",
        "--dry",
        "--verify",
        "--environment",
        "--project-id",
        "--skip-build",
        "--skip-install",
        "--no-log-upload",
        "--log-upload-debug",
        "--build-command",
        "--skip-env-check",
        "--compute",
        "--health-check-path",
        "--app",
        "--force-rules",
    ] {
        assert!(
            message.contains(flag),
            "missing rejected flag {flag}: {message}"
        );
    }
}

#[test]
fn legacy_handoff_mode_is_rejected_before_output_creation() {
    let output = tempfile::tempdir().unwrap();
    let error = EdgeBuildHandoffOutput::from_values(
        Some("V1"),
        Some(output.path()),
        true,
        Some(Uuid::now_v7()),
    )
    .err()
    .expect("legacy writer must be rejected");
    assert!(
        error
            .to_string()
            .contains("Source V1 build handoff is no longer supported")
    );
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
}
