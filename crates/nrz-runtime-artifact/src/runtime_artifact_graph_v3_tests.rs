use super::*;
use nrz_contract::runtime_artifact_graph_v3::RuntimeArtifactGraphV3;
use serde_json::Value;

fn policy() -> ApplicationPackagePolicy {
    ApplicationPackagePolicy {
        canonicalization_profile: "test-profile-v1".into(),
        generator_identity: "test-generator-v1".into(),
    }
}

fn graph() -> Value {
    json!({
        "schemaVersion":"RUNTIME_ARTIFACT_GRAPH_V3.0",
        "application":{"schemaVersion":"APPLICATION_MEMBERSHIP_V1.0", "components":[],
            "manifest":{"mediaType":SOURCE_BUNDLE_V2_MEDIA_TYPE,
                "digest":{"algorithm":"BLAKE3", "value":"a".repeat(64)}, "size":1024}},
        "dependencies":[], "layers":[
            {"target":"STATIC", "name":"public", "rootPath":"public"},
            {"target":"COMPUTE", "name":"api", "rootPath":"server", "entrypoint":"server/main.js",
                "launch":{"profile":"NODE_24", "args":[]},
                "network":nrz_source_bundle::ComputeNetwork::public_default(),
                "limits":{"timeoutMs":9_007_199_254_740_991u64,"memoryMb":256},
                "buildRuntimeTarget":"node-24", "dependencyMounts":[]}
        ]
    })
}

fn verify(value: Value) -> Result<VerifiedRuntimeArtifactGraphV3, GraphV3Error> {
    verify_runtime_artifact_graph_v3(serde_json::from_value(value)?, &policy())
}

fn reference(bytes: &[u8]) -> BlobReference {
    BlobReference {
        media_type: RUNTIME_ARTIFACT_GRAPH_V3_MEDIA_TYPE.into(),
        digest: digest(bytes),
        size: bytes.len() as u64,
    }
}

fn package(role: &str, hex: char, erofs_size: u64) -> Value {
    let vector: Value = serde_json::from_str(include_str!(
        "../../nrz-contract/fixtures/runtime-artifact-graph-v3-conformance.json"
    ))
    .unwrap();
    let graph: Value = serde_json::from_str(vector["canonical"].as_str().unwrap()).unwrap();
    let mut package = graph["application"]["components"][0].clone();
    let layout = crate::application_package_v1::application_package_layout(erofs_size).unwrap();
    package["role"] = json!(role);
    package["blob"]["digest"]["value"] = json!(hex.to_string().repeat(64));
    package["blob"]["size"] = json!(layout.package_size);
    package["erofsSize"] = json!(erofs_size);
    package["verity"]["dataBlockCount"] = json!(layout.data_block_count);
    package["verity"]["treeOffset"] = json!(layout.tree_offset);
    package["verity"]["treeLength"] = json!(layout.tree_length);
    package
}

fn graph_with_physical_references() -> Value {
    let mut value = graph();
    value["application"]["components"] =
        json!([package("STATIC", 'b', 4096), package("COMPUTE", 'c', 4097)]);
    value["dependencies"] = json!((0..2).map(|index| {
        let hex = if index == 0 { "d" } else { "e" }.repeat(64);
        let size = if index == 0 { 4096 } else { 8192 };
        json!({"legacy":{
            "materializationId":format!("{index:064x}"), "manifestDigest":"f".repeat(64),
            "kind":"JAVASCRIPT_NODE_MODULES", "mountPoint":format!("/output/dep{index}"),
            "compatibility":{"runtimeFamily":"javascript", "runtimeVersion":"node-24",
                "os":"linux", "architecture":"x86_64", "libc":"glibc", "abi":"glibc-2.42",
                "packageManager":"bun", "packageManagerVersion":"1.4.2",
                "runnerRootfsDigest":format!("sha256:{}", "a".repeat(64)), "buildPolicyGeneration":1},
            "blobDescriptor":{"mediaType":crate::DEPENDENCY_EROFS_MEDIA_TYPE,
                "digest":format!("sha256:{hex}"), "size":size}},
            "physicalBlob":{"mediaType":crate::DEPENDENCY_EROFS_MEDIA_TYPE,
                "digest":{"algorithm":"BLAKE3", "value":hex}, "size":size}})
    }).collect::<Vec<_>>());
    value["layers"][1]["dependencyMounts"] = json!((0..2).map(|index|
        json!({"materializationId":format!("{index:064x}"), "mountPoint":format!("/output/dep{index}")}))
        .collect::<Vec<_>>());
    value
}

