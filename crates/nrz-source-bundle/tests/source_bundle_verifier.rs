use bytes::Bytes;
use futures::stream;
use nrz_source_bundle::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SourceBundleVerificationBudget,
    SourceBundleVerificationInput, canonical_source_logical_manifest_json,
    compute_logical_manifest_sha256, compute_source_artifact_id, sha256_hex,
    verify_source_bundle_bytes, verify_source_bundle_stream,
};
use serde_json::{Value, json};

const OWNER_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000001";
const FIXTURE_LOGICAL_MANIFEST_SHA256: &str =
    "caa9dd9266993677c330917f8025a22bb9fc77d19dbe308b993f19789b76b8e5";
const FIXTURE_SOURCE_ARTIFACT_ID_FOR_A_SOURCE_SHA: &str =
    "1f268533777567893f1b4a3136aef72777effc17fe41ebabd84a93499d3bd545";
const MANY_TINY_FILES_COUNT: usize = 66_000;

#[tokio::test]
async fn rejects_unsafe_or_unbounded_route_response_headers() {
    let oversized = "x".repeat(8193);
    let long_name = "x".repeat(257);
    let many = Value::Object(
        (0..129)
            .map(|i| (format!("x-fixture-{i}"), json!("value")))
            .collect(),
    );
    for headers in [
        json!({"bad name":"value"}),
        json!({"x-fixture":"value\r\nx-injected: yes"}),
        json!({"x-fixture":oversized}),
        json!({long_name:"value"}),
        many,
    ] {
        let mut manifest = fixture_manifest();
        manifest["routes"][0]["headers"] = headers;
        let compressed = bundle(
            &manifest,
            &[
                ("dist/index.html", b"<h1>ONREZA</h1>\n".as_slice()),
                ("server/index.js", b"export default fetch;\n".as_slice()),
            ],
        );
        let input = input_for(manifest, &compressed);
        assert_eq!(
            verify_source_bundle_bytes(input, compressed)
                .await
                .unwrap_err()
                .error_code,
            "SOURCE_ROUTE_HEADERS_INVALID"
        );
    }
}

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
async fn source_verification_rejects_conflicting_runtime_declarations() {
    for config in [
        json!({"applicationRuntime":{"family":"PYTHON","args":[]},"runtimeFamily":"JAVASCRIPT"}),
        json!({"applicationRuntime":{"family":"EXECUTABLE","args":[]},"isBinaryEntry":false}),
        json!({"applicationRuntime":{"family":"NODE","args":[]},"runtimeFamily":"PYTHON"}),
    ] {
        let mut manifest = fixture_manifest();
        manifest["layers"][1]["runtimeConfig"] = config;
        let bytes = bundle(
            &manifest,
            &[
                ("dist/index.html", b"<h1>ONREZA</h1>\n".as_slice()),
                ("server/index.js", b"export default fetch;\n".as_slice()),
            ],
        );
        let error = verify_source_bundle_bytes(input_for(manifest, &bytes), bytes)
            .await
            .unwrap_err();
        assert_eq!(error.error_code, "SOURCE_APPLICATION_RUNTIME_INVALID");
    }
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
async fn accepts_relative_symlink_chains_and_symlink_only_directory_descendants() {
    for (links, file_path, body) in [
        (
            [
                ("node_modules/.bin/tool", "../pkg/bin/tool"),
                ("node_modules/pkg", ".pnpm/pkg"),
            ],
            "node_modules/.pnpm/pkg/bin/tool",
            b"#!/usr/bin/env node\n".as_slice(),
        ),
        (
            [("alias", "dir"), ("dir/link", "../real.txt")],
            "real.txt",
            b"real".as_slice(),
        ),
    ] {
        let mut files = links
            .iter()
            .map(|(path, target)| {
                json!({
                    "path": path,
                    "sha256": sha256_hex(target.as_bytes()),
                    "size": 0,
                    "role": "compute",
                    "entryType": "symlink",
                    "linkTarget": target
                })
            })
            .collect::<Vec<_>>();
        files.push(json!({
            "path": file_path,
            "sha256": sha256_hex(body),
            "size": body.len(),
            "role": "compute"
        }));
        let manifest = json!({
            "schemaVersion": "SOURCE_BUNDLE_V1.0",
            "files": files
        });
        let compressed = compressed_tar(&manifest, |tar| {
            for (path, target) in links {
                tar_symlink(tar, path, target);
            }
            tar_file(tar, file_path, body, b'0');
            tar.extend_from_slice(&[0; 512]);
        });
        let input = input_for(manifest.clone(), &compressed);

        let result = verify_source_bundle_bytes(input, compressed).await.unwrap();

        assert_eq!(result.logical_manifest, manifest);
        assert_eq!(result.summary.file_count, 3);
        assert_eq!(result.summary.artifact_size_bytes, body.len() as i64);
    }
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
    assert_eq!(
        error.to_string(),
        "SOURCE_TAR_PAX_UNSUPPORTED: Unsupported PAX key: mtime"
    );
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

#[tokio::test]
async fn rejects_manifest_over_trusted_budget_before_missing_payloads() {
    let manifest = fixture_manifest();
    let compressed = bundle(&manifest, &[]);
    let exact = input_for(manifest, &compressed);
    for budget in [
        SourceBundleVerificationBudget {
            max_file_count: 1,
            ..exact.budget
        },
        SourceBundleVerificationBudget {
            max_logical_bytes: 37,
            ..exact.budget
        },
        SourceBundleVerificationBudget {
            max_static_file_bytes: 15,
            ..exact.budget
        },
    ] {
        let input = SourceBundleVerificationInput {
            budget,
            ..exact.clone()
        };
        let error = verify_source_bundle_bytes(input, compressed.clone())
            .await
            .unwrap_err();
        assert_eq!(error.error_code, "SOURCE_MANIFEST_BUDGET_EXCEEDED");
    }
}

#[tokio::test]
async fn rejects_stream_manifest_over_trusted_budget_before_bad_payload() {
    let manifest = fixture_manifest();
    let compressed = bundle(&manifest, &[("dist/index.html", b"incorrect contents")]);
    let mut input = input_for(manifest, &compressed);
    input.budget.max_logical_bytes = 37;
    let error = verify_source_bundle_stream(input, stream::iter([Ok::<_, String>(compressed)]))
        .await
        .unwrap_err();
    assert_eq!(error.error_code, "SOURCE_MANIFEST_BUDGET_EXCEEDED");
}

fn fixture_manifest() -> Value {
    serde_json::from_str(include_str!("fixtures/basic-manifest.json")).unwrap()
}

#[tokio::test]
async fn regression_reads_errors_after_the_first_zstd_frame() {
    let manifest = fixture_manifest();
    let compressed = fixture_bundle(&manifest);
    let input = input_for(manifest, &compressed);
    let stream = stream::iter([
        Ok::<Bytes, &str>(compressed),
        Err("source object read failed after the frame"),
    ]);

    let error = verify_source_bundle_stream(input, stream)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_OBJECT_READ_FAILED");
}

#[tokio::test]
async fn regression_distinguishes_corrupted_zstd_from_source_read_errors() {
    let compressed = Bytes::from_static(b"not a zstd archive");
    let input = input_for(fixture_manifest(), &compressed);

    let error = verify_source_bundle_bytes(input, compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_ZSTD_DECODE_FAILED");
}

#[tokio::test]
async fn regression_verifies_all_zstd_frames_independently_of_chunk_boundaries() {
    for trailing in [b"".as_slice(), b"nonzero tar suffix".as_slice()] {
        let manifest = fixture_manifest();
        let first = fixture_bundle(&manifest);
        let second = Bytes::from(zstd::bulk::compress(trailing, 3).unwrap());
        let mut complete = first.to_vec();
        complete.extend_from_slice(&second);
        let complete = Bytes::from(complete);
        let input = input_for(manifest, &complete);
        let joined = verify_source_bundle_bytes(input.clone(), complete).await;
        let split = verify_source_bundle_stream(
            input,
            stream::iter([Ok::<Bytes, &str>(first), Ok(second)]),
        )
        .await;

        assert_eq!(joined, split, "frame results must not depend on chunking");
        if trailing.is_empty() {
            assert_eq!(joined.unwrap().summary.file_count, 2);
        } else {
            assert_eq!(joined.unwrap_err().error_code, "SOURCE_TAR_TRAILING_DATA");
        }
    }
}

#[tokio::test]
async fn regression_rejects_corrupted_tar_header_checksums() {
    for corrupt_manifest in [true, false] {
        for checksum in [b"0000000\0", b"invalid\0"] {
            let manifest = fixture_manifest();
            let compressed = compressed_tar(&manifest, |tar| {
                let header_offset = if corrupt_manifest { 0 } else { tar.len() };
                tar_file(tar, "dist/index.html", b"<h1>ONREZA</h1>\n", b'0');
                tar_file(tar, "server/index.js", b"export default fetch;\n", b'0');
                tar[header_offset + 148..header_offset + 156].copy_from_slice(checksum);
                tar.extend_from_slice(&[0; 512]);
            });
            let input = input_for(manifest, &compressed);

            let error = verify_source_bundle_bytes(input, compressed)
                .await
                .unwrap_err();

            assert_eq!(error.error_code, "SOURCE_TAR_HEADER_INVALID");
        }
    }
}

#[tokio::test]
async fn regression_rejects_symlinks_in_reserved_metadata_entries() {
    for metadata_path in [SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, ".__onreza/extra.json"] {
        let manifest = fixture_manifest();
        let compressed = compressed_tar(&manifest, |tar| {
            let header_offset = if metadata_path == SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH {
                0
            } else {
                let offset = tar.len();
                tar_file(tar, metadata_path, b"{}", b'0');
                offset
            };
            let mut header =
                tar::Header::from_byte_slice(&tar[header_offset..header_offset + 512]).clone();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_link_name("dist/index.html").unwrap();
            header.set_cksum();
            tar[header_offset..header_offset + 512].copy_from_slice(header.as_bytes());
            tar_file(tar, "dist/index.html", b"<h1>ONREZA</h1>\n", b'0');
            tar_file(tar, "server/index.js", b"export default fetch;\n", b'0');
            tar.extend_from_slice(&[0; 512]);
        });
        let tar_bytes = zstd::stream::decode_all(compressed.as_ref()).unwrap();
        let mut archive = tar::Archive::new(tar_bytes.as_slice());
        let entry = archive
            .entries()
            .unwrap()
            .map(Result::unwrap)
            .find(|entry| entry.path().unwrap() == std::path::Path::new(metadata_path))
            .unwrap();
        assert!(entry.header().entry_type().is_symlink());
        let input = input_for(manifest, &compressed);

        let error = verify_source_bundle_bytes(input, compressed)
            .await
            .unwrap_err();

        assert_eq!(error.error_code, "SOURCE_TAR_ENTRY_UNSUPPORTED");
    }
}

#[tokio::test]
async fn regression_rejects_non_utf8_tar_paths_and_symlink_targets() {
    let path = "bad-\u{fffd}";
    let body = b"real";
    for corrupt_link in [false, true] {
        let mut files = vec![json!({
            "path":path, "sha256":sha256_hex(body), "size":body.len(), "role":"compute"
        })];
        if corrupt_link {
            files.push(json!({
                "path":"alias", "sha256":sha256_hex(path), "size":0,
                "role":"compute", "entryType":"symlink", "linkTarget":path
            }));
        }
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":files});
        let compressed = compressed_tar(&manifest, |tar| {
            let mut header_offset = tar.len();
            tar_file(tar, path, body, b'0');
            let field = if corrupt_link {
                header_offset = tar.len();
                tar_symlink(tar, "alias", path);
                157
            } else {
                0
            };
            let mut header =
                tar::Header::from_byte_slice(&tar[header_offset..header_offset + 512]).clone();
            header.as_mut_bytes()[field..field + 100].fill(0);
            header.as_mut_bytes()[field..field + 5].copy_from_slice(b"bad-\xff");
            header.set_cksum();
            tar[header_offset..header_offset + 512].copy_from_slice(header.as_bytes());
            tar.extend_from_slice(&[0; 512]);
        });
        let tar_bytes = zstd::stream::decode_all(compressed.as_ref()).unwrap();
        let mut archive = tar::Archive::new(tar_bytes.as_slice());
        let entry = archive
            .entries()
            .unwrap()
            .nth(if corrupt_link { 2 } else { 1 })
            .unwrap()
            .unwrap();
        let actual = if corrupt_link {
            entry.link_name().unwrap().unwrap()
        } else {
            entry.path().unwrap()
        };
        assert!(actual.to_str().is_none());
        let input = input_for(manifest, &compressed);

        let error = verify_source_bundle_bytes(input, compressed)
            .await
            .unwrap_err();

        assert_eq!(
            error.error_code,
            if corrupt_link {
                "SOURCE_TAR_LINK_TARGET_UNSAFE"
            } else {
                "SOURCE_TAR_PATH_INVALID"
            }
        );
    }
}

#[tokio::test]
async fn regression_rejects_dangling_and_consecutive_pax_headers() {
    for consecutive in [false, true] {
        let manifest = fixture_manifest();
        let compressed = compressed_tar(&manifest, |tar| {
            if consecutive {
                pax_header(tar, "path", "dist/index.html");
                pax_header(tar, "path", "dist/index.html");
            }
            tar_file(tar, "dist/index.html", b"<h1>ONREZA</h1>\n", b'0');
            tar_file(tar, "server/index.js", b"export default fetch;\n", b'0');
            if !consecutive {
                pax_header(tar, "path", "future-entry");
            }
            tar.extend_from_slice(&[0; 512]);
        });
        let tar_bytes = zstd::stream::decode_all(compressed.as_ref()).unwrap();
        let mut archive = tar::Archive::new(tar_bytes.as_slice());
        assert!(archive.entries().unwrap().any(|entry| entry.is_err()));
        let input = input_for(manifest, &compressed);

        let error = verify_source_bundle_bytes(input, compressed)
            .await
            .unwrap_err();

        assert_eq!(error.error_code, "SOURCE_TAR_PAX_INVALID");
    }
}

#[tokio::test]
async fn regression_tar_size_requires_nonempty_octal_text() {
    let mut binary_zero = [0_u8; 12];
    binary_zero[0] = 0x80;
    for (size_field, body, consumer_accepts, verifier_accepts) in [
        ([0_u8; 12], b"".as_slice(), false, false),
        ([b' '; 12], b"".as_slice(), false, false),
        (*b"00000000000\0", b"".as_slice(), true, true),
        (*b" 0000000001 ", b"x".as_slice(), true, true),
        (*b"00000000008\0", b"".as_slice(), false, false),
        // Binary sizes are a TAR extension outside the existing octal-only
        // SOURCE_BUNDLE contract, even though the general extractor accepts it.
        (binary_zero, b"".as_slice(), true, false),
    ] {
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
            {"path":"asset.bin","sha256":sha256_hex(body),"size":body.len(),"role":"compute"}
        ]});
        let compressed = compressed_tar(&manifest, |archive| {
            let mut header = tar::Header::new_ustar();
            header.set_path("asset.bin").unwrap();
            header.set_mode(0o644);
            header.set_size(body.len() as u64);
            header.as_mut_bytes()[124..136].copy_from_slice(&size_field);
            header.set_cksum();
            archive.extend_from_slice(header.as_bytes());
            archive.extend_from_slice(body);
            archive.extend(std::iter::repeat_n(0, (512 - body.len() % 512) % 512));
            archive.extend_from_slice(&[0; 1024]);
        });
        let decoded = zstd::stream::decode_all(compressed.as_ref()).unwrap();
        let mut archive = tar::Archive::new(decoded.as_slice());
        let native = archive.entries().unwrap().last().unwrap();
        assert_eq!(native.is_ok(), consumer_accepts, "{size_field:?}");
        if let Ok(entry) = native {
            assert_eq!(entry.size(), body.len() as u64);
            assert_eq!(
                restored_archive_path_bytes(&compressed, "asset.bin").unwrap(),
                body
            );
        }
        for chunk_size in [1, compressed.len()] {
            let input = input_for(manifest.clone(), &compressed);
            let chunks = compressed
                .chunks(chunk_size)
                .map(|chunk| Ok::<_, std::io::Error>(Bytes::copy_from_slice(chunk)))
                .collect::<Vec<_>>();
            let result = verify_source_bundle_stream(input, stream::iter(chunks)).await;
            if verifier_accepts {
                assert_eq!(result.unwrap().logical_manifest, manifest);
            } else {
                assert_eq!(
                    result.unwrap_err().error_code,
                    "SOURCE_TAR_HEADER_INVALID",
                    "{size_field:?}"
                );
            }
        }
    }
}

