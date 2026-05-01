use std::fs;

use sha2::{Digest, Sha256};
use tempfile::tempdir;

use super::FileEntry;
use super::pack_v1::*;

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn fe(path: &str, bytes: &[u8]) -> FileEntry {
    FileEntry {
        path: path.into(),
        size: bytes.len() as u64,
        content_hash: sha(bytes),
    }
}

fn manifest(json: &str) -> crate::build::manifest::Manifest {
    serde_json::from_str(json).unwrap()
}

#[test]
fn static_pack_plan_materializes_pack_parts_and_summary() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("index.html"), b"hello").unwrap();
    fs::write(dir.path().join("assets/app.js"), b"app").unwrap();

    let files = vec![fe("index.html", b"hello"), fe("assets/app.js", b"app")];
    let plan = build_static_pack_plan(dir.path(), &files).unwrap();

    assert_eq!(plan.summary.file_count, 2);
    assert_eq!(plan.summary.total_logical_bytes, "8");
    assert_eq!(plan.summary.pack_parts.len(), 1);
    assert_eq!(plan.parts[0].size, 8);
    assert_eq!(plan.summary.paths[0].path, "assets/app.js");
    assert_eq!(plan.summary.paths[1].path, "index.html");
}

#[test]
fn static_pack_plan_materializes_bytes_in_canonical_path_order() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();

    let files = vec![fe("b.txt", b"b"), fe("a.txt", b"a")];
    let plan = build_static_pack_plan(dir.path(), &files).unwrap();

    assert_eq!(plan.parts.len(), 1);
    assert_eq!(plan.parts[0].sha256, sha(b"ab"));

    let a = plan
        .summary
        .paths
        .iter()
        .find(|p| p.path == "a.txt")
        .unwrap();
    let b = plan
        .summary
        .paths
        .iter()
        .find(|p| p.path == "b.txt")
        .unwrap();

    assert_eq!((a.part_index, a.offset, a.length), (0, 0, 1));
    assert_eq!((b.part_index, b.offset, b.length), (0, 1, 1));
}

#[test]
fn static_pack_plan_empty_static_layer_still_declares_empty_part() {
    let dir = tempdir().unwrap();
    let plan = build_static_pack_plan(dir.path(), &[]).unwrap();

    assert_eq!(plan.summary.file_count, 0);
    assert_eq!(plan.summary.pack_parts.len(), 1);
    assert_eq!(plan.summary.pack_parts[0].size, 0);
    assert_eq!(
        plan.summary.pack_parts[0].sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn static_pack_plan_deduplicates_identical_file_ranges() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("copy")).unwrap();
    fs::write(dir.path().join("a.txt"), b"same").unwrap();
    fs::write(dir.path().join("copy/b.txt"), b"same").unwrap();
    fs::write(dir.path().join("c.txt"), b"different").unwrap();

    let files = vec![
        fe("a.txt", b"same"),
        fe("copy/b.txt", b"same"),
        fe("c.txt", b"different"),
    ];
    let plan = build_static_pack_plan(dir.path(), &files).unwrap();

    assert_eq!(plan.summary.file_count, 3);
    assert_eq!(plan.total_logical_bytes, 17);
    assert_eq!(plan.parts.len(), 1);
    assert_eq!(plan.parts[0].size, 13);
    assert_eq!(plan.summary.pack_parts[0].size, 13);

    let a = plan
        .summary
        .paths
        .iter()
        .find(|p| p.path == "a.txt")
        .unwrap();
    let b = plan
        .summary
        .paths
        .iter()
        .find(|p| p.path == "copy/b.txt")
        .unwrap();
    let c = plan
        .summary
        .paths
        .iter()
        .find(|p| p.path == "c.txt")
        .unwrap();

    assert_eq!((a.part_index, a.offset, a.length), (0, 0, 4));
    assert_eq!((b.part_index, b.offset, b.length), (0, 0, 4));
    assert_eq!((c.part_index, c.offset, c.length), (0, 4, 9));
    assert_eq!(plan.summary.pack_parts[0].sha256, sha(b"samedifferent"));
}

#[test]
fn pack_part_count_limit_matches_server_manifest_codec() {
    assert!(ensure_pack_part_count(MAX_PACK_PARTS_PER_MANIFEST).is_ok());

    let err = ensure_pack_part_count(MAX_PACK_PARTS_PER_MANIFEST + 1).unwrap_err();
    assert!(err.to_string().contains("999"), "{err}");
}

#[test]
fn multipart_chunks_for_bytes_uses_offsets_sizes_and_sha256() {
    let bytes = b"abcdef";
    let chunks = multipart_chunks_for_bytes(bytes, 2).unwrap();

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].part_number, 1);
    assert_eq!(chunks[0].offset, 0);
    assert_eq!(chunks[0].size, 2);
    assert_eq!(chunks[0].sha256, sha(b"ab"));
    assert_eq!(chunks[2].offset, 4);
    assert_eq!(chunks[2].sha256, sha(b"ef"));
}

#[test]
fn static_layer_dirs_and_files_in_dirs_select_only_static_files() {
    let manifest = manifest(
        r#"{
          "version": 1,
          "layers": [
            { "name": "cdn", "target": "STATIC", "directory": "public" },
            { "name": "srv", "target": "COMPUTE", "directory": ".", "entry": "server.js" }
          ],
          "routes": []
        }"#,
    );
    let dirs = static_layer_dirs(&manifest);
    let files = vec![
        fe("public/app.css", b"css"),
        fe("server.js", b"server"),
        fe("node_modules/pkg/index.js", b"dep"),
    ];

    let filtered = files_in_dirs(&files, &dirs);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].path, "public/app.css");
}

#[test]
fn isolate_upload_plan_uses_layer_relative_paths() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("worker/chunks")).unwrap();
    fs::write(dir.path().join("worker/main.js"), b"main").unwrap();
    fs::write(dir.path().join("worker/chunks/a.js"), b"chunk").unwrap();
    let files = vec![
        fe("worker/main.js", b"main"),
        fe("worker/chunks/a.js", b"chunk"),
    ];
    let manifest = manifest(
        r#"{
          "version": 1,
          "layers": [
            {
              "name": "edge",
              "target": "ISOLATE",
              "directory": "worker",
              "entry": "main.js",
              "export": "fetch"
            }
          ],
          "routes": []
        }"#,
    );

    let plan = build_isolate_upload_plan(dir.path(), &manifest, &files).unwrap();
    assert_eq!(plan.modules.len(), 1);
    assert_eq!(plan.modules[0].files[0].path, "chunks/a.js");
    assert_eq!(plan.modules[0].files[1].path, "main.js");
    assert!(
        plan.local_path_for_target("edge", "main.js", &sha(b"main"))
            .is_some()
    );
}

#[test]
fn compute_bundle_uploads_are_declared_per_compute_layer() {
    let manifest = manifest(
        r#"{
          "version": 1,
          "layers": [
            { "name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js" }
          ],
          "routes": []
        }"#,
    );
    let bundle_sha = "a".repeat(64);
    let uploads =
        build_compute_bundle_uploads(&manifest, &bundle_sha, 123, Some(b"bundle")).unwrap();

    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].layer_name, "server");
    assert_eq!(uploads[0].bundle_sha256, bundle_sha);
    assert_eq!(uploads[0].size, "123");
}
