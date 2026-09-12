use std::fs::{self, File};
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use nrz_dependency_materializer::{
    DependencyMaterializationKind, DependencyTreeLimits, ErofsToolchain,
    SourceBundleMaterializationPolicy, SourceBundleMaterializationRequest,
    canonicalization_policy_digest, materialize_source_bundle_runtime,
};
use nrz_source_bundle::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceLogicalManifest,
    SourceLogicalManifestEntryType, SourceLogicalManifestFile, SourceLogicalManifestLayer,
    compute_logical_manifest_sha256, sha256_hex,
};
use serde_json::json;
use tempfile::TempDir;

const SERVER_BODY: &[u8] = b"export default { fetch() {} };\n";
const DEPENDENCY_BODY: &[u8] = b"export const dependency = true;\n";

#[test]
fn materializes_dependency_images_and_exact_runtime_graph_from_one_source_bundle() {
    let temp = TempDir::new().unwrap();
    let manifest = source_manifest();
    let source_path = temp.path().join("source.tar.zst");
    write_source_bundle(&source_path, &manifest);
    let source_bytes = fs::read(&source_path).unwrap();
    let logical_manifest = serde_json::to_value(&manifest).unwrap();
    let logical_manifest_sha256 = compute_logical_manifest_sha256(&logical_manifest);
    let source_sha256 = sha256_hex(&source_bytes);
    let toolchain = fake_erofs_toolchain(temp.path());
    let output_root = temp.path().join("runtime");

    let result = materialize_source_bundle_runtime(
        &toolchain,
        SourceBundleMaterializationRequest {
            source_path: &source_path,
            logical_manifest_sha256: &logical_manifest_sha256,
            source_sha256: &source_sha256,
            source_size_bytes: source_bytes.len() as u64,
            manifest: &manifest,
            output_root: &output_root,
            policy: SourceBundleMaterializationPolicy {
                kind: DependencyMaterializationKind::JavaScriptNodeModules,
                compatibility: compatibility(),
                tree_limits: tree_limits(),
                max_total_files: 10,
                max_total_bytes: 1024,
            },
        },
    )
    .unwrap();

    assert_eq!(result.dependencies.len(), 1);
    let dependency = &result.dependencies[0];
    assert_eq!(dependency.layer_name, "server");
    assert_eq!(dependency.mount_point, "/output/node_modules");
    assert_eq!(
        fs::read(&dependency.image_path).unwrap(),
        b"fake-erofs-v1\n"
    );
    assert_eq!(
        fs::metadata(&dependency.image_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o444
    );
    assert_eq!(dependency.manifest.wire().expanded_file_count, 1);
    assert_eq!(
        dependency.manifest.wire().expanded_bytes,
        DEPENDENCY_BODY.len() as i64
    );
    assert_eq!(
        dependency
            .manifest
            .wire()
            .canonicalization_policy_digest
            .as_str(),
        canonicalization_policy_digest()
    );

    let graph = result.graph.wire();
    assert_eq!(graph.dependencies.len(), 1);
    assert_eq!(
        graph.dependencies[0].mount_point.as_str(),
        "/output/node_modules"
    );
    assert_eq!(
        graph.dependencies[0].materialization_id.as_str(),
        dependency.manifest.materialization_id()
    );
    assert_eq!(graph.runtime_layers.len(), 1);
    assert_eq!(graph.runtime_layers[0].layer_name.as_str(), "server");
    assert_eq!(graph.runtime_layers[0].entrypoint.as_str(), "server.js");
    assert_eq!(
        graph.runtime_layers[0].dependency_materialization_ids[0].as_str(),
        dependency.manifest.materialization_id()
    );
    assert_eq!(
        graph.application.manifest_digest.as_str(),
        logical_manifest_sha256
    );
    assert_eq!(
        graph.application.blob_descriptor.digest.as_str(),
        format!("sha256:{source_sha256}")
    );
}

#[test]
fn rejects_compute_runtime_family_that_disagrees_with_build_policy() {
    let temp = TempDir::new().unwrap();
    let mut manifest = source_manifest();
    manifest.files.retain(|file| file.role != "dependency");
    manifest.layers[0].runtime_config = Some(json!({ "runtimeFamily": "PYTHON" }));
    let output_root = temp.path().join("runtime");
    let toolchain = fake_erofs_toolchain(temp.path());

    let error = materialize_source_bundle_runtime(
        &toolchain,
        SourceBundleMaterializationRequest {
            source_path: &temp.path().join("unused-source.tar.zst"),
            logical_manifest_sha256: &"a".repeat(64),
            source_sha256: &"b".repeat(64),
            source_size_bytes: 1,
            manifest: &manifest,
            output_root: &output_root,
            policy: SourceBundleMaterializationPolicy {
                kind: DependencyMaterializationKind::JavaScriptNodeModules,
                compatibility: compatibility(),
                tree_limits: tree_limits(),
                max_total_files: 10,
                max_total_bytes: 1024,
            },
        },
    )
    .err()
    .expect("runtime family mismatch must fail before materialization");

    assert!(
        error
            .to_string()
            .contains("build policy requires JAVASCRIPT")
    );
    assert!(!output_root.exists());
}

#[test]
fn materializes_a_manifest_owned_cross_tree_dependency_symlink() {
    const LINK_TARGET: &str = "../../../node_modules/@prisma/client";
    let temp = TempDir::new().unwrap();
    let mut manifest = source_manifest();
    manifest.files.push(source_file(
        "node_modules/@prisma/client/index.js",
        DEPENDENCY_BODY,
        "dependency",
    ));
    manifest.files.push(SourceLogicalManifestFile {
        path: ".next/node_modules/@prisma/client-generated".to_string(),
        sha256: sha256_hex(LINK_TARGET.as_bytes()),
        size: 0,
        entry_type: SourceLogicalManifestEntryType::Symlink,
        link_target: Some(LINK_TARGET.to_string()),
        content_type: None,
        role: "dependency".to_string(),
        layer_name: Some("server".to_string()),
        executable: false,
    });
    let source_path = temp.path().join("source-cross-tree.tar.zst");
    let encoder =
        zstd::stream::write::Encoder::new(File::create(&source_path).unwrap(), 1).unwrap();
    let mut archive = tar::Builder::new(encoder);
    append_file(
        &mut archive,
        SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
        &serde_json::to_vec(&manifest).unwrap(),
    );
    append_file(&mut archive, "server/server.js", SERVER_BODY);
    append_file(&mut archive, "node_modules/pkg/index.js", DEPENDENCY_BODY);
    append_file(
        &mut archive,
        "node_modules/@prisma/client/index.js",
        DEPENDENCY_BODY,
    );
    append_symlink(
        &mut archive,
        ".next/node_modules/@prisma/client-generated",
        LINK_TARGET,
    );
    let encoder = archive.into_inner().unwrap();
    encoder.finish().unwrap();
    let source_bytes = fs::read(&source_path).unwrap();
    let logical_manifest_sha256 =
        compute_logical_manifest_sha256(&serde_json::to_value(&manifest).unwrap());
    let source_sha256 = sha256_hex(&source_bytes);

    let result = materialize_source_bundle_runtime(
        &fake_erofs_toolchain(temp.path()),
        SourceBundleMaterializationRequest {
            source_path: &source_path,
            logical_manifest_sha256: &logical_manifest_sha256,
            source_sha256: &source_sha256,
            source_size_bytes: source_bytes.len() as u64,
            manifest: &manifest,
            output_root: &temp.path().join("runtime-cross-tree"),
            policy: SourceBundleMaterializationPolicy {
                kind: DependencyMaterializationKind::JavaScriptNodeModules,
                compatibility: compatibility(),
                tree_limits: tree_limits(),
                max_total_files: 10,
                max_total_bytes: 1024,
            },
        },
    )
    .unwrap();

    assert_eq!(result.dependencies.len(), 2);
    let root = result
        .dependencies
        .iter()
        .find(|dependency| dependency.mount_point == "/output/node_modules")
        .unwrap();
    let next = result
        .dependencies
        .iter()
        .find(|dependency| dependency.mount_point == "/output/.next/node_modules")
        .unwrap();
    assert_eq!(
        root.manifest.wire().canonicalization_policy_digest.as_str(),
        canonicalization_policy_digest()
    );
    assert_ne!(
        next.manifest.wire().canonicalization_policy_digest.as_str(),
        canonicalization_policy_digest()
    );
    assert_eq!(next.manifest.wire().symlink_count, 1);
    assert_eq!(result.graph.wire().dependencies.len(), 2);
}

fn source_manifest() -> SourceLogicalManifest {
    SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
        capabilities: Vec::new(),
        files: vec![
            source_file("server/server.js", SERVER_BODY, "compute"),
            source_file("node_modules/pkg/index.js", DEPENDENCY_BODY, "dependency"),
        ],
        layers: vec![SourceLogicalManifestLayer {
            name: "server".to_string(),
            target: "COMPUTE".to_string(),
            root_path: Some("server".to_string()),
            entrypoint: Some("server/server.js".to_string()),
            runtime_config: Some(json!({ "memoryMb": 256 })),
        }],
        routes: Vec::new(),
        entrypoints: Vec::new(),
    }
}