#[tokio::test]
async fn regression_tar_prefix_follows_the_extractors_header_flavor() {
    for (flavor, ustar) in [("old", false), ("gnu", false), ("ustar", true)] {
        for declared_prefix in [false, true] {
            let declared = if declared_prefix {
                "prefix/index.html"
            } else {
                "index.html"
            };
            let restored = if ustar {
                "prefix/index.html"
            } else {
                "index.html"
            };
            let body = b"asset";
            let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
                {"path":declared,"sha256":sha256_hex(body),"size":body.len(),"role":"static"}
            ]});
            let compressed = compressed_tar(&manifest, |archive| {
                let mut header = match flavor {
                    "old" => tar::Header::new_old(),
                    "gnu" => tar::Header::new_gnu(),
                    "ustar" => tar::Header::new_ustar(),
                    _ => unreachable!(),
                };
                header.set_path("index.html").unwrap();
                header.set_size(body.len() as u64);
                header.set_mode(0o644);
                header.as_mut_bytes()[345..351].copy_from_slice(b"prefix");
                header.set_cksum();
                archive.extend_from_slice(header.as_bytes());
                archive.extend_from_slice(body);
                archive.extend(std::iter::repeat_n(0, 512 - body.len()));
                archive.extend_from_slice(&[0; 1024]);
            });
            assert_eq!(
                restored_archive_path_bytes(&compressed, restored).unwrap(),
                body
            );
            if declared != restored {
                assert_eq!(
                    restored_archive_path_bytes(&compressed, declared)
                        .unwrap_err()
                        .kind(),
                    std::io::ErrorKind::NotFound
                );
            }
            let result =
                verify_source_bundle_bytes(input_for(manifest.clone(), &compressed), compressed)
                    .await;
            if declared == restored {
                assert_eq!(result.unwrap().logical_manifest, manifest, "{flavor}");
            } else {
                assert_eq!(
                    result.unwrap_err().error_code,
                    "SOURCE_ARCHIVE_UNDECLARED_ENTRY",
                    "{flavor}"
                );
            }
        }
    }
}

