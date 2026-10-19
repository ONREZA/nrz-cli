use super::*;

fn verifier(manifest: Value) -> StreamingTarVerifier {
    let logical_manifest_sha256 = crate::compute_logical_manifest_sha256(&manifest);
    let source_sha256 = sha256_hex(b"fixture");
    let input = SourceBundleVerificationInput {
        owner_workspace_id: "fixture".to_string(),
        source_artifact_id: compute_source_artifact_id(
            "fixture",
            &logical_manifest_sha256,
            &source_sha256,
            None,
        ),
        source_sha256,
        logical_manifest_sha256,
        budget: SourceBundleVerificationBudget::from_manifest(
            &serde_json::from_value(manifest.clone()).unwrap(),
        )
        .unwrap(),
    };
    StreamingTarVerifier::new(input)
}

fn manifest_header(size: u64) -> tar::Header {
    let mut header = tar::Header::new_ustar();
    header
        .set_path(SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH)
        .unwrap();
    header.set_mode(0o644);
    header.set_size(size);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    header
}

#[test]
fn manifest_admission_pauses_before_allocating_a_declared_large_body() {
    let mut verifier = verifier(
        serde_json::from_str(include_str!("../tests/fixtures/basic-manifest.json")).unwrap(),
    );
    verifier.memory_admission = true;
    verifier
        .push(manifest_header(16 * 1024 * 1024).as_bytes())
        .unwrap();

    assert_eq!(
        verifier.pending_memory.map(|(bytes, _)| bytes),
        Some(64 * 1024 * 1024)
    );
    assert_eq!(verifier.entry.as_ref().unwrap().pax_body.capacity(), 0);
    assert!(verifier.buffer.is_empty());
}

#[test]
fn pax_metadata_cannot_borrow_the_logical_file_payload_budget() {
    let mut manifest: Value =
        serde_json::from_str(include_str!("../tests/fixtures/basic-manifest.json")).unwrap();
    manifest["files"][1]["size"] = serde_json::json!(1024 * 1024 * 1024_u64);
    let canonical = canonical_source_logical_manifest_json(&manifest);
    let mut verifier = verifier(manifest);
    verifier
        .push(manifest_header(canonical.len() as u64).as_bytes())
        .unwrap();
    verifier.push(canonical.as_bytes()).unwrap();
    verifier
        .push(&vec![0; padding_for(canonical.len())])
        .unwrap();
    let mut header = tar::Header::new_ustar();
    header.set_path("PaxHeader").unwrap();
    header.set_mode(0o644);
    header.set_size(MAX_METADATA_OVERHEAD_BYTES + 1);
    header.set_entry_type(tar::EntryType::XHeader);
    header.set_cksum();

    let error = verifier.push(header.as_bytes()).unwrap_err();

    assert_eq!(error.error_code, "SOURCE_EXPANSION_LIMIT");
    assert!(verifier.entry.is_none());
    assert!(verifier.buffer.is_empty());
}
