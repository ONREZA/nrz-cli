use super::*;
use nrz_source_bundle::source_bundle_v2::{
    SourceBundleV2ManifestBudget, admission::ManifestMemoryPool, encode_source_bundle_v2_manifest,
};
use serde_json::{Value, json};
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};

const LIMITS: PreparationLimits = PreparationLimits {
    max_file_bytes: 1024 * 1024,
    max_nodes: 100,
    max_path_bytes: 1024,
};

fn regular(path: &str, bytes: &[u8]) -> Value {
    json!({"path":path, "entryType":"file", "role":"static",
        "blobDigest":{"algorithm":"BLAKE3", "value":blake3::hash(bytes).to_hex().as_str()},
        "size":bytes.len(), "executable":false})
}

fn link(path: &str, target: &str) -> Value {
    json!({"path":path, "entryType":"symlink", "role":"static",
        "linkTarget":target, "size":0})
}

fn admit(mut files: Vec<Value>, pool: &ManifestMemoryPool) -> AdmittedSourceBundleV2Manifest {
    files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let compute = files.iter().any(|file| file.get("layerName").is_some());
    let (capabilities, layers, entrypoints) = if compute {
        (
            json!(["COMPUTE_NETWORK_V1"]),
            json!([{
                "name":"server", "target":"COMPUTE", "rootPath":"app",
                "entrypoint":"app/main.js", "runtimeConfig":{
                    "applicationRuntime":{"family":"BUN", "args":[]},
                    "buildRuntimeVersion":"bun-1.4.2", "memoryMb":256,
                    "network":nrz_source_bundle::ComputeNetwork::public_default()
                }
            }]),
            json!(["app/main.js"]),
        )
    } else {
        (json!([]), json!([]), json!([]))
    };
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: 100,
        max_logical_bytes: LIMITS.max_file_bytes,
        max_static_file_bytes: LIMITS.max_file_bytes,
    };
    let routes = if compute {
        json!([{"pattern":"/", "layerName":"server"}])
    } else {
        json!([])
    };
    let bytes = encode_source_bundle_v2_manifest(
        json!({
            "schemaVersion":"SOURCE_BUNDLE_V2.0", "capabilities":capabilities,
            "files":files, "layers":layers, "routes":routes, "entrypoints":entrypoints,
        }),
        budget,
    )
    .unwrap();
    let mut input = pool.reserve_input(bytes.len()).unwrap();
    input.append(&bytes).unwrap();
    input.verify(budget, u64::MAX).unwrap()
}

fn pool() -> ManifestMemoryPool {
    ManifestMemoryPool::new(64 * 1024 * 1024)
}

fn copied(prepared: &PreparedSourceV2, path: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    prepared
        .open_file(path)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

fn empty_staging(staging: &Path) {
    assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
}

#[test]
fn copies_new_inodes_streams_and_preserves_content_after_retained_writer_mutation() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let bytes = vec![b'x'; COPY_BUFFER_BYTES * 2 + 17];
    fs::write(input.path().join("file"), &bytes).unwrap();
    let mut producer = File::options()
        .write(true)
        .open(input.path().join("file"))
        .unwrap();
    let prepared = prepare_source_v2(
        admit(vec![regular("file", &bytes)], &pool()),
        &ArtifactRoot::open(input.path()).unwrap(),
        staging.path(),
        LIMITS,
    )
    .unwrap();
    let original = producer.metadata().unwrap();
    let target = prepared
        .root
        .open_file(Path::new("file"))
        .unwrap()
        .metadata()
        .unwrap();
    assert_ne!(
        (original.dev(), original.ino()),
        (target.dev(), target.ino())
    );
    assert_eq!(target.mode() & 0o777, 0o775);
    assert_eq!(
        fs::metadata(prepared._staging.path()).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(prepared._staging.path().join("tree"))
            .unwrap()
            .mode()
            & 0o777,
        0o775
    );
    producer.write_all(b"changed").unwrap();
    producer.set_len(7).unwrap();
    assert_eq!(copied(&prepared, "file"), bytes);
    let mut reader = prepared.open_file("file").unwrap();
    reader.seek(SeekFrom::End(-1)).unwrap();
    let mut last = [0];
    reader.read_exact(&mut last).unwrap();
    assert_eq!(last, [b'x']);
}