#[tokio::test]
async fn regression_rejects_duplicate_pax_override_keys_independently_of_chunking() {
    for key in ["path", "linkpath"] {
        for identical in [false, true] {
            let target = "verified-target";
            let override_value = if key == "path" { "alias" } else { target };
            let first = if identical {
                override_value
            } else if key == "path" {
                "restored-alias"
            } else {
                "restored-target"
            };
            let regular = b"regular";
            let verified = b"verified";
            let restored = b"restored";
            let alias = if key == "path" {
                json!({"path":"alias","sha256":sha256_hex(regular),"size":regular.len(),"role":"compute"})
            } else {
                json!({"path":"alias","sha256":sha256_hex(target),"size":0,"role":"compute","entryType":"symlink","linkTarget":target})
            };
            let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[alias,
                {"path":target,"sha256":sha256_hex(verified),"size":verified.len(),"role":"compute"},
                {"path":"restored-target","sha256":sha256_hex(restored),"size":restored.len(),"role":"compute"}
            ]});
            let compressed = compressed_tar(&manifest, |archive| {
                tar_file(archive, target, verified, b'0');
                tar_file(archive, "restored-target", restored, b'0');
                let records = format!(
                    "{}{}",
                    pax_record(key, first),
                    pax_record(key, override_value)
                );
                tar_file(archive, "PaxHeader", records.as_bytes(), b'x');
                if key == "path" {
                    tar_file(archive, "header-only", regular, b'0');
                } else {
                    tar_symlink(archive, "alias", "header-only");
                }
                archive.extend_from_slice(&[0; 1024]);
            });
            let decoded = zstd::stream::decode_all(compressed.as_ref()).unwrap();
            let mut archive = tar::Archive::new(decoded.as_slice());
            let entry = archive.entries().unwrap().last().unwrap().unwrap();
            let consumer_value = if key == "path" {
                entry.path().unwrap().into_owned()
            } else {
                entry.link_name().unwrap().unwrap().into_owned()
            };
            assert_eq!(consumer_value.to_str(), Some(first));
            if key == "path" {
                assert_eq!(
                    restored_archive_path_bytes(&compressed, first).unwrap(),
                    regular
                );
            } else {
                #[cfg(unix)]
                assert_eq!(
                    restored_archive_path_bytes(&compressed, "alias").unwrap(),
                    if identical { verified } else { restored }
                );
            }
            for chunk_size in [1, 17, compressed.len()] {
                let input = input_for(manifest.clone(), &compressed);
                let chunks = compressed
                    .chunks(chunk_size)
                    .map(|chunk| Ok::<_, std::io::Error>(Bytes::copy_from_slice(chunk)))
                    .collect::<Vec<_>>();
                let error = verify_source_bundle_stream(input, stream::iter(chunks))
                    .await
                    .unwrap_err();
                assert_eq!(
                    error.error_code, "SOURCE_TAR_PAX_INVALID",
                    "{key}, identical={identical}, chunk={chunk_size}"
                );
            }
        }
    }
}