fn source_file(path: &str, body: &[u8], role: &str) -> SourceLogicalManifestFile {
    SourceLogicalManifestFile {
        path: path.to_string(),
        sha256: sha256_hex(body),
        size: body.len() as u64,
        entry_type: SourceLogicalManifestEntryType::File,
        link_target: None,
        content_type: None,
        role: role.to_string(),
        layer_name: Some("server".to_string()),
        executable: false,
    }
}

fn write_source_bundle(path: &Path, manifest: &SourceLogicalManifest) {
    let encoder = zstd::stream::write::Encoder::new(File::create(path).unwrap(), 1).unwrap();
    let mut archive = tar::Builder::new(encoder);
    append_file(
        &mut archive,
        SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
        &serde_json::to_vec(manifest).unwrap(),
    );
    append_file(&mut archive, "server/server.js", SERVER_BODY);
    append_file(&mut archive, "node_modules/pkg/index.js", DEPENDENCY_BODY);
    let encoder = archive.into_inner().unwrap();
    encoder.finish().unwrap();
}

fn append_file<W: std::io::Write>(archive: &mut tar::Builder<W>, path: &str, body: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_cksum();
    archive
        .append_data(&mut header, path, Cursor::new(body))
        .unwrap();
}

fn append_symlink<W: std::io::Write>(archive: &mut tar::Builder<W>, path: &str, target: &str) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_mode(0o777);
    header.set_size(0);
    header.set_link_name(target).unwrap();
    header.set_cksum();
    archive
        .append_data(&mut header, path, Cursor::new([]))
        .unwrap();
}

