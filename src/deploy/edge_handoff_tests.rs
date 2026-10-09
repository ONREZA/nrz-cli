use std::fs;

use nrz_source_bundle::{
    EDGE_BUILD_HANDOFF_V1_FILE, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION,
    EDGE_BUILD_SOURCE_BUNDLE_V1_FILE, EdgeBuildHandoffV1,
};
use tempfile::tempdir;
use uuid::Uuid;

use super::edge_handoff::{
    EDGE_BUILD_HANDOFF_MODE_V1, EdgeBuildHandoffOutput, validate_resume_arguments,
};
use crate::artifact::FileEntry;
use crate::artifact::source_bundle_v1::{SourceBundlePlan, build_source_bundle_plan};
use crate::build::manifest::Manifest;
use crate::cli::DeployArgs;

fn prepare_server_handoff(
    project: &std::path::Path,
    output: &std::path::Path,
    manifest: &Manifest,
    source: &[u8],
) -> anyhow::Result<(SourceBundlePlan, EdgeBuildHandoffOutput)> {
    fs::write(project.join("server.js"), source)?;
    let plan = build_source_bundle_plan(
        project,
        manifest,
        &[FileEntry {
            path: "server.js".to_string(),
            size: source.len() as u64,
            content_hash: nrz_source_bundle::sha256_hex(source),
            kind: crate::artifact::ArtifactFileKind::File,
            symlink_resolved_path: None,
            symlink_target: None,
        }],
    )?;
    let publisher = EdgeBuildHandoffOutput::from_values(
        Some(EDGE_BUILD_HANDOFF_MODE_V1),
        Some(output),
        true,
        Some(Uuid::now_v7()),
    )?
    .expect("handoff mode");
    Ok((plan, publisher))
}

fn server_manifest() -> anyhow::Result<Manifest> {
    Ok(serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [{"name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js"}],
        "routes": []
    }))?)
}

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
            .contains("requires NRZ_EDGE_BUILD_HANDOFF=V1")
    );

    let error = EdgeBuildHandoffOutput::from_values(
        Some(EDGE_BUILD_HANDOFF_MODE_V1),
        Some(std::path::Path::new("/workspace/output")),
        false,
        Some(deployment_id),
    )
    .err()
    .expect("non-platform runner must fail");
    assert!(error.to_string().contains("requires NRZ_RUNNER=PLATFORM"));

    let error = EdgeBuildHandoffOutput::from_values(
        Some(EDGE_BUILD_HANDOFF_MODE_V1),
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
fn publishes_archive_before_one_strict_atomic_descriptor() -> anyhow::Result<()> {
    let project = tempdir()?;
    let output = tempdir()?;
    let source = b"console.log('ready');\n";
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [{
            "name": "server",
            "target": "COMPUTE",
            "directory": ".",
            "entry": "server.js"
        }],
        "routes": []
    }))?;
    let (plan, publisher) =
        prepare_server_handoff(project.path(), output.path(), &manifest, source)?;

    let handoff = publisher.publish(&plan)?;
    assert_eq!(handoff.schema_version, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION);
    assert_eq!(
        fs::read(output.path().join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE))?,
        fs::read(plan.source_path())?
    );
    let persisted: EdgeBuildHandoffV1 =
        serde_json::from_slice(&fs::read(output.path().join(EDGE_BUILD_HANDOFF_V1_FILE))?)?;
    assert_eq!(persisted, handoff);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            fs::metadata(output.path().join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE))?
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert!(fs::read_dir(output.path())?.all(|entry| {
        !entry
            .expect("output entry")
            .file_name()
            .to_string_lossy()
            .starts_with('.')
    }));

    let error = publisher
        .publish(&plan)
        .expect_err("published handoff is immutable");
    assert!(error.to_string().contains("output already exists"));
    Ok(())
}

