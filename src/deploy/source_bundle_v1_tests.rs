use std::fs;
use std::io::Cursor;
#[cfg(unix)]
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use tempfile::tempdir;

use super::source_bundle_v1::*;
use super::*;

fn static_manifest() -> crate::build::manifest::Manifest {
    serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "static", "target": "STATIC", "directory": "." }
        ],
        "routes": []
    }))
    .unwrap()
}

#[tokio::test]
async fn source_bundle_plan_is_deterministic_and_uses_identity_file_hashes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("b.txt"), b"b").unwrap();
    fs::write(dir.path().join("a.txt"), b"a").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let first = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let second = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    assert_eq!(first.source_sha256, second.source_sha256);
    assert_eq!(
        first.logical_manifest_sha256,
        second.logical_manifest_sha256
    );
    assert_eq!(first.logical_manifest.files[0].path, "a.txt");
    assert_eq!(first.logical_manifest.files[1].path, "b.txt");
    assert_eq!(
        first.logical_manifest.files[0].sha256,
        format!("{:x}", Sha256::digest(b"a"))
    );
    assert_eq!(
        first.logical_manifest.files[0].role,
        SourceLogicalManifestFileRole::Static
    );
    assert!(first.multipart.is_none());

    let compressed = first.read_all().await.unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&compressed)),
        first.source_sha256
    );
    assert_eq!(compressed.len() as u64, first.source_size_bytes);
}

#[tokio::test]
async fn source_bundle_embeds_canonical_logical_manifest_first() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), b"hello").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let compressed = plan.read_all().await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes));
    let mut entries = archive.entries().unwrap();
    let mut manifest_entry = entries.next().unwrap().unwrap();
    let mut manifest_body = String::new();
    std::io::Read::read_to_string(&mut manifest_entry, &mut manifest_body).unwrap();

    assert_eq!(
        manifest_entry.path().unwrap().to_string_lossy(),
        ".__onreza/logical-manifest.json"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(manifest_body.as_bytes())),
        plan.logical_manifest_sha256
    );
    assert_eq!(plan.logical_manifest_summary.file_count, 1);
    assert_eq!(plan.logical_manifest_summary.logical_static_bytes, "5");
}

#[test]
fn source_bundle_treats_header_and_redirect_files_as_static_content() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("_headers"), b"plain user file").unwrap();
    fs::write(dir.path().join("_redirects"), b"plain user file").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let headers = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "_headers")
        .unwrap();
    let redirects = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "_redirects")
        .unwrap();
    assert_eq!(headers.role, SourceLogicalManifestFileRole::Static);
    assert_eq!(redirects.role, SourceLogicalManifestFileRole::Static);
}

#[test]
fn source_bundle_plan_rejects_reserved_metadata_namespace() {
    let manifest = static_manifest();

    let root_file = tempdir().unwrap();
    fs::write(root_file.path().join(".__onreza"), b"user").unwrap();
    let files = scan_dir(root_file.path()).unwrap();
    let err = build_source_bundle_plan(root_file.path(), &manifest, &files).unwrap_err();
    assert!(
        err.to_string().contains("reserves metadata namespace"),
        "{err}"
    );

    let manifest_collision = tempdir().unwrap();
    fs::create_dir(manifest_collision.path().join(".__onreza")).unwrap();
    fs::write(
        manifest_collision
            .path()
            .join(".__onreza/logical-manifest.json"),
        b"user",
    )
    .unwrap();
    let files = scan_dir(manifest_collision.path()).unwrap();
    let err = build_source_bundle_plan(manifest_collision.path(), &manifest, &files).unwrap_err();
    assert!(
        err.to_string().contains("reserves metadata namespace"),
        "{err}"
    );
}