#[tokio::test]
async fn verifies_combined_pax_path_and_link_records() {
    let target = "real-α";
    let body = b"real";
    let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
        {"path":target, "sha256":sha256_hex(body), "size":body.len(), "role":"compute"},
        {"path":"alias", "sha256":sha256_hex(target), "size":0,
            "role":"compute", "entryType":"symlink", "linkTarget":target}
    ]});
    let compressed = compressed_tar(&manifest, |tar| {
        tar_file(tar, target, body, b'0');
        let records = format!(
            "{}{}",
            pax_record("path", "alias"),
            pax_record("linkpath", target)
        );
        tar_file(tar, "PaxHeader", records.as_bytes(), b'x');
        tar_symlink(tar, "header-alias", "header-target");
        tar.extend_from_slice(&[0; 512]);
    });

    let result = verify_source_bundle_bytes(input_for(manifest.clone(), &compressed), compressed)
        .await
        .unwrap();

    assert_eq!(result.logical_manifest, manifest);
    assert_eq!(result.summary.file_count, 2);
}

#[tokio::test]
async fn rejects_malformed_pax_records_before_archive_payloads() {
    for (record, code) in [
        (b"11path=abc\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b" path=abc\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"z path=abc\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"+12 path=abc\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"0 path=abc\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (
            b"9999999999999999999999999 path=x\n".as_slice(),
            "SOURCE_TAR_PAX_INVALID",
        ),
        (b"100 path=x\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"1 x\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"10 path=x?".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"10 path x\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"5 =x\n".as_slice(), "SOURCE_TAR_PAX_INVALID"),
        (b"10 p\xffth=x\n".as_slice(), "SOURCE_TAR_PAX_INVALID_UTF8"),
        (b"9 path=\xff\n".as_slice(), "SOURCE_TAR_PAX_INVALID_UTF8"),
    ] {
        let manifest = fixture_manifest();
        let compressed = compressed_tar(&manifest, |tar| {
            tar_file(tar, "PaxHeader", record, b'x');
            tar.extend_from_slice(&[0; 512]);
        });

        let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
            .await
            .unwrap_err();

        assert_eq!(error.error_code, code, "malformed PAX record: {record:?}");
    }
}

#[tokio::test]
async fn rejects_invalid_manifest_entry_metadata_before_missing_payloads() {
    for (entry, code) in [
        (
            json!({"path":"alias", "entryType":"file", "linkTarget":"real"}),
            "SOURCE_MANIFEST_LINK_INVALID",
        ),
        (
            json!({"path":"alias", "entryType":"symlink", "linkTarget":"real", "size":1}),
            "SOURCE_MANIFEST_LINK_INVALID",
        ),
        (
            json!({"path":"alias", "entryType":"symlink"}),
            "SOURCE_MANIFEST_LINK_INVALID",
        ),
        (
            json!({"path":"alias", "entryType":"symlink", "linkTarget":"../outside", "sha256":sha256_hex("../outside")}),
            "SOURCE_MANIFEST_LINK_TARGET_UNSAFE",
        ),
        (
            json!({"path":"alias", "entryType":"symlink", "linkTarget":"real", "sha256":sha256_hex("wrong")}),
            "SOURCE_MANIFEST_LINK_SHA_MISMATCH",
        ),
    ] {
        let mut file =
            json!({"path":"alias", "sha256":sha256_hex("real"), "size":0, "role":"compute"});
        for (key, value) in entry.as_object().unwrap() {
            file[key] = value.clone();
        }
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
            file, {"path":"real", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
        ]});
        let compressed = compressed_tar(&manifest, |tar| tar.extend_from_slice(&[0; 512]));

        let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
            .await
            .unwrap_err();

        assert_eq!(error.error_code, code);
    }
}

