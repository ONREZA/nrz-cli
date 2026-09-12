use bytes::Bytes;
use futures::stream;
use nrz_source_bundle::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SourceBundleVerificationInput,
    canonical_source_logical_manifest_json, compute_logical_manifest_sha256,
    compute_source_artifact_id, sha256_hex, verify_source_bundle_bytes,
    verify_source_bundle_stream,
};
use serde_json::{Value, json};

const OWNER_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000001";
const FIXTURE_LOGICAL_MANIFEST_SHA256: &str =
    "caa9dd9266993677c330917f8025a22bb9fc77d19dbe308b993f19789b76b8e5";
const FIXTURE_SOURCE_ARTIFACT_ID_FOR_A_SOURCE_SHA: &str =
    "1f268533777567893f1b4a3136aef72777effc17fe41ebabd84a93499d3bd545";
const MANY_TINY_FILES_COUNT: usize = 66_000;

#[tokio::test]
async fn accepts_deterministic_tar_zst_bundle() {
    let manifest = fixture_manifest();
    let compressed = bundle(
        &manifest,
        &[
            ("dist/index.html", b"<h1>ONREZA</h1>\n".as_slice()),
            ("server/index.js", b"export default fetch;\n".as_slice()),
        ],
    );
    let input = input_for(manifest, &compressed);

    let result = verify_source_bundle_bytes(input, compressed).await.unwrap();

    assert_eq!(result.summary.file_count, 2);
    assert_eq!(result.summary.logical_static_bytes, 16);
    assert_eq!(result.summary.artifact_size_bytes, 22);
}