#[test]
fn physical_identity_rejects_conflicting_sizes_across_graph_members() {
    let baseline = graph_with_physical_references();
    verify(baseline.clone()).unwrap();
    for (target, source) in [
        (
            "/application/manifest/digest/value",
            "/application/components/0/blob/digest/value",
        ),
        (
            "/application/manifest/digest/value",
            "/dependencies/0/physicalBlob/digest/value",
        ),
        (
            "/application/components/1/blob/digest/value",
            "/application/components/0/blob/digest/value",
        ),
        (
            "/dependencies/1/physicalBlob/digest/value",
            "/application/components/0/blob/digest/value",
        ),
        (
            "/dependencies/1/physicalBlob/digest/value",
            "/dependencies/0/physicalBlob/digest/value",
        ),
    ] {
        let mut value = baseline.clone();
        *value.pointer_mut(target).unwrap() = baseline.pointer(source).unwrap().clone();
        assert!(
            matches!(verify(value.clone()), Err(GraphV3Error::Invariant(message))
            if message == "physical blob identity has conflicting sizes"),
            "{target} <- {source}"
        );
        let bytes = artifact_json_v1::encode(&value, limits()).unwrap();
        assert!(
            matches!(decode_runtime_artifact_graph_v3(&bytes, &reference(&bytes), &policy()),
            Err(GraphV3Error::Invariant(message)) if message == "physical blob identity has conflicting sizes")
        );
    }
    let mut legacy = baseline;
    for dependency in legacy["dependencies"].as_array_mut().unwrap() {
        dependency.as_object_mut().unwrap().remove("physicalBlob");
    }
    legacy["dependencies"][1]["legacy"]["blobDescriptor"]["digest"] =
        legacy["dependencies"][0]["legacy"]["blobDescriptor"]["digest"].clone();
    assert!(
        matches!(verify(legacy), Err(GraphV3Error::Invariant(message))
        if message == "physical blob identity has conflicting sizes")
    );
}

#[test]
fn physical_identity_allows_equal_size_aliases_and_separate_digest_algorithms() {
    let mut value = graph_with_physical_references();
    value["application"]["manifest"]["digest"]["value"] = json!("b".repeat(64));
    value["application"]["manifest"]["size"] = json!(4096);
    value["application"]["components"][1] = package("COMPUTE", 'b', 4096);
    for dependency in value["dependencies"].as_array_mut().unwrap() {
        dependency["physicalBlob"]["digest"]["value"] = json!("b".repeat(64));
        dependency["physicalBlob"]["size"] = json!(4096);
        dependency["legacy"]["blobDescriptor"]["size"] = json!(4096);
    }
    let verified = verify(value.clone()).unwrap();
    decode_runtime_artifact_graph_v3(
        verified.canonical_bytes(),
        &reference(verified.canonical_bytes()),
        &policy(),
    )
    .unwrap();
    for dependency in value["dependencies"].as_array_mut().unwrap() {
        dependency.as_object_mut().unwrap().remove("physicalBlob");
        dependency["legacy"]["blobDescriptor"]["digest"] =
            json!(format!("sha256:{}", "b".repeat(64)));
        dependency["legacy"]["blobDescriptor"]["size"] = json!(8192);
    }
    verify(value).unwrap();
}

