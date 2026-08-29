use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use nrz_source_bundle::{
    EDGE_BUILD_HANDOFF_V1_FILE, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION,
    EDGE_BUILD_SOURCE_BUNDLE_V1_FILE, EdgeBuildHandoffV1,
};
use tempfile::tempdir;
use uuid::Uuid;

use super::edge_handoff::{EDGE_BUILD_HANDOFF_MODE_V1, EdgeBuildHandoffOutput};
use crate::artifact::FileEntry;
use crate::artifact::source_bundle_v1::build_source_bundle_plan;
use crate::build::manifest::Manifest;

#[test]
fn edge_handoff_mode_is_explicit_and_runner_scoped() {
    let deployment_id = Uuid::now_v7();
    assert!(
        EdgeBuildHandoffOutput::from_values(None, None, false, None)
            .expect("mode resolution")
            .is_none()
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
fn publishes_archive_before_one_strict_atomic_descriptor() -> anyhow::Result<()> {
    let project = tempdir()?;
    let output = tempdir()?;
    let source = b"console.log('ready');\n";
    fs::write(project.path().join("server.js"), source)?;
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
    let plan = build_source_bundle_plan(
        project.path(),
        &manifest,
        &[FileEntry {
            path: "server.js".to_string(),
            size: source.len() as u64,
            content_hash: nrz_source_bundle::sha256_hex(source),
            kind: crate::artifact::ArtifactFileKind::File,
            symlink_resolved_path: None,
        }],
    )?;
    let publisher = EdgeBuildHandoffOutput::from_values(
        Some(EDGE_BUILD_HANDOFF_MODE_V1),
        Some(output.path()),
        true,
        Some(Uuid::now_v7()),
    )?
    .expect("handoff mode");

    let handoff = publisher.publish(&plan)?;
    assert_eq!(handoff.schema_version, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION);
    assert_eq!(
        fs::read(output.path().join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE))?,
        fs::read(plan.source_path())?
    );
    let persisted: EdgeBuildHandoffV1 =
        serde_json::from_slice(&fs::read(output.path().join(EDGE_BUILD_HANDOFF_V1_FILE))?)?;
    assert_eq!(persisted, handoff);
    assert_eq!(
        fs::metadata(output.path().join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE))?
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
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