#[tokio::test]
async fn rejects_an_executable_manifest_entry_with_a_non_executable_archive_mode() {
    let mut manifest = fixture_manifest();
    manifest["files"][1]["executable"] = json!(true);
    let compressed = bundle(
        &manifest,
        &[
            ("dist/index.html", b"<h1>ONREZA</h1>\n".as_slice()),
            ("server/index.js", b"export default fetch;\n".as_slice()),
        ],
    );
    let input = input_for(manifest, &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_ARCHIVE_MODE_MISMATCH");
}

#[tokio::test]
async fn allows_tar_framing_overhead_for_many_tiny_files() {
    let empty_sha = sha256_hex(b"");
    let files = (0..MANY_TINY_FILES_COUNT)
        .map(|index| {
            json!({
                "path": format!("files/{index:05}.txt"),
                "sha256": empty_sha,
                "size": 0,
                "role": "compute"
            })
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "schemaVersion": "SOURCE_BUNDLE_V1.0",
        "capabilities": [],
        "files": files,
        "layers": [],
        "routes": [],
        "entrypoints": []
    });
    let compressed = compressed_tar(&manifest, |tar| {
        for index in 0..MANY_TINY_FILES_COUNT {
            tar_file(tar, &format!("files/{index:05}.txt"), b"", b'0');
        }
        tar.extend_from_slice(&[0; 512]);
    });
    let input = input_for(manifest, &compressed);

    let result = verify_source_bundle_bytes(input, compressed).await.unwrap();

    assert_eq!(result.summary.file_count, MANY_TINY_FILES_COUNT as i32);
    assert_eq!(result.summary.artifact_size_bytes, 0);
}

#[tokio::test]
async fn rejects_undeclared_archive_entry() {
    let manifest = fixture_manifest();
    let compressed = bundle(
        &manifest,
        &[
            ("dist/index.html", b"<h1>ONREZA</h1>\n".as_slice()),
            ("server/index.js", b"export default fetch;\n".as_slice()),
            ("dist/extra.txt", b"extra".as_slice()),
        ],
    );
    let input = input_for(manifest, &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_ARCHIVE_UNDECLARED_ENTRY");
}

#[tokio::test]
async fn rejects_archive_without_embedded_logical_manifest() {
    let manifest = fixture_manifest();
    let compressed = compressed_tar_without_manifest(|tar| {
        tar_file(tar, "dist/index.html", b"<h1>ONREZA</h1>\n", b'0');
        tar_file(tar, "server/index.js", b"export default fetch;\n", b'0');
        tar.extend_from_slice(&[0; 512]);
    });
    let input = input_for(manifest, &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_LOGICAL_MANIFEST_ORDER_INVALID");
}

#[tokio::test]
async fn rejects_nonzero_trailing_data_after_end_marker() {
    let manifest = fixture_manifest();
    let compressed = compressed_tar(&manifest, |tar| {
        tar_file(tar, "dist/index.html", b"<h1>ONREZA</h1>\n", b'0');
        tar_file(tar, "server/index.js", b"export default fetch;\n", b'0');
        tar.extend_from_slice(&[0; 512]);
        tar.extend_from_slice(b"not zero");
    });
    let input = input_for(manifest, &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_TAR_TRAILING_DATA");
}

#[tokio::test]
async fn rejects_hardlink_entries() {
    let manifest = fixture_manifest();
    let compressed = compressed_tar(&manifest, |tar| {
        tar_file(tar, "dist/index.html", b"", b'1');
        tar.extend_from_slice(&[0; 512]);
    });
    let input = input_for(manifest, &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_TAR_ENTRY_UNSUPPORTED");
}

#[tokio::test]
async fn accepts_repeated_regular_file_bytes_for_local_hardlink_sources() {
    let body = b"esbuild";
    let file_sha = sha256_hex(body);
    let manifest = json!({
        "schemaVersion": "SOURCE_BUNDLE_V1.0",
        "capabilities": [],
        "files": [
            {
                "path": "node_modules/@esbuild/linux-x64/bin/esbuild",
                "sha256": file_sha.clone(),
                "size": body.len(),
                "role": "compute"
            },
            {
                "path": "node_modules/esbuild/bin/esbuild",
                "sha256": file_sha,
                "size": body.len(),
                "role": "compute"
            }
        ],
        "layers": [],
        "routes": [],
        "entrypoints": []
    });
    let compressed = bundle(
        &manifest,
        &[
            (
                "node_modules/@esbuild/linux-x64/bin/esbuild",
                body.as_slice(),
            ),
            ("node_modules/esbuild/bin/esbuild", body.as_slice()),
        ],
    );
    let input = input_for(manifest, &compressed);

    let result = verify_source_bundle_bytes(input, compressed).await.unwrap();

    assert_eq!(result.summary.file_count, 2);
    assert_eq!(result.summary.artifact_size_bytes, (body.len() * 2) as i64);
}

#[tokio::test]
async fn accepts_safe_relative_symlink_entries() {
    let link_target = ".pnpm/pkg";
    let bin_link_target = "../pkg/bin/tool";
    let body = b"#!/usr/bin/env node\n";
    let manifest = json!({
        "schemaVersion": "SOURCE_BUNDLE_V1.0",
        "capabilities": [],
        "files": [
            {
                "path": "node_modules/.bin/tool",
                "sha256": sha256_hex(bin_link_target.as_bytes()),
                "size": 0,
                "role": "compute",
                "entryType": "symlink",
                "linkTarget": bin_link_target
            },
            {
                "path": "node_modules/pkg",
                "sha256": sha256_hex(link_target.as_bytes()),
                "size": 0,
                "role": "compute",
                "entryType": "symlink",
                "linkTarget": link_target
            },
            {
                "path": "node_modules/.pnpm/pkg/bin/tool",
                "sha256": sha256_hex(body),
                "size": body.len(),
                "role": "compute"
            }
        ],
        "layers": [],
        "routes": [],
        "entrypoints": []
    });
    let compressed = compressed_tar(&manifest, |tar| {
        tar_symlink(tar, "node_modules/.bin/tool", bin_link_target);
        tar_symlink(tar, "node_modules/pkg", link_target);
        tar_file(tar, "node_modules/.pnpm/pkg/bin/tool", body, b'0');
        tar.extend_from_slice(&[0; 512]);
    });
    let input = input_for(manifest, &compressed);

    let result = verify_source_bundle_bytes(input, compressed).await.unwrap();

    assert_eq!(result.summary.file_count, 3);
    assert_eq!(result.summary.artifact_size_bytes, body.len() as i64);
}

#[tokio::test]
async fn accepts_directory_symlink_targets_with_symlink_only_descendants() {
    let alias_target = "dir";
    let child_target = "../real.txt";
    let body = b"real";
    let manifest = json!({
        "schemaVersion": "SOURCE_BUNDLE_V1.0",
        "capabilities": [],
        "files": [
            {
                "path": "alias",
                "sha256": sha256_hex(alias_target.as_bytes()),
                "size": 0,
                "role": "compute",
                "entryType": "symlink",
                "linkTarget": alias_target
            },
            {
                "path": "dir/link",
                "sha256": sha256_hex(child_target.as_bytes()),
                "size": 0,
                "role": "compute",
                "entryType": "symlink",
                "linkTarget": child_target
            },
            {
                "path": "real.txt",
                "sha256": sha256_hex(body),
                "size": body.len(),
                "role": "compute"
            }
        ],
        "layers": [],
        "routes": [],
        "entrypoints": []
    });
    let compressed = compressed_tar(&manifest, |tar| {
        tar_symlink(tar, "alias", alias_target);
        tar_symlink(tar, "dir/link", child_target);
        tar_file(tar, "real.txt", body, b'0');
        tar.extend_from_slice(&[0; 512]);
    });
    let input = input_for(manifest, &compressed);

    let result = verify_source_bundle_bytes(input, compressed).await.unwrap();

    assert_eq!(result.summary.file_count, 3);
    assert_eq!(result.summary.artifact_size_bytes, body.len() as i64);
}

#[tokio::test]
async fn rejects_symlink_targets_that_traverse_through_regular_files() {
    let link_target = "dir/file/../other";
    let body = b"real";
    let manifest = json!({
        "schemaVersion": "SOURCE_BUNDLE_V1.0",
        "capabilities": [],
        "files": [
            {
                "path": "alias",
                "sha256": sha256_hex(link_target.as_bytes()),
                "size": 0,
                "role": "compute",
                "entryType": "symlink",
                "linkTarget": link_target
            },
            {
                "path": "dir/file",
                "sha256": sha256_hex(body),
                "size": body.len(),
                "role": "compute"
            },
            {
                "path": "dir/other",
                "sha256": sha256_hex(body),
                "size": body.len(),
                "role": "compute"
            }
        ],
        "layers": [],
        "routes": [],
        "entrypoints": []
    });
    let compressed = compressed_tar(&manifest, |tar| {
        tar_symlink(tar, "alias", link_target);
        tar_file(tar, "dir/file", body, b'0');
        tar_file(tar, "dir/other", body, b'0');
        tar.extend_from_slice(&[0; 512]);
    });
    let input = input_for(manifest, &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_MANIFEST_LINK_TARGET_UNSAFE");
}

#[tokio::test]
async fn rejects_unsupported_pax_keys() {
    let manifest = fixture_manifest();
    let compressed = compressed_tar(&manifest, |tar| {
        pax_header(tar, "mtime", "1");
        tar_file(tar, "dist/index.html", b"<h1>ONREZA</h1>\n", b'0');
        tar.extend_from_slice(&[0; 512]);
    });
    let input = input_for(manifest, &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_TAR_PAX_UNSUPPORTED");
}

#[tokio::test]
async fn verifies_chunked_streaming_input() {
    let manifest = fixture_manifest();
    let compressed = bundle(
        &manifest,
        &[
            ("dist/index.html", b"<h1>ONREZA</h1>\n".as_slice()),
            ("server/index.js", b"export default fetch;\n".as_slice()),
        ],
    );
    let input = input_for(manifest, &compressed);
    let chunks = compressed
        .chunks(3)
        .map(|chunk| Ok::<_, std::convert::Infallible>(Bytes::copy_from_slice(chunk)))
        .collect::<Vec<_>>();

    let result = verify_source_bundle_stream(input, stream::iter(chunks))
        .await
        .unwrap();

    assert_eq!(result.summary.file_count, 2);
}

#[test]
fn locks_shared_manifest_digest_and_source_artifact_vector() {
    let manifest = fixture_manifest();
    let logical_manifest_sha256 = compute_logical_manifest_sha256(&manifest);
    let source_artifact_id = compute_source_artifact_id(
        OWNER_WORKSPACE_ID,
        &logical_manifest_sha256,
        &"a".repeat(64),
        None,
    );

    assert_eq!(logical_manifest_sha256, FIXTURE_LOGICAL_MANIFEST_SHA256);
    assert_eq!(
        source_artifact_id,
        FIXTURE_SOURCE_ARTIFACT_ID_FOR_A_SOURCE_SHA
    );
}

fn fixture_manifest() -> Value {
    serde_json::from_str(include_str!("fixtures/basic-manifest.json")).unwrap()
}

fn input_for(manifest: Value, compressed: &Bytes) -> SourceBundleVerificationInput {
    let logical_manifest_sha256 = compute_logical_manifest_sha256(&manifest);
    let source_sha256 = sha256_hex(compressed);
    let source_artifact_id = compute_source_artifact_id(
        OWNER_WORKSPACE_ID,
        &logical_manifest_sha256,
        &source_sha256,
        None,
    );
    SourceBundleVerificationInput {
        owner_workspace_id: OWNER_WORKSPACE_ID.to_string(),
        source_artifact_id,
        source_sha256,
        logical_manifest_sha256,
    }
}

fn bundle(manifest: &Value, entries: &[(&str, &[u8])]) -> Bytes {
    compressed_tar(manifest, |tar| {
        for (path, body) in entries {
            tar_file(tar, path, body, b'0');
        }
        tar.extend_from_slice(&[0; 512]);
    })
}

fn compressed_tar(manifest: &Value, write: impl FnOnce(&mut Vec<u8>)) -> Bytes {
    let mut tar = Vec::new();
    let manifest_body = canonical_source_logical_manifest_json(manifest);
    tar_file(
        &mut tar,
        SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
        manifest_body.as_bytes(),
        b'0',
    );
    write(&mut tar);
    Bytes::from(zstd::bulk::compress(&tar, 3).unwrap())
}

fn compressed_tar_without_manifest(write: impl FnOnce(&mut Vec<u8>)) -> Bytes {
    let mut tar = Vec::new();
    write(&mut tar);
    Bytes::from(zstd::bulk::compress(&tar, 3).unwrap())
}

fn tar_file(tar: &mut Vec<u8>, path: &str, body: &[u8], typeflag: u8) {
    let mut header = [0_u8; 512];
    let path_bytes = path.as_bytes();
    assert!(path_bytes.len() <= 100);
    header[..path_bytes.len()].copy_from_slice(path_bytes);
    header[100..108].copy_from_slice(b"0000644\0");
    let size = format!("{:011o}\0", body.len());
    header[124..(124 + size.len())].copy_from_slice(size.as_bytes());
    header[156] = typeflag;
    tar.extend_from_slice(&header);
    if typeflag == b'0' || typeflag == 0 || typeflag == b'x' {
        tar.extend_from_slice(body);
        let padding = (512 - (body.len() % 512)) % 512;
        tar.extend(std::iter::repeat_n(0, padding));
    }
}

fn tar_symlink(tar: &mut Vec<u8>, path: &str, target: &str) {
    let mut header = [0_u8; 512];
    let path_bytes = path.as_bytes();
    let target_bytes = target.as_bytes();
    assert!(path_bytes.len() <= 100);
    assert!(target_bytes.len() <= 100);
    header[..path_bytes.len()].copy_from_slice(path_bytes);
    header[100..108].copy_from_slice(b"0000777\0");
    header[124..136].copy_from_slice(b"00000000000\0");
    header[156] = b'2';
    header[157..(157 + target_bytes.len())].copy_from_slice(target_bytes);
    tar.extend_from_slice(&header);
}

fn pax_header(tar: &mut Vec<u8>, key: &str, value: &str) {
    let body = pax_record(key, value);
    tar_file(tar, "PaxHeader", body.as_bytes(), b'x');
}

fn pax_record(key: &str, value: &str) -> String {
    let mut length = key.len() + value.len() + 4;
    loop {
        let record = format!("{length} {key}={value}\n");
        if record.len() == length {
            return record;
        }
        length = record.len();
    }
}