#[tokio::test]
async fn rejects_dangling_cyclic_and_nested_manifest_symlinks() {
    for (links, nested_file, code) in [
        (
            vec![("alias", "missing")],
            None,
            "SOURCE_MANIFEST_LINK_TARGET_MISSING",
        ),
        (
            vec![("alias", "other"), ("other", "alias")],
            None,
            "SOURCE_MANIFEST_LINK_TARGET_MISSING",
        ),
        (
            vec![("alias", "alias")],
            None,
            "SOURCE_MANIFEST_LINK_TARGET_MISSING",
        ),
        (
            vec![("alias", "directory")],
            Some("alias/child"),
            "SOURCE_MANIFEST_LINK_NESTED_ENTRY",
        ),
    ] {
        let mut files = links
            .into_iter()
            .map(|(path, target)| {
                json!({
                    "path":path, "sha256":sha256_hex(target), "size":0,
                    "role":"compute", "entryType":"symlink", "linkTarget":target
                })
            })
            .collect::<Vec<_>>();
        files.push(json!({"path":"directory/file", "sha256":sha256_hex("data"), "size":4, "role":"compute"}));
        if let Some(path) = nested_file {
            files.push(
                json!({"path":path, "sha256":sha256_hex("data"), "size":4, "role":"compute"}),
            );
        }
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":files});
        let compressed = compressed_tar(&manifest, |tar| tar.extend_from_slice(&[0; 512]));

        let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
            .await
            .unwrap_err();

        assert_eq!(error.error_code, code);
    }
}

