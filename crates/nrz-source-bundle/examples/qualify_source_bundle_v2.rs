//! Manual byte-ceiling qualification: cargo run -p nrz-source-bundle --example qualify_source_bundle_v2
//! This does not qualify concurrency, latency, filesystem trees, or file contents.

use nrz_contract::artifact_json_v1;
use nrz_source_bundle::source_bundle_v2::{
    SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES, SourceBundleV2ManifestBudget, SourceBundleV2ManifestError,
    SourceBundleV2ManifestSummary, decode_source_bundle_v2_manifest,
    encode_source_bundle_v2_manifest,
};
use serde_json::{Value, json};

fn manifest() -> Value {
    json!({
        "schemaVersion":"SOURCE_BUNDLE_V2.0", "capabilities":[], "layers":[],
        "routes":[], "entrypoints":[], "files":[{
            "path":"a", "role":"static", "entryType":"file",
            "blobDigest":{"algorithm":"BLAKE3", "value":"0".repeat(64)},
            "size":0, "executable":false, "contentType":""
        }]
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: 1,
        max_logical_bytes: 0,
        max_static_file_bytes: 0,
    };
    let overhead = encode_source_bundle_v2_manifest(manifest(), budget)?.len();
    let content_bytes = SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES - overhead;
    let mut value = manifest();
    value["files"][0]["contentType"] = Value::String("x".repeat(content_bytes));
    let mut bytes = encode_source_bundle_v2_manifest(value, budget)?;
    assert_eq!(bytes.len(), SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES);
    let verified = decode_source_bundle_v2_manifest(&bytes, budget)?;
    assert_eq!(
        verified.summary(),
        SourceBundleV2ManifestSummary {
            file_count: 1,
            logical_static_bytes: 0,
            artifact_size_bytes: 0,
            max_static_file_size_bytes: 0,
        }
    );
    assert_eq!(
        verified.manifest_blake3(),
        blake3::hash(&bytes).to_hex().as_str()
    );
    println!(
        "manifestBytes={} manifestBlake3={}",
        bytes.len(),
        verified.manifest_blake3()
    );
    drop(verified);

    bytes.push(b'\n');
    assert!(matches!(
        decode_source_bundle_v2_manifest(&bytes, budget),
        Err(SourceBundleV2ManifestError::Canonical(
            artifact_json_v1::Error::ByteLimit
        ))
    ));
    drop(bytes);

    let mut oversized = manifest();
    oversized["files"][0]["contentType"] = Value::String("x".repeat(content_bytes + 1));
    assert!(matches!(
        encode_source_bundle_v2_manifest(oversized, budget),
        Err(SourceBundleV2ManifestError::Canonical(
            artifact_json_v1::Error::ByteLimit
        ))
    ));
    println!("decodeMaxPlusOne=ByteLimit encodeMaxPlusOne=ByteLimit");
    Ok(())
}
