use super::*;
use tempfile::tempdir;

fn binding() -> EdgeBuildBindingV2 {
    EdgeBuildBindingV2 {
        deployment_id: Uuid::now_v7().to_string(),
        command_id: Uuid::now_v7().to_string(),
        command_digest: "a".repeat(64),
        build_attempt_id: Uuid::now_v7().to_string(),
        attempt: 1,
        source_writer_generation: 2,
        source_environment_id: Uuid::now_v7().to_string(),
    }
}

#[tokio::test]
async fn actual_static_deploy_plan_publishes_initial_v2_before_v1_materialization()
-> anyhow::Result<()> {
    assert_static_deploy_plan_publishes_v2(0).await
}

#[tokio::test]
async fn actual_static_deploy_plan_publishes_retried_v2_before_v1_materialization()
-> anyhow::Result<()> {
    assert_static_deploy_plan_publishes_v2(1).await
}

async fn assert_static_deploy_plan_publishes_v2(retry_count: u32) -> anyhow::Result<()> {
    let project = tempdir()?;
    let output = tempdir()?;
    fs::create_dir(project.path().join("dist"))?;
    fs::write(
        project.path().join("dist/index.html"),
        b"<html>static</html>",
    )?;
    let mut config = crate::config::ProjectConfig::default();
    config.build.output_directory = Some("dist".into());
    let (command, args) = crate::deploy::test_support::deploy_context(
        project.path(),
        &config,
        &["--skip-build", "--skip-install"],
    );
    let plan = crate::deploy::test_support::build_plan(&args, &command, &[]).await?;
    let mut scope = binding();
    scope.attempt = u64::from(retry_count) + 1;
    let publisher = EdgeBuildHandoffOutput {
        root: output.path().into(),
        mode: EDGE_BUILD_HANDOFF_MODE_V2,
        binding: Some(scope.clone()),
    };
    let handoff = publisher.publish_v2(
        &plan,
        &scope.deployment_id,
        &scope.source_environment_id,
        retry_count,
    )?;
    assert_eq!(handoff.binding, scope);
    let bytes = fs::read(output.path().join(EDGE_BUILD_SOURCE_MANIFEST_V2_FILE))?;
    assert_eq!(
        blake3::hash(&bytes).to_hex().as_str(),
        handoff.source_manifest.blake3
    );
    let persisted: EdgeBuildHandoffV2 =
        serde_json::from_slice(&fs::read(output.path().join(EDGE_BUILD_HANDOFF_V2_FILE))?)?;
    assert_eq!(persisted, handoff);
    assert!(
        !output
            .path()
            .join(nrz_source_bundle::EDGE_BUILD_SOURCE_BUNDLE_V1_FILE)
            .exists()
    );
    assert!(
        publisher
            .publish_v2(
                &plan,
                &scope.deployment_id,
                &scope.source_environment_id,
                retry_count + 1
            )
            .is_err()
    );
    assert!(
        publisher
            .publish_v2(
                &plan,
                &scope.deployment_id,
                &scope.source_environment_id,
                retry_count
            )
            .is_err()
    );
    fs::write(project.path().join("dist/index.html"), b"mutated")?;
    let root = output.path().join(EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY);
    let frozen = if root.join("index.html").exists() {
        root.join("index.html")
    } else {
        root.join("dist/index.html")
    };
    assert_eq!(fs::read(frozen)?, b"<html>static</html>");
    Ok(())
}

#[test]
fn authenticated_binding_rejects_stale_wrong_scope_zero_and_overflow() {
    let scope = binding();
    let publisher = |binding| EdgeBuildHandoffOutput {
        root: std::path::PathBuf::new(),
        mode: EDGE_BUILD_HANDOFF_MODE_V2,
        binding: Some(binding),
    };
    let output = publisher(scope.clone());
    assert!(
        output
            .validate_v2_binding(&Uuid::now_v7().to_string(), &scope.source_environment_id, 0)
            .is_err()
    );
    assert!(
        output
            .validate_v2_binding(&scope.deployment_id, &Uuid::now_v7().to_string(), 0)
            .is_err()
    );
    // A stale positive command attempt must not be accepted as the API retry counter.
    assert!(
        output
            .validate_v2_binding(&scope.deployment_id, &scope.source_environment_id, 1)
            .is_err()
    );
    let mut zero = scope.clone();
    zero.attempt = 0;
    assert!(
        publisher(zero)
            .validate_v2_binding(&scope.deployment_id, &scope.source_environment_id, 0)
            .is_err()
    );
    let mut overflow = scope.clone();
    overflow.attempt = u64::from(u32::MAX) + 1;
    assert!(
        publisher(overflow)
            .validate_v2_binding(&scope.deployment_id, &scope.source_environment_id, u32::MAX)
            .is_err()
    );
}

#[test]
fn source_environment_claim_is_required_and_canonical() -> anyhow::Result<()> {
    let valid = binding();
    valid.validate().map_err(anyhow::Error::msg)?;
    let mut missing = serde_json::to_value(&valid)?;
    missing
        .as_object_mut()
        .unwrap()
        .remove("sourceEnvironmentId");
    assert!(serde_json::from_value::<EdgeBuildBindingV2>(missing).is_err());
    for environment in [
        "",
        "00000000-0000-0000-0000-000000000000",
        "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA",
        "aaaaaaaaaaaa4aaa8aaaaaaaaaaaaaaa",
    ] {
        let mut invalid = valid.clone();
        invalid.source_environment_id = environment.into();
        assert!(invalid.validate().is_err(), "{environment}");
    }
    let decoded: EdgeBuildBindingV2 = serde_json::from_slice(&serde_json::to_vec(&valid)?)?;
    assert_eq!(decoded, valid);
    Ok(())
}

#[test]
fn strict_handoff_rejects_host_paths_unknown_fields_and_bad_binding() -> anyhow::Result<()> {
    let handoff = EdgeBuildHandoffV2 {
        schema_version: EDGE_BUILD_HANDOFF_V2_SCHEMA_VERSION.into(),
        binding: binding(),
        source_manifest: EdgeBuildSourceManifestV2 {
            path: EDGE_BUILD_SOURCE_MANIFEST_V2_FILE.into(),
            media_type: SOURCE_BUNDLE_V2_MEDIA_TYPE.into(),
            schema_version: SOURCE_BUNDLE_V2_SCHEMA_VERSION.into(),
            blake3: "b".repeat(64),
            size_bytes: 1,
            summary: nrz_source_bundle::source_bundle_v2::SourceBundleV2ManifestSummary {
                file_count: 1,
                logical_static_bytes: 1,
                artifact_size_bytes: 0,
                max_static_file_size_bytes: 1,
            },
        },
        source_tree: EdgeBuildSourceTreeV2 {
            path: EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY.into(),
        },
    };
    handoff.validate().map_err(anyhow::Error::msg)?;
    let mut tampered = handoff.clone();
    tampered.source_tree.path = "/etc".into();
    assert!(tampered.validate().is_err());
    let mut tampered = handoff.clone();
    tampered.binding.command_digest = "A".repeat(64);
    assert!(tampered.validate().is_err());
    let mut value = serde_json::to_value(handoff)?;
    value["untrustedAuthority"] = serde_json::json!(true);
    assert!(serde_json::from_value::<EdgeBuildHandoffV2>(value).is_err());
    Ok(())
}