#[tokio::test]
async fn accepts_empty_entries_and_ignores_regular_file_linkname_bytes() {
    let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
        {"path":"empty", "sha256":sha256_hex(""), "size":0, "role":"compute"},
        {"path":"real", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
    ]});
    let compressed = compressed_tar(&manifest, |tar| {
        tar_file(tar, "PaxHeader", b"", b'x');
        tar_file(tar, "empty", b"", b'0');
        tar_file(tar, ".__onreza/empty", b"", b'0');
        let offset = tar.len();
        tar_file(tar, "real", b"data", b'0');
        let mut header = tar::Header::from_byte_slice(&tar[offset..offset + 512]).clone();
        header.as_mut_bytes()[157] = 0xff;
        header.set_cksum();
        tar[offset..offset + 512].copy_from_slice(header.as_bytes());
        tar.extend_from_slice(&[0; 1024]);
    });
    assert_eq!(
        restored_archive_path_bytes(&compressed, "empty").unwrap(),
        b""
    );
    assert_eq!(
        restored_archive_path_bytes(&compressed, "real").unwrap(),
        b"data"
    );
    let decoded = zstd::stream::decode_all(compressed.as_ref()).unwrap();
    let compressed = Bytes::from(
        decoded
            .chunks(512)
            .flat_map(|block| zstd::bulk::compress(block, 3).unwrap())
            .collect::<Vec<_>>(),
    );
    for chunk_size in [1, 17, 65_536] {
        let input = input_for(manifest.clone(), &compressed);
        let chunks = compressed
            .chunks(chunk_size)
            .map(|chunk| Ok::<_, std::io::Error>(Bytes::copy_from_slice(chunk)))
            .collect::<Vec<_>>();
        let verified = verify_source_bundle_stream(input, stream::iter(chunks))
            .await
            .unwrap();
        assert_eq!(verified.logical_manifest, manifest);
        assert_eq!(verified.summary.file_count, 2);
    }
}

#[tokio::test]
async fn accepts_empty_and_nonempty_auxiliary_metadata_before_payloads() {
    let manifest = fixture_manifest();
    let compressed = compressed_tar(&manifest, |tar| {
        tar_file(tar, ".__onreza/empty", b"", b'0');
        tar_file(tar, ".__onreza/context", b"auxiliary metadata", b'0');
        tar_file(tar, "dist/index.html", b"<h1>ONREZA</h1>\n", b'0');
        tar_file(tar, "server/index.js", b"export default fetch;\n", b'0');
        tar.extend_from_slice(&[0; 1024]);
    });
    let result = verify_source_bundle_bytes(input_for(manifest.clone(), &compressed), compressed)
        .await
        .unwrap();
    assert_eq!(result.logical_manifest, manifest);
    assert_eq!(result.summary.file_count, 2);
}

#[tokio::test]
async fn regression_symlink_expansion_limits_match_restored_linux_paths() {
    let mut unreadable_admissions = Vec::new();
    for (links, final_path, readable) in [
        (
            (0..40)
                .map(|index| {
                    (
                        format!("a{index}"),
                        if index == 0 {
                            "real".to_string()
                        } else {
                            format!("a{}", index - 1)
                        },
                    )
                })
                .collect::<Vec<_>>(),
            "a39",
            true,
        ),
        (
            (0..41)
                .map(|index| {
                    (
                        format!("a{index}"),
                        if index == 0 {
                            "real".to_string()
                        } else {
                            format!("a{}", index - 1)
                        },
                    )
                })
                .collect(),
            "a40",
            false,
        ),
        (
            (0..6)
                .map(|index| {
                    (
                        format!("a{index}"),
                        if index == 0 {
                            "dir".to_string()
                        } else {
                            format!("a{}/../a{}", index - 1, index - 1)
                        },
                    )
                })
                .collect(),
            "a5/file",
            false,
        ),
    ] {
        let real_path = if final_path.contains('/') {
            "dir/file"
        } else {
            "real"
        };
        let mut files = links.iter().map(|(path, target)| json!({"path":path, "sha256":sha256_hex(target), "size":0, "role":"compute", "entryType":"symlink", "linkTarget":target})).collect::<Vec<_>>();
        files.push(
            json!({"path":real_path, "sha256":sha256_hex("data"), "size":4, "role":"compute"}),
        );
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":files});
        let compressed = compressed_tar(&manifest, |tar| {
            tar_file(tar, real_path, b"data", b'0');
            for (path, target) in &links {
                tar_symlink(tar, path, target);
            }
            tar.extend_from_slice(&[0; 1024]);
        });
        let restored = restored_archive_path_bytes(&compressed, final_path);
        let verified =
            verify_source_bundle_bytes(input_for(manifest, &compressed), compressed).await;
        if readable {
            assert_eq!(restored.unwrap(), b"data");
            assert_eq!(verified.unwrap().summary.file_count, 41);
        } else {
            assert_eq!(restored.unwrap_err().raw_os_error(), Some(40));
            match verified {
                Ok(_) => unreadable_admissions.push(final_path),
                Err(error) => assert_eq!(error.error_code, "SOURCE_MANIFEST_LINK_TARGET_UNSAFE"),
            }
        }
    }
    assert!(
        unreadable_admissions.is_empty(),
        "Admitted unreadable paths: {unreadable_admissions:?}"
    );
}

