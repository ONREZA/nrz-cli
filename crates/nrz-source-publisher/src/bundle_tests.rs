use super::*;
use crate::publisher::validate_upload_targets;
use nrz_api::PrepareUpload200Response;
use serde_json::json;

#[test]
fn multipart_targets_are_bound_to_the_verified_part_inventory() {
    let manifest: SourceLogicalManifest = serde_json::from_value(json!({
        "schemaVersion": nrz_source_bundle::SOURCE_BUNDLE_V1_SCHEMA_VERSION,
        "files": [], "layers": [], "routes": [], "entrypoints": [], "capabilities": []
    }))
    .unwrap();
    let part_hash = nrz_source_bundle::sha256_hex(b"part one");
    let last_hash = nrz_source_bundle::sha256_hex(b"last");
    let bundle = PreparedSourceBundle {
        input: SourceBundleInput {
            path: PathBuf::new(),
            source_sha256: part_hash.clone(),
            source_size_bytes: MULTIPART_CHUNK_BYTES + 4,
            logical_manifest_sha256: last_hash.clone(),
            verification_budget: SourceBundleVerificationBudget::from_manifest(&manifest).unwrap(),
        },
        source_artifact_id: last_hash.clone(),
        manifest,
        max_static_file_size_bytes: 0,
        multipart: Some(MultipartDescriptor {
            part_size_bytes: MULTIPART_CHUNK_BYTES,
            parts: vec![
                MultipartPart {
                    part_number: 1,
                    size_bytes: MULTIPART_CHUNK_BYTES,
                    sha256: part_hash.clone(),
                },
                MultipartPart {
                    part_number: 2,
                    size_bytes: 4,
                    sha256: last_hash.clone(),
                },
            ],
        }),
    };
    let response = json!({
        "bucket": "fixture", "expiresAt": "2026-09-01T12:00:00Z", "fastPath": false,
        "kind": "source-upload", "sourceArtifactId": last_hash,
        "sourceObjectKey": "source/fixture", "uploadSessionId": Uuid::now_v7(),
        "requiredComplete": "multipart-complete+upload-complete",
        "multipart": { "mode": "multipart", "uploadId": "fixture", "chunkSize": MULTIPART_CHUNK_BYTES,
            "chunks": [
                { "partNumber": 1, "contentLength": MULTIPART_CHUNK_BYTES, "sha256": part_hash, "url": "https://objects.invalid/1" },
                { "partNumber": 2, "contentLength": 4, "sha256": last_hash, "url": "https://objects.invalid/2" }
            ] }
    });
    let valid: PrepareUpload200Response = serde_json::from_value(response.clone()).unwrap();
    validate_upload_targets(&bundle, &valid).unwrap();
    let mut reversed = valid;
    reversed.multipart.as_mut().unwrap().chunks.reverse();
    validate_upload_targets(&bundle, &reversed).unwrap();
    let mut duplicate = reversed.clone();
    let chunks = &mut duplicate.multipart.as_mut().unwrap().chunks;
    chunks[1] = chunks[0].clone();
    assert!(matches!(
        validate_upload_targets(&bundle, &duplicate),
        Err(SourcePublicationError::InvalidResponse(_))
    ));

    for (label, pointer, value) in [
        ("chunk size", "/multipart/chunkSize", json!(1)),
        ("unknown part", "/multipart/chunks/0/partNumber", json!(3)),
        ("zero part", "/multipart/chunks/0/partNumber", json!(0)),
        ("duplicate part", "/multipart/chunks/0/partNumber", json!(2)),
        (
            "oversized part",
            "/multipart/chunks/1/contentLength",
            json!(i64::MAX),
        ),
        (
            "negative size",
            "/multipart/chunks/1/contentLength",
            json!(-1),
        ),
        (
            "different digest",
            "/multipart/chunks/0/sha256",
            json!(last_hash),
        ),
        ("incomplete inventory", "/multipart/chunks", json!([])),
        (
            "wrong completion",
            "/requiredComplete",
            json!("upload-complete"),
        ),
    ] {
        let mut changed = response.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        let changed: PrepareUpload200Response = serde_json::from_value(changed).unwrap();
        assert!(
            matches!(
                validate_upload_targets(&bundle, &changed),
                Err(SourcePublicationError::InvalidResponse(_))
            ),
            "{label}"
        );
    }
}
