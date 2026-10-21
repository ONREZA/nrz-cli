use super::*;

#[test]
fn preflight_uses_serde_json_syntax_and_recursion_limits() {
    for body in [
        b"null".as_slice(),
        b"true",
        b"-1",
        b"1.5",
        br#""escaped\nvalue""#,
        br#"{"": [null, false, 12, -2, 3.5, "x", {}, []]}"#,
    ] {
        assert!(source_bundle_verification_manifest_memory_bytes(body).is_ok());
    }
    let too_deep = format!("{}0{}", "[".repeat(129), "]".repeat(129));
    for body in [
        b"{".as_slice(),
        b"null trailing",
        b"[1,]",
        &[0xff],
        too_deep.as_bytes(),
    ] {
        assert!(serde_json::from_slice::<Value>(body).is_err());
        assert_eq!(
            source_bundle_verification_manifest_memory_bytes(body)
                .unwrap_err()
                .error_code,
            "SOURCE_LOGICAL_MANIFEST_INVALID"
        );
    }
}

#[test]
fn ordinary_large_manifests_do_not_pay_for_aggregate_symlink_traversal() {
    let files = (0..10_000)
        .map(|index| {
            serde_json::json!({
                "path": format!("{index:05}{}", "x".repeat(251)),
                "sha256": "a".repeat(64), "size": 0, "role": "compute"
            })
        })
        .collect::<Vec<_>>();
    let manifest = serde_json::json!({ "schemaVersion": "SOURCE_BUNDLE_V1.0", "files": files });
    let body = serde_json::to_vec(&manifest).unwrap();
    let forecast = source_bundle_verification_manifest_memory_bytes(&body).unwrap();

    assert!(forecast < crate::source_bundle_verifier_max_decoder_memory_bytes());
}