#[tokio::test]
async fn graph_rejects_invalid_raw_targets_and_physical_root_escape() {
    for target in [
        "",
        "/real",
        "bad\\target",
        "bad\0target",
        ".",
        "real/",
        "real/..",
        "directory-alias/../../real",
    ] {
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
            {"path":"alias", "sha256":sha256_hex(target), "size":0, "role":"compute", "entryType":"symlink", "linkTarget":target},
            {"path":"directory-alias", "sha256":sha256_hex("dir"), "size":0, "role":"compute", "entryType":"symlink", "linkTarget":"dir"},
            {"path":"dir/file", "sha256":sha256_hex("data"), "size":4, "role":"compute"},
            {"path":"real", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
        ]});
        let compressed = compressed_tar(&manifest, |tar| tar.extend_from_slice(&[0; 512]));
        let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
            .await
            .unwrap_err();
        assert_eq!(
            error.error_code, "SOURCE_MANIFEST_LINK_TARGET_UNSAFE",
            "target: {target:?}"
        );
    }
}

#[tokio::test]
async fn archive_truncation_reports_incomplete_header_body_and_padding() {
    let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
        {"path":"real", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
    ]});
    for kept in [511, 515, 516, 1023, 1024] {
        let compressed = compressed_tar(&manifest, |tar| {
            let mut entry = Vec::new();
            tar_file(&mut entry, "real", b"data", b'0');
            tar.extend_from_slice(&entry[..kept]);
        });
        let error =
            verify_source_bundle_bytes(input_for(manifest.clone(), &compressed), compressed)
                .await
                .unwrap_err();
        assert_eq!(
            error.error_code, "SOURCE_TAR_TRUNCATED",
            "kept {kept} bytes"
        );
    }
}

#[tokio::test]
async fn manifest_header_admission_preserves_the_inclusive_metadata_size_limit() {
    for (size, code) in [
        (512 * 1024 * 1024, "SOURCE_TAR_TRUNCATED"),
        (512 * 1024 * 1024 + 1, "SOURCE_LOGICAL_MANIFEST_TOO_LARGE"),
    ] {
        let compressed = compressed_tar_without_manifest(|tar| {
            let mut header = tar::Header::new_ustar();
            header
                .set_path(SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH)
                .unwrap();
            header.set_mode(0o644);
            header.set_size(size);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            tar.extend_from_slice(header.as_bytes());
        });
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[]});
        let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
            .await
            .unwrap_err();
        assert_eq!(error.error_code, code);
    }
}

#[tokio::test]
async fn rejects_symlink_headers_with_payload_bytes() {
    let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
        {"path":"alias", "sha256":sha256_hex("real"), "size":0,
            "role":"compute", "entryType":"symlink", "linkTarget":"real"},
        {"path":"real", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
    ]});
    let compressed = compressed_tar(&manifest, |tar| {
        let mut header = tar::Header::new_ustar();
        header.set_path("alias").unwrap();
        header.set_mode(0o777);
        header.set_size(1);
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_link_name("real").unwrap();
        header.set_cksum();
        tar.extend_from_slice(header.as_bytes());
        tar.extend_from_slice(&[0; 1024]);
    });
    let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
        .await
        .unwrap_err();
    assert_eq!(error.error_code, "SOURCE_TAR_SYMLINK_INVALID");
}

#[tokio::test]
async fn symlink_resolution_follows_directory_aliases_before_parent_components() {
    for (target, real_path, directory_path, directory_target) in [
        (
            "directory-alias/../directory-alias/file",
            "dir/file",
            "dir/child",
            "dir",
        ),
        (
            "directory-alias/../real",
            "dir/real",
            "dir/child/file",
            "dir/child",
        ),
        (
            "directory-alias/../../real",
            "dir/real",
            "dir/deeper/child/file",
            "dir/deeper/child",
        ),
    ] {
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
            {"path":"alias", "sha256":sha256_hex(target), "size":0, "role":"compute", "entryType":"symlink", "linkTarget":target},
            {"path":"directory-alias", "sha256":sha256_hex(directory_target), "size":0, "role":"compute", "entryType":"symlink", "linkTarget":directory_target},
            {"path":real_path, "sha256":sha256_hex("data"), "size":4, "role":"compute"},
            {"path":directory_path, "sha256":sha256_hex("child"), "size":5, "role":"compute"}
        ]});
        let compressed = compressed_tar(&manifest, |tar| {
            tar_file(tar, real_path, b"data", b'0');
            tar_file(tar, directory_path, b"child", b'0');
            tar_symlink(tar, "directory-alias", directory_target);
            tar_symlink(tar, "alias", target);
            tar.extend_from_slice(&[0; 1024]);
        });
        assert_eq!(
            restored_archive_path_bytes(&compressed, "alias").unwrap(),
            b"data"
        );
        let verified = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
            .await
            .unwrap();
        assert_eq!(verified.summary.file_count, 4);
        assert_eq!(verified.summary.artifact_size_bytes, 9);
    }
}

#[tokio::test]
async fn regression_rejects_symlink_traversal_through_missing_directories() {
    let target = "missing/../real";
    let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
        {"path":"alias", "sha256":sha256_hex(target), "size":0,
            "role":"compute", "entryType":"symlink", "linkTarget":target},
        {"path":"real", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
    ]});
    let compressed = compressed_tar(&manifest, |tar| {
        tar_file(tar, "real", b"data", b'0');
        tar_symlink(tar, "alias", target);
        tar.extend_from_slice(&[0; 1024]);
    });
    assert_eq!(
        restored_archive_path_bytes(&compressed, "alias")
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
        .await
        .unwrap_err();
    assert_eq!(error.error_code, "SOURCE_MANIFEST_LINK_TARGET_MISSING");
}