fn fake_erofs_toolchain(root: &Path) -> ErofsToolchain {
    let mkfs = root.join("mkfs.erofs");
    let fsck = root.join("fsck.erofs");
    write_executable(
        &mkfs,
        "#!/bin/sh\nset -eu\nprevious=\ncurrent=\nfor argument in \"$@\"; do\n  previous=$current\n  current=$argument\ndone\nprintf 'fake-erofs-v1\\n' > \"$previous\"\n",
    );
    write_executable(
        &fsck,
        "#!/bin/sh\nset -eu\nlast=\nfor argument in \"$@\"; do\n  last=$argument\ndone\ntest -s \"$last\"\n",
    );
    ErofsToolchain::new_direct_for_tests(mkfs, fsck).unwrap()
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn compatibility() -> serde_json::Value {
    json!({
        "runtimeFamily": "bun",
        "runtimeVersion": "1.4.2",
        "os": "linux",
        "architecture": "x86_64",
        "libc": "glibc",
        "abi": "glibc-2.42",
        "packageManager": "bun",
        "packageManagerVersion": "1.4.2",
        "runnerRootfsDigest": format!("sha256:{}", "d".repeat(64)),
        "buildPolicyGeneration": 1,
    })
}

fn tree_limits() -> DependencyTreeLimits {
    DependencyTreeLimits {
        max_files: 10,
        max_expanded_bytes: 1024,
        max_path_bytes: 512,
        max_symlinks: 10,
    }
}