#[test]
fn exact_external_identity_and_immutable_projection_roundtrip() {
    let verified = verify(graph()).unwrap();
    let bytes = verified.canonical_bytes();
    let expected = reference(bytes);
    let decoded = decode_runtime_artifact_graph_v3(bytes, &expected, &policy()).unwrap();
    assert_eq!(decoded.wire(), verified.wire());
    assert_eq!(decoded.canonical_bytes(), bytes);
    assert_eq!(decoded.graph_digest(), &expected.digest);
    assert_eq!(
        decoded.application_identity(),
        verified.application_identity()
    );
    let Layer::Compute { limits, .. } = &decoded.wire().layers[1] else {
        panic!()
    };
    assert_eq!(
        std::time::Duration::from_millis(limits.timeout_ms.unwrap()).as_millis(),
        9_007_199_254_740_991
    );

    let mut different_launch = graph();
    different_launch["layers"][1]["launch"]["args"] = json!(["changed"]);
    let different_launch = verify(different_launch).unwrap();
    assert_ne!(different_launch.graph_digest(), verified.graph_digest());
    assert_eq!(
        different_launch.application_identity(),
        verified.application_identity()
    );
    let mut different_source = graph();
    different_source["application"]["manifest"]["digest"]["value"] = json!("b".repeat(64));
    let different_source = verify(different_source).unwrap();
    assert_ne!(
        different_source.application_identity(),
        verified.application_identity()
    );
}

#[test]
fn decoder_checks_reference_canonical_bytes_and_closed_shape() {
    let verified = verify(graph()).unwrap();
    let bytes = verified.canonical_bytes();
    for field in ["size", "digest", "mediaType"] {
        let mut expected = reference(bytes);
        match field {
            "size" => expected.size += 1,
            "digest" => expected.digest.value = "b".repeat(64),
            _ => expected.media_type = SOURCE_BUNDLE_V2_MEDIA_TYPE.into(),
        }
        assert!(decode_runtime_artifact_graph_v3(bytes, &expected, &policy()).is_err());
    }
    let mut whitespace = bytes.to_vec();
    whitespace.push(b'\n');
    assert!(matches!(
        decode_runtime_artifact_graph_v3(&whitespace, &reference(&whitespace), &policy()),
        Err(GraphV3Error::Canonical(
            artifact_json_v1::Error::NonCanonical
        ))
    ));
    let mut unknown = graph();
    unknown["graphDigest"] = json!("self-reference");
    let bytes = artifact_json_v1::encode(&unknown, limits()).unwrap();
    assert!(matches!(
        decode_runtime_artifact_graph_v3(&bytes, &reference(&bytes), &policy()),
        Err(GraphV3Error::Shape(_))
    ));
}

#[test]
fn typed_metadata_rejects_invalid_domains_before_identity() {
    for (pointer, value) in [
        ("/application/manifest/size", json!(0)),
        (
            "/application/manifest/size",
            json!(SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES + 1),
        ),
        ("/application/manifest/digest/value", json!("not-hex")),
        ("/layers/1/name", json!("public")),
        ("/layers/1/name", json!("bad\nname")),
        ("/layers/1/rootPath", json!(".")),
        ("/layers/1/entrypoint", json!("elsewhere/main.js")),
        ("/layers/1/entrypoint", json!("server\\main.js")),
        ("/layers/1/limits/timeoutMs", json!(0)),
        (
            "/layers/1/limits/timeoutMs",
            json!(crate::MAX_SAFE_INTEGER + 1),
        ),
        ("/layers/1/limits/memoryMb", json!(8193)),
        ("/layers/1/buildRuntimeTarget", json!("node-22")),
        ("/layers/1/launch/args", json!(["a".repeat(4097)])),
        ("/layers/1/launch/args", json!(["bad\0arg"])),
        ("/layers/1/network/ports/0/port", json!(0)),
    ] {
        let mut wire = graph();
        *wire.pointer_mut(pointer).unwrap() = value;
        assert!(verify(wire).is_err(), "{pointer}");
    }
    let mut wire = graph();
    wire["layers"][1]["runtimePolicyLineage"] = json!("NODE:024");
    assert!(verify(wire).is_err());
}