#[tokio::test]
async fn regression_rejects_regular_files_with_nested_archive_entries() {
    let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
        {"path":"foo", "sha256":sha256_hex("data"), "size":4, "role":"compute"},
        {"path":"foo/bar", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
    ]});
    let compressed = bundle(&manifest, &[("foo", b"data"), ("foo/bar", b"data")]);
    let extracted = tempfile::tempdir().unwrap();
    let decoded = zstd::stream::decode_all(compressed.as_ref()).unwrap();
    assert!(
        tar::Archive::new(decoded.as_slice())
            .unpack(extracted.path())
            .is_err()
    );
    let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
        .await
        .unwrap_err();
    assert_eq!(error.error_code, "SOURCE_MANIFEST_PATH_INVALID");
}

#[tokio::test]
async fn rejects_symlink_target_traversal_through_a_file_alias() {
    let target = "dir/file-link/../real";
    let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":[
        {"path":"alias", "sha256":sha256_hex(target), "size":0,
            "role":"compute", "entryType":"symlink", "linkTarget":target},
        {"path":"dir/file-link", "sha256":sha256_hex("real"), "size":0,
            "role":"compute", "entryType":"symlink", "linkTarget":"real"},
        {"path":"dir/real", "sha256":sha256_hex("data"), "size":4, "role":"compute"}
    ]});
    let compressed = compressed_tar(&manifest, |tar| tar.extend_from_slice(&[0; 512]));

    let error = verify_source_bundle_bytes(input_for(manifest, &compressed), compressed)
        .await
        .unwrap_err();

    assert_eq!(error.error_code, "SOURCE_MANIFEST_LINK_TARGET_UNSAFE");
}

fn fixture_bundle(manifest: &Value) -> Bytes {
    bundle(
        manifest,
        &[
            ("dist/index.html", b"<h1>ONREZA</h1>\n".as_slice()),
            ("server/index.js", b"export default fetch;\n".as_slice()),
        ],
    )
}

#[tokio::test]
async fn archive_expansion_budget_accounts_for_padding_and_pax_framing() {
    let metadata = vec![0; 32 * 1024 * 1024 - 512];
    for (path_length, body_size, symlink) in [
        (100, 0, false),
        (513, 513, false),
        (100, 1, true),
        (101, 512, true),
    ] {
        let path = "x".repeat(path_length);
        let body = vec![0; body_size];
        let mut files = vec![json!({
            "path":path, "sha256":sha256_hex(&body), "size":body.len(), "role":"compute"
        })];
        if symlink {
            files.push(json!({
                "path":"alias", "sha256":sha256_hex(&path), "size":0,
                "role":"compute", "entryType":"symlink", "linkTarget":path
            }));
        }
        let manifest = json!({"schemaVersion":"SOURCE_BUNDLE_V1.0", "files":files});
        for extra_padding in [0, 512] {
            let compressed = compressed_tar(&manifest, |tar| {
                let header_path = if path_length > 100 {
                    pax_header(tar, "path", &path);
                    "PaxFile"
                } else {
                    &path
                };
                tar_file(tar, header_path, &body, b'0');
                if symlink {
                    let header_target = if path_length > 100 {
                        pax_header(tar, "linkpath", &path);
                        "PaxTarget"
                    } else {
                        &path
                    };
                    tar_symlink(tar, "alias", header_target);
                }
                tar_file(tar, ".__onreza/metadata", &metadata, b'0');
                tar.extend(std::iter::repeat_n(0, 1024 + extra_padding));
            });
            let result =
                verify_source_bundle_bytes(input_for(manifest.clone(), &compressed), compressed)
                    .await;
            if extra_padding == 0 {
                assert_eq!(result.unwrap().logical_manifest, manifest);
            } else {
                assert_eq!(result.unwrap_err().error_code, "SOURCE_EXPANSION_LIMIT");
            }
        }
    }
}

fn restored_archive_path_bytes(compressed: &Bytes, path: &str) -> std::io::Result<Vec<u8>> {
    let extracted = tempfile::tempdir().unwrap();
    let decoded = zstd::stream::decode_all(compressed.as_ref()).unwrap();
    tar::Archive::new(decoded.as_slice())
        .unpack(extracted.path())
        .unwrap();
    std::fs::read(extracted.path().join(path))
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
    let budget =
        SourceBundleVerificationBudget::from_manifest(&serde_json::from_value(manifest).unwrap())
            .unwrap();
    SourceBundleVerificationInput {
        budget,
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
    let mut header = tar::Header::new_ustar();
    header.set_path(path).unwrap();
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_entry_type(tar::EntryType::new(typeflag));
    header.set_cksum();
    tar.extend_from_slice(header.as_bytes());
    if typeflag == b'0' || typeflag == 0 || typeflag == b'x' {
        tar.extend_from_slice(body);
        let padding = (512 - (body.len() % 512)) % 512;
        tar.extend(std::iter::repeat_n(0, padding));
    }
}

fn tar_symlink(tar: &mut Vec<u8>, path: &str, target: &str) {
    let mut header = tar::Header::new_ustar();
    header.set_path(path).unwrap();
    header.set_link_name(target).unwrap();
    header.set_mode(0o777);
    header.set_size(0);
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_cksum();
    tar.extend_from_slice(header.as_bytes());
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