#[test]
fn rejects_digest_short_and_long_files_and_cleans_partial_snapshot() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let root = ArtifactRoot::open(input.path()).unwrap();
    for (actual, expected_reason) in [
        (b"xy".as_slice(), "BLAKE3 mismatch"),
        (b"x", "short file"),
        (b"xyz", "long file"),
    ] {
        fs::write(input.path().join("file"), actual).unwrap();
        assert!(matches!(
            prepare_source_v2(admit(vec![regular("file", b"ok")], &pool()), &root, staging.path(), LIMITS),
            Err(PreparationError::Content { reason, .. }) if reason == expected_reason
        ));
        empty_staging(staging.path());
    }
    fs::write(input.path().join("file"), []).unwrap();
    let prepared = prepare_source_v2(
        admit(vec![regular("file", b"")], &pool()),
        &root,
        staging.path(),
        LIMITS,
    )
    .unwrap();
    assert_eq!(copied(&prepared, "file"), b"");
}

#[test]
fn refuses_final_symlinks_directories_and_fifo_without_blocking() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let root = ArtifactRoot::open(input.path()).unwrap();
    fs::write(input.path().join("target"), b"ok").unwrap();
    symlink("target", input.path().join("file")).unwrap();
    assert!(
        prepare_source_v2(
            admit(vec![regular("file", b"ok")], &pool()),
            &root,
            staging.path(),
            LIMITS
        )
        .is_err()
    );
    fs::remove_file(input.path().join("file")).unwrap();
    fs::create_dir(input.path().join("file")).unwrap();
    assert!(
        prepare_source_v2(
            admit(vec![regular("file", b"ok")], &pool()),
            &root,
            staging.path(),
            LIMITS
        )
        .is_err()
    );
    fs::remove_dir(input.path().join("file")).unwrap();
    let fifo = std::ffi::CString::new(input.path().join("file").as_os_str().as_bytes()).unwrap();
    // SAFETY: fifo is a valid NUL-terminated path for the duration of this call.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(
        prepare_source_v2(
            admit(vec![regular("file", b"ok")], &pool()),
            &root,
            staging.path(),
            LIMITS
        )
        .is_err()
    );
    empty_staging(staging.path());
}

#[test]
fn raw_link_target_must_match_even_when_alternate_spelling_resolves_to_same_file() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    fs::write(input.path().join("file"), b"ok").unwrap();
    symlink("./file", input.path().join("alias")).unwrap();
    assert!(matches!(
        prepare_source_v2(
            admit(vec![link("alias", "file"), regular("file", b"ok")], &pool()),
            &ArtifactRoot::open(input.path()).unwrap(),
            staging.path(),
            LIMITS,
        ),
        Err(PreparationError::Content {
            reason: "symlink target mismatch",
            ..
        })
    ));
    empty_staging(staging.path());
}

#[test]
fn acquired_root_survives_path_rebinding_and_rejects_escaping_ancestors() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let source = input.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("file"), b"ok").unwrap();
    let root = ArtifactRoot::open(&source).unwrap();
    fs::rename(&source, input.path().join("held")).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(source.join("file"), b"no").unwrap();
    let prepared = prepare_source_v2(
        admit(vec![regular("file", b"ok")], &pool()),
        &root,
        staging.path(),
        LIMITS,
    )
    .unwrap();
    assert_eq!(copied(&prepared, "file"), b"ok");
    drop(prepared);
    symlink(&source, input.path().join("held/escape")).unwrap();
    assert!(
        prepare_source_v2(
            admit(vec![regular("escape/file", b"no")], &pool()),
            &root,
            staging.path(),
            LIMITS
        )
        .is_err()
    );
    empty_staging(staging.path());
}

#[test]
fn combined_snapshot_keeps_all_roles_cross_role_links_and_ignores_undeclared_extras() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    fs::create_dir(input.path().join("app")).unwrap();
    fs::create_dir(input.path().join("deps")).unwrap();
    fs::write(input.path().join("app/main.js"), b"main").unwrap();
    fs::write(input.path().join("deps/library"), b"lib").unwrap();
    fs::write(input.path().join("page"), b"page").unwrap();
    fs::write(input.path().join("extra"), b"unlisted").unwrap();
    symlink("../page", input.path().join("app/page")).unwrap();
    let mut main = regular("app/main.js", b"main");
    main["role"] = json!("compute");
    main["layerName"] = json!("server");
    let mut alias = link("app/page", "../page");
    alias["role"] = json!("compute");
    alias["layerName"] = json!("server");
    let mut dependency = regular("deps/library", b"lib");
    dependency["role"] = json!("dependency");
    dependency["layerName"] = json!("server");
    let prepared = prepare_source_v2(
        admit(
            vec![main, alias, dependency, regular("page", b"page")],
            &pool(),
        ),
        &ArtifactRoot::open(input.path()).unwrap(),
        staging.path(),
        LIMITS,
    )
    .unwrap();
    assert_eq!(copied(&prepared, "app/main.js"), b"main");
    assert_eq!(copied(&prepared, "deps/library"), b"lib");
    assert_eq!(prepared.root.read(Path::new("page")).unwrap(), b"page");
    assert_eq!(
        prepared.root.read_link(Path::new("app/page")).unwrap(),
        Path::new("../page")
    );
    assert!(!prepared._staging.path().join("tree/extra").exists());
    for denied in ["extra", "app/page", "app", "../page", "/page"] {
        assert!(matches!(
            prepared.open_file(denied),
            Err(PreparationError::NotDeclaredFile)
        ));
    }
}