#[test]
fn graph_byte_limit_accepts_exact_boundary_and_rejects_next_byte() {
    let mut value = graph();
    value["layers"][1]["launch"]["args"] = json!(vec![""; 8]);
    let base = verify(value.clone()).unwrap().canonical_bytes().len();
    let mut remaining = RUNTIME_ARTIFACT_GRAPH_V3_MAX_BYTES - base;
    for arg in value["layers"][1]["launch"]["args"].as_array_mut().unwrap() {
        let len = remaining.min(4096);
        *arg = json!("x".repeat(len));
        remaining -= len;
    }
    assert_eq!(remaining, 0);
    let verified = verify(value.clone()).unwrap();
    assert_eq!(
        verified.canonical_bytes().len(),
        RUNTIME_ARTIFACT_GRAPH_V3_MAX_BYTES
    );
    decode_runtime_artifact_graph_v3(
        verified.canonical_bytes(),
        &reference(verified.canonical_bytes()),
        &policy(),
    )
    .unwrap();
    let last = value["layers"][1]["launch"]["args"][7]
        .as_str()
        .unwrap()
        .to_owned()
        + "x";
    value["layers"][1]["launch"]["args"][7] = json!(last);
    assert!(matches!(
        verify(value),
        Err(GraphV3Error::Canonical(artifact_json_v1::Error::ByteLimit))
    ));
    let oversized = vec![b' '; RUNTIME_ARTIFACT_GRAPH_V3_MAX_BYTES + 1];
    assert!(
        decode_runtime_artifact_graph_v3(&oversized, &reference(&oversized), &policy()).is_err()
    );
}

#[test]
fn typed_cardinality_is_checked_without_schema_validation() {
    let mut value = graph();
    value["layers"] = json!(
        (0..11)
            .map(|i| json!({"target":"STATIC", "name":format!("layer{i}")}))
            .collect::<Vec<_>>()
    );
    let wire: RuntimeArtifactGraphV3 = serde_json::from_value(value).unwrap();
    assert!(verify_runtime_artifact_graph_v3(wire, &policy()).is_err());
}

#[test]
fn frozen_canonical_membership_and_graph_vectors_cover_descriptor_identity() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../nrz-contract/fixtures/runtime-artifact-graph-v3-conformance.json"
    ))
    .unwrap();
    let bytes = vector["canonical"].as_str().unwrap().as_bytes();
    let expected = BlobReference {
        media_type: RUNTIME_ARTIFACT_GRAPH_V3_MEDIA_TYPE.into(),
        digest: BlobDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: vector["graphBlake3"].as_str().unwrap().into(),
        },
        size: bytes.len() as u64,
    };
    let decoded = decode_runtime_artifact_graph_v3(bytes, &expected, &policy()).unwrap();
    assert_eq!(
        decoded.application_identity().value,
        vector["applicationBlake3"]
    );
    let encoded = verify_runtime_artifact_graph_v3(decoded.wire().clone(), &policy()).unwrap();
    assert_eq!(encoded.canonical_bytes(), bytes);
    assert_eq!(
        artifact_json_v1::encode(
            &serde_json::to_value(&decoded.wire().application).unwrap(),
            limits()
        )
        .unwrap(),
        vector["applicationCanonical"].as_str().unwrap().as_bytes()
    );
    for (field, changed) in [
        (
            "/application/components/0/blob/digest/value",
            json!("e".repeat(64)),
        ),
        (
            "/application/components/0/verity/rootHash",
            json!("e".repeat(64)),
        ),
        (
            "/application/components/0/verity/salt",
            json!("e".repeat(64)),
        ),
        ("/application/components/0/role", json!("STATIC")),
        ("/application/components/0/erofsSize", json!(4095)),
    ] {
        let mut value: Value = serde_json::from_slice(bytes).unwrap();
        *value.pointer_mut(field).unwrap() = changed;
        let different = verify(value).unwrap();
        assert_ne!(
            different.application_identity(),
            decoded.application_identity(),
            "{field}"
        );
        assert_ne!(different.graph_digest(), decoded.graph_digest(), "{field}");
        assert!(
            decode_runtime_artifact_graph_v3(different.canonical_bytes(), &expected, &policy())
                .is_err(),
            "{field}"
        );
    }
}