#[test]
fn logical_manifest_sha_uses_stable_key_ordering() {
    let left = SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_SCHEMA_VERSION.to_string(),
        capabilities: vec![],
        files: vec![SourceLogicalManifestFile {
            path: "index.html".into(),
            sha256: "a".repeat(64),
            size: 5,
            entry_type: None,
            link_target: None,
            content_type: Some("text/html; charset=utf-8".into()),
            role: SourceLogicalManifestFileRole::Static,
            layer_name: Some("static".into()),
        }],
        layers: vec![SourceLogicalManifestLayer {
            name: "static".into(),
            target: SourceLogicalManifestLayerTarget::Static,
            root_path: None,
            entrypoint: None,
            runtime_config: None,
        }],
        routes: vec![],
        entrypoints: vec![],
    };
    let mut right_value = serde_json::json!({
        "routes": [],
        "layers": [{ "target": "STATIC", "name": "static" }],
        "entrypoints": [],
        "files": [{
            "role": "static",
            "size": 5,
            "sha256": "a".repeat(64),
            "path": "index.html",
            "contentType": "text/html; charset=utf-8",
            "layerName": "static"
        }],
        "capabilities": [],
        "schemaVersion": "SOURCE_BUNDLE_V1.0"
    });
    let right: SourceLogicalManifest = serde_json::from_value(right_value.take()).unwrap();

    assert_eq!(
        compute_logical_manifest_sha256(&left).unwrap(),
        compute_logical_manifest_sha256(&right).unwrap()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_serializes_hardlinked_files_as_regular_files() {
    let dir = tempdir().unwrap();
    let platform_bin = dir
        .path()
        .join("node_modules/@esbuild/linux-x64/bin/esbuild");
    fs::create_dir_all(platform_bin.parent().unwrap()).unwrap();
    fs::write(&platform_bin, b"esbuild").unwrap();
    let package_bin = dir.path().join("node_modules/esbuild/bin/esbuild");
    fs::create_dir_all(package_bin.parent().unwrap()).unwrap();
    fs::hard_link(&platform_bin, &package_bin).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let package_manifest_entry = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/esbuild/bin/esbuild")
        .unwrap();
    assert_eq!(package_manifest_entry.entry_type, None);
    assert_eq!(package_manifest_entry.size, b"esbuild".len() as u64);

    let compressed = plan.read_all().await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes.as_slice()));
    let mut hardlinked_paths = Vec::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        if path == "node_modules/@esbuild/linux-x64/bin/esbuild"
            || path == "node_modules/esbuild/bin/esbuild"
        {
            assert!(entry.header().entry_type().is_file());
            hardlinked_paths.push(path);
        }
    }
    hardlinked_paths.sort();
    assert_eq!(
        hardlinked_paths,
        [
            "node_modules/@esbuild/linux-x64/bin/esbuild",
            "node_modules/esbuild/bin/esbuild"
        ]
    );

    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes.as_slice()))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read(extracted.path().join("node_modules/esbuild/bin/esbuild")).unwrap(),
        b"esbuild"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_preserves_safe_relative_symlinks() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("node_modules/.pnpm/pkg")).unwrap();
    fs::write(
        dir.path().join("node_modules/.pnpm/pkg/index.js"),
        b"module.exports = 1",
    )
    .unwrap();
    std::os::unix::fs::symlink(".pnpm/pkg", dir.path().join("node_modules/pkg")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let symlink = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg")
        .unwrap();
    assert_eq!(
        symlink.entry_type,
        Some(SourceLogicalManifestEntryType::Symlink)
    );
    assert_eq!(symlink.link_target.as_deref(), Some(".pnpm/pkg"));
    assert_eq!(symlink.size, 0);
    assert_eq!(
        symlink.sha256,
        format!("{:x}", Sha256::digest(b".pnpm/pkg"))
    );

    let compressed = plan.read_all().await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes));
    let mut saw_symlink = false;
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        if entry.path().unwrap().to_string_lossy() != "node_modules/pkg" {
            continue;
        }
        assert!(entry.header().entry_type().is_symlink());
        assert_eq!(
            entry.link_name().unwrap().unwrap(),
            PathBuf::from(".pnpm/pkg")
        );
        saw_symlink = true;
    }
    assert!(saw_symlink);
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_accepts_symlink_chain_through_archive_prefix() {
    let dir = tempdir().unwrap();
    let package_dir = dir
        .path()
        .join("node_modules/.pnpm/foo@1.0.0/node_modules/foo");
    fs::create_dir_all(package_dir.join("bin")).unwrap();
    fs::write(package_dir.join("bin/foo.js"), b"console.log('foo')").unwrap();
    fs::create_dir_all(dir.path().join("node_modules/.bin")).unwrap();
    std::os::unix::fs::symlink(
        ".pnpm/foo@1.0.0/node_modules/foo",
        dir.path().join("node_modules/foo"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../foo/bin/foo.js",
        dir.path().join("node_modules/.bin/foo"),
    )
    .unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let compressed = plan.read_all().await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read_to_string(extracted.path().join("node_modules/.bin/foo")).unwrap(),
        "console.log('foo')"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_symlink_to_empty_directory() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("empty")).unwrap();
    std::os::unix::fs::symlink("empty", dir.path().join("empty-link")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(
        err.to_string()
            .contains("target is not included in archive"),
        "{err}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_plan_accepts_symlink_to_directory_with_symlink_only_descendants() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("dir")).unwrap();
    fs::write(dir.path().join("real.txt"), b"real").unwrap();
    std::os::unix::fs::symlink("../real.txt", dir.path().join("dir/link")).unwrap();
    std::os::unix::fs::symlink("dir", dir.path().join("alias")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let compressed = plan.read_all().await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read_to_string(extracted.path().join("alias/link")).unwrap(),
        "real"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_symlink_to_filtered_file() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("cache")).unwrap();
    fs::write(dir.path().join("cache/data.txt"), b"cache").unwrap();
    std::os::unix::fs::symlink("cache/data.txt", dir.path().join("cache-link")).unwrap();
    let manifest = static_manifest();
    let mut files = scan_dir(dir.path()).unwrap();
    files.retain(|file| file.path != "cache/data.txt");

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(
        err.to_string()
            .contains("target is not included in archive"),
        "{err}"
    );
}

#[cfg(unix)]
#[test]
fn source_bundle_plan_rejects_overlong_symlink_target() {
    let dir = tempdir().unwrap();
    let target = "a".repeat(SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS + 1);
    std::os::unix::fs::symlink(&target, dir.path().join("long-link")).unwrap();
    let manifest = static_manifest();
    let files = vec![FileEntry {
        path: "long-link".into(),
        size: 0,
        content_hash: format!("{:x}", Sha256::digest(target.as_bytes())),
    }];

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(err.to_string().contains("target too long"), "{err}");
}

#[test]
fn source_bundle_plan_rejects_legacy_manifest_middleware() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("middleware")).unwrap();
    let middleware_body = b"export default function middleware() {}";
    fs::write(dir.path().join("middleware/auth.mjs"), middleware_body).unwrap();
    let manifest: crate::build::manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "static", "target": "STATIC", "directory": "." }
        ],
        "routes": [],
        "middleware": [{
            "name": "auth",
            "bundlePath": "middleware/auth.mjs",
            "codeHash": "sha256-abc",
            "matchers": ["^/.*$"]
        }]
    }))
    .unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let err = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(
        err.to_string()
            .contains("manifest middleware is no longer supported"),
        "{err}"
    );
}

#[test]
fn source_bundle_multipart_threshold_matches_server_contract() {
    assert!(!should_use_multipart(MULTIPART_THRESHOLD_BYTES - 1));
    assert!(should_use_multipart(MULTIPART_THRESHOLD_BYTES));
    assert!(should_use_multipart(MULTIPART_THRESHOLD_BYTES + 1));
}
