pub(crate) fn make_detection(
    framework: &str,
    ssr: Option<crate::detect::types::SsrAnalysis>,
) -> crate::detect::types::DetectionResult {
    crate::detect::types::DetectionResult {
        framework: framework.to_string(),
        name: framework.to_string(),
        version: None,
        suggested_compute: crate::detect::types::ComputeType::Process,
        reason: String::new(),
        metadata: crate::detect::types::DetectionMetadata {
            source_build_context: None,
            uses_typescript: None,
            config_files: vec![],
            runtime: crate::detect::types::RuntimeInfo {
                runtime_type: crate::detect::types::RuntimeType::Node,
                version: None,
            },
            package_manager: None,
            build_info: None,
            monorepo: None,
            ssr_analysis: ssr,

            structure: vec![],
        },
    }
}

pub(crate) fn validated_source_bundle_manifest(
    source: &crate::artifact::source_bundle_v1::SourceBundlePlan,
) -> nrz_source_bundle::SourceLogicalManifest {
    let logical =
        serde_json::from_value(serde_json::to_value(&source.logical_manifest).unwrap()).unwrap();
    nrz_runtime_artifact::validate_source_bundle_application_graph(
        &source.logical_manifest_sha256,
        &source.source_sha256,
        source.source_size_bytes,
        &logical,
    )
    .unwrap();
    logical
}

pub(crate) async fn assert_source_bundle_verified(
    source: &crate::artifact::source_bundle_v1::SourceBundlePlan,
    logical: &nrz_source_bundle::SourceLogicalManifest,
) {
    let owner = uuid::Uuid::nil().to_string();
    nrz_source_bundle::verify_source_bundle_bytes(
        nrz_source_bundle::SourceBundleVerificationInput {
            owner_workspace_id: owner.clone(),
            source_artifact_id: nrz_source_bundle::compute_source_artifact_id(
                &owner,
                &source.logical_manifest_sha256,
                &source.source_sha256,
                None,
            ),
            source_sha256: source.source_sha256.clone(),
            logical_manifest_sha256: source.logical_manifest_sha256.clone(),
            budget: nrz_source_bundle::SourceBundleVerificationBudget::from_manifest(logical)
                .unwrap(),
        },
        std::fs::read(source.source_path()).unwrap().into(),
    )
    .await
    .unwrap();
}

pub(crate) fn unpack_source_bundle(
    source: &crate::artifact::source_bundle_v1::SourceBundlePlan,
) -> tempfile::TempDir {
    let unpacked = tempfile::tempdir().unwrap();
    let decoder =
        zstd::stream::read::Decoder::new(std::fs::File::open(source.source_path()).unwrap())
            .unwrap();
    tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
    unpacked
}

pub(crate) fn assert_compute_layer_entrypoints(logical: &nrz_source_bundle::SourceLogicalManifest) {
    for layer in logical
        .layers
        .iter()
        .filter(|layer| layer.target == "COMPUTE")
    {
        let file = logical
            .files
            .iter()
            .find(|file| Some(file.path.as_str()) == layer.entrypoint.as_deref())
            .unwrap();
        assert_eq!(file.role, "compute");
        assert_eq!(file.layer_name.as_deref(), Some(layer.name.as_str()));
        let target = layer.runtime_config.as_ref().unwrap()["buildRuntimeVersion"]
            .as_str()
            .unwrap();
        nrz_runtime_artifact::compile_source_runtime_layer_for_target(layer, &[], Some(target))
            .unwrap();
    }
}