#[test]
fn handoff_rejects_a_source_graph_with_unowned_compute_entrypoint() -> anyhow::Result<()> {
    let project = tempdir()?;
    let output = tempdir()?;
    let source = b"export default () => 'ready';\n";
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            {"name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js"},
            {"name": "worker", "target": "COMPUTE", "directory": ".", "entry": "server.js"}
        ],
        "routes": []
    }))?;
    let (plan, publisher) =
        prepare_server_handoff(project.path(), output.path(), &manifest, source)?;

    let error = publisher
        .publish(&plan)
        .expect_err("invalid source graph must fail");
    assert!(error.to_string().contains("entrypoint is not owned"));
    assert_eq!(
        error
            .chain()
            .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
            .map(|error| error.code.as_str()),
        Some("INVALID_RUNTIME_ARTIFACT_GRAPH")
    );
    assert!(!output.path().join(EDGE_BUILD_HANDOFF_V1_FILE).exists());
    assert!(
        !output
            .path()
            .join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE)
            .exists()
    );
    Ok(())
}

#[test]
fn handoff_commit_preserves_outputs_created_after_preflight() -> anyhow::Result<()> {
    for descriptor_collision in [false, true] {
        let project = tempdir()?;
        let output = tempdir()?;
        let manifest = server_manifest()?;
        let (plan, publisher) =
            prepare_server_handoff(project.path(), output.path(), &manifest, b"ready")?;
        let archive = output.path().join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE);
        let descriptor = output.path().join(EDGE_BUILD_HANDOFF_V1_FILE);
        let foreign = if descriptor_collision {
            &descriptor
        } else {
            &archive
        };
        fs::write(foreign, b"foreign output")?;
        let result = publisher.publish_inner(
            &plan,
            output.path(),
            &output.path().join("archive.tmp"),
            &archive,
            &output.path().join("descriptor.tmp"),
            &descriptor,
        );
        assert!(
            result.is_err(),
            "existing final output must reject the commit"
        );
        assert_eq!(fs::read(foreign)?, b"foreign output");
        if descriptor_collision {
            assert!(
                !archive.exists(),
                "failed descriptor commit must release its owned archive"
            );
        } else {
            assert!(!descriptor.exists());
        }
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn handoff_reports_real_directory_permission_failures() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    const WORKER: &str = "NRZ_HANDOFF_DIRECTORY_PERMISSION_WORKER";
    if std::env::var_os(WORKER).is_none() {
        let mut child = std::process::Command::new(std::env::current_exe()?);
        child
            .arg("--exact")
            .arg("deploy::edge_handoff_tests::handoff_reports_real_directory_permission_failures")
            .env(WORKER, "1")
            .env("TMPDIR", "/tmp");
        let result = child.output()?;
        assert!(
            result.status.success(),
            "permission worker failed: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return Ok(());
    }

    // Drop privileges after exec so a private checkout remains accessible to the parent runner.
    if unsafe { libc::geteuid() } == 0 {
        assert_eq!(unsafe { libc::setgid(65_534) }, 0, "drop worker group");
        assert_eq!(unsafe { libc::setuid(65_534) }, 0, "drop worker user");
    }

    for (mode, expected_context, committed) in [
        (0o200, "failed to inspect handoff output", false),
        (0o333, "failed to open handoff directory", true),
    ] {
        let project = tempdir()?;
        let output = tempdir()?;
        let manifest = server_manifest()?;
        let (plan, publisher) =
            prepare_server_handoff(project.path(), output.path(), &manifest, b"ready")?;
        fs::set_permissions(output.path(), fs::Permissions::from_mode(mode))?;
        let result = publisher.publish(&plan);
        fs::set_permissions(output.path(), fs::Permissions::from_mode(0o700))?;

        let error = result.expect_err("directory permission failures must reach the caller");
        assert!(error.to_string().contains(expected_context), "{error:#}");
        assert_eq!(
            error
                .chain()
                .find_map(|cause| cause.downcast_ref::<std::io::Error>())
                .map(std::io::Error::kind),
            Some(std::io::ErrorKind::PermissionDenied)
        );
        let mut entries = fs::read_dir(output.path())?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort();
        let expected: Vec<std::ffi::OsString> = if committed {
            assert_eq!(
                fs::read(output.path().join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE))?,
                fs::read(plan.source_path())?
            );
            vec![
                EDGE_BUILD_HANDOFF_V1_FILE.into(),
                EDGE_BUILD_SOURCE_BUNDLE_V1_FILE.into(),
            ]
        } else {
            Vec::new()
        };
        assert_eq!(entries, expected);
    }
    Ok(())
}
