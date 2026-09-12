use nrz_source_bundle::{
    EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION, EDGE_BUILD_SOURCE_BUNDLE_V1_FILE, EdgeBuildHandoffV1,
    EdgeBuildSourceBundleV1, SOURCE_BUNDLE_V1_MEDIA_TYPE, SOURCE_BUNDLE_V1_SCHEMA_VERSION,
};

fn valid_handoff() -> EdgeBuildHandoffV1 {
    EdgeBuildHandoffV1 {
        schema_version: EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION.to_string(),
        source_bundle: EdgeBuildSourceBundleV1 {
            path: EDGE_BUILD_SOURCE_BUNDLE_V1_FILE.to_string(),
            media_type: SOURCE_BUNDLE_V1_MEDIA_TYPE.to_string(),
            schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
            sha256: "a".repeat(64),
            size_bytes: 1024,
            logical_manifest_sha256: "b".repeat(64),
        },
    }
}

#[test]
fn accepts_the_exact_versioned_handoff_contract() {
    assert_eq!(valid_handoff().validate(), Ok(()));
}

#[test]
fn rejects_a_producer_selected_source_path() {
    let mut handoff = valid_handoff();
    handoff.source_bundle.path = "../source-bundle.tar.zst".to_string();

    assert_eq!(
        handoff.validate(),
        Err("Edge build handoff source bundle path is not canonical".to_string())
    );
}

#[test]
fn rejects_noncanonical_digest_and_size_evidence() {
    let mut handoff = valid_handoff();
    handoff.source_bundle.sha256 = "A".repeat(64);
    assert_eq!(
        handoff.validate(),
        Err(
            "Edge build handoff source bundle sha256 must be a lowercase SHA-256 digest"
                .to_string()
        )
    );

    let mut handoff = valid_handoff();
    handoff.source_bundle.size_bytes = 0;
    assert_eq!(
        handoff.validate(),
        Err("Edge build handoff source bundle sizeBytes is invalid".to_string())
    );
}