#[test]
fn exact_limits_count_shared_implicit_directories_once_and_deny_before_creating_output() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let paths = ["a/b/one", "a/b/two", "a/c/three", "ab/four"];
    for path in paths {
        fs::create_dir_all(input.path().join(path).parent().unwrap()).unwrap();
        fs::write(input.path().join(path), b"ok").unwrap();
    }
    let files = || paths.into_iter().map(|path| regular(path, b"ok")).collect();
    let root = ArtifactRoot::open(input.path()).unwrap();
    // Two roots, four implicit directories (a, a/b, a/c, ab), four files.
    let exact = PreparationLimits {
        max_file_bytes: 8,
        max_nodes: 10,
        max_path_bytes: 9,
    };
    for refused in [
        PreparationLimits {
            max_file_bytes: 7,
            ..exact
        },
        PreparationLimits {
            max_nodes: 9,
            ..exact
        },
        PreparationLimits {
            max_path_bytes: 8,
            ..exact
        },
    ] {
        assert!(matches!(
            prepare_source_v2(admit(files(), &pool()), &root, staging.path(), refused),
            Err(PreparationError::Limit(_))
        ));
        empty_staging(staging.path());
    }
    let prepared =
        prepare_source_v2(admit(files(), &pool()), &root, staging.path(), exact).unwrap();
    assert_eq!(
        prepared.usage(),
        PreparationUsage {
            file_bytes: 8,
            nodes: 10
        }
    );
    assert_eq!(
        fs::metadata(prepared._staging.path().join("tree/a/b"))
            .unwrap()
            .mode()
            & 0o777,
        0o775
    );
}

#[test]
fn link_target_bytes_and_zero_limits_are_real_preflight_limits() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let root = ArtifactRoot::open(input.path()).unwrap();
    let files = || vec![link("a", "./././z"), regular("z", b"")];
    assert!(matches!(
        prepare_source_v2(
            admit(files(), &pool()),
            &root,
            staging.path(),
            PreparationLimits {
                max_path_bytes: 6,
                ..LIMITS
            }
        ),
        Err(PreparationError::Limit("symlink target byte limit"))
    ));
    for max_nodes in [0, 1] {
        assert!(matches!(
            prepare_source_v2(
                admit(vec![], &pool()),
                &root,
                staging.path(),
                PreparationLimits {
                    max_nodes,
                    ..LIMITS
                }
            ),
            Err(PreparationError::Limit("node limit"))
        ));
    }
    empty_staging(staging.path());
    let prepared = prepare_source_v2(
        admit(vec![], &pool()),
        &root,
        staging.path(),
        PreparationLimits {
            max_file_bytes: 0,
            max_nodes: 2,
            max_path_bytes: 0,
        },
    )
    .unwrap();
    assert_eq!(
        prepared.usage(),
        PreparationUsage {
            file_bytes: 0,
            nodes: 2
        }
    );
}

#[test]
fn prepared_owner_retains_manifest_capacity_and_releases_snapshot_on_drop_and_failure() {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    fs::write(input.path().join("file"), b"ok").unwrap();
    let root = ArtifactRoot::open(input.path()).unwrap();
    let pool = pool();
    let admitted = admit(vec![regular("file", b"ok")], &pool);
    let reserved = pool.reserved_bytes();
    let prepared = prepare_source_v2(admitted, &root, staging.path(), LIMITS).unwrap();
    assert_eq!(pool.reserved_bytes(), reserved);
    assert_eq!(prepared.manifest().summary().file_count, 1);
    let reader = prepared.open_file("file").unwrap();
    assert_eq!(pool.reserved_bytes(), reserved);
    drop(reader);
    drop(prepared);
    assert_eq!(pool.reserved_bytes(), 0);
    empty_staging(staging.path());
    fs::write(input.path().join("file"), b"no").unwrap();
    assert!(
        prepare_source_v2(
            admit(vec![regular("file", b"ok")], &pool),
            &root,
            staging.path(),
            LIMITS
        )
        .is_err()
    );
    assert_eq!(pool.reserved_bytes(), 0);
    empty_staging(staging.path());
}
