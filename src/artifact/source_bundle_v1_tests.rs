use std::fs;
use std::io::Cursor;
#[cfg(unix)]
use std::path::PathBuf;

use tempfile::tempdir;

use crate::deploy::hash::sha256_hex;
use crate::deploy::scan_dir;

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

fn compute_manifest() -> crate::build::manifest::Manifest {
    serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js" }
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
    assert_eq!(first.logical_manifest.files[0].sha256, sha256_hex(b"a"));
    assert_eq!(
        first.logical_manifest.files[0].role,
        SourceLogicalManifestFileRole::Static
    );
    let compressed = tokio::fs::read(first.source_path()).await.unwrap();
    assert_eq!(sha256_hex(&compressed), first.source_sha256);
    assert_eq!(compressed.len() as u64, first.source_size_bytes);
}

#[test]
fn source_bundle_assigns_dependency_ownership_only_for_trusted_materialization() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
    fs::write(dir.path().join("server.js"), b"require('pkg')").unwrap();
    fs::write(
        dir.path().join("node_modules/pkg/index.js"),
        b"module.exports = 1",
    )
    .unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &compute_manifest(),
        &files,
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
    )
    .unwrap();
    let embedded = build_source_bundle_plan_with_scan(
        dir.path(),
        &compute_manifest(),
        &files,
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::Embedded,
    )
    .unwrap();

    let server = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "server.js")
        .unwrap();
    let dependency = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg/index.js")
        .unwrap();
    assert_eq!(server.role, SourceLogicalManifestFileRole::Compute);
    assert_eq!(dependency.role, SourceLogicalManifestFileRole::Dependency);
    assert_eq!(dependency.layer_name.as_deref(), Some("server"));
    let embedded_dependency = embedded
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg/index.js")
        .unwrap();
    assert_eq!(
        embedded_dependency.role,
        SourceLogicalManifestFileRole::Compute
    );
    assert_eq!(embedded_dependency.layer_name.as_deref(), Some("server"));
}

#[cfg(unix)]
#[tokio::test]
async fn source_bundle_projects_workspace_packages_into_dependency_root() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("dist")).unwrap();
    fs::write(dir.path().join("dist/server.js"), b"require('pkg')").unwrap();
    fs::create_dir_all(dir.path().join("packages/pkg")).unwrap();
    fs::write(
        dir.path().join("packages/pkg/package.json"),
        br#"{"name":"pkg"}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("packages/pkg/bin.js"),
        b"console.log('pkg')",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("node_modules/.bin")).unwrap();
    std::os::unix::fs::symlink("../packages/pkg", dir.path().join("node_modules/pkg")).unwrap();
    std::os::unix::fs::symlink("../pkg/bin.js", dir.path().join("node_modules/.bin/pkg")).unwrap();
    let files = scan_dir(dir.path()).unwrap();
    let scan = RuntimeArtifactScan::Selected {
        roots: vec![
            RuntimeArtifactScanRoot {
                path: "dist".into(),
                kind: RuntimeArtifactScanRootKind::BuildOutput,
            },
            RuntimeArtifactScanRoot {
                path: "node_modules".into(),
                kind: RuntimeArtifactScanRootKind::NodeModules,
            },
        ],
        symlink_roots: vec!["packages/pkg".into()],
    };

    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &compute_manifest(),
        &files,
        &scan,
        RuntimeDependencyPackaging::TrustedMaterialization,
    )
    .unwrap();

    assert!(
        plan.logical_manifest
            .files
            .iter()
            .all(|file| !file.path.starts_with("packages/pkg"))
    );
    let package = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/pkg/package.json")
        .unwrap();
    assert_eq!(package.role, SourceLogicalManifestFileRole::Dependency);
    let bin = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/.bin/pkg")
        .unwrap();
    assert_eq!(bin.link_target.as_deref(), Some("../pkg/bin.js"));

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
    let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed)).unwrap();
    let extracted = tempdir().unwrap();
    tar::Archive::new(Cursor::new(tar_bytes))
        .unpack(extracted.path())
        .unwrap();
    assert_eq!(
        fs::read_to_string(extracted.path().join("node_modules/.bin/pkg")).unwrap(),
        "console.log('pkg')"
    );
}

#[tokio::test]
async fn source_bundle_embeds_canonical_logical_manifest_first() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), b"hello").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
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
        sha256_hex(manifest_body.as_bytes()),
        plan.logical_manifest_sha256
    );
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
fn source_bundle_marks_prerender_files_under_layer_root() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("_prerender")).unwrap();
    fs::write(dir.path().join("_prerender/index.html"), b"<main/>").unwrap();
    fs::create_dir_all(dir.path().join("server")).unwrap();
    fs::write(dir.path().join("server/server.js"), b"// server").unwrap();

    let manifest: crate::build::manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            { "name": "prerendered", "target": "STATIC", "directory": "_prerender" },
            { "name": "server", "target": "COMPUTE", "directory": "server", "entry": "server.js" }
        ],
        "routes": [
            {
                "pattern": "^/.*$",
                "layer": "prerendered",
                "priority": 75,
                "fallthrough": true,
                "fallthroughWhen": [
                    { "type": "header", "name": "rsc", "value": "1" },
                    { "type": "query", "name": "_rsc" }
                ]
            },
            { "pattern": "^/.*$", "layer": "server", "priority": 0 }
        ],
        "prerender": {
            "layer": "prerendered",
            "pages": { "/": { "html": "index.html" } }
        }
    }))
    .unwrap();

    let files = scan_dir(dir.path()).unwrap();
    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let prerender = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "_prerender/index.html")
        .unwrap();
    assert_eq!(prerender.role, SourceLogicalManifestFileRole::Prerender);
    assert_eq!(prerender.layer_name.as_deref(), Some("prerendered"));

    assert_eq!(
        plan.logical_manifest.routes[0].fallthrough_when.as_ref(),
        manifest.routes[0].fallthrough_when.as_ref()
    );
}

#[test]
fn nuxt_public_asset_maps_to_static_layer_root() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("public")).unwrap();
    fs::write(dir.path().join("public/favicon.svg"), b"<svg/>").unwrap();
    fs::create_dir_all(dir.path().join("server")).unwrap();
    fs::write(dir.path().join("server/index.mjs"), b"// server").unwrap();

    let manifest = crate::build::manifest::generate_nuxt_manifest(true);
    crate::build::manifest::validate(&manifest).unwrap();
    crate::build::manifest::verify_files(dir.path(), &manifest).unwrap();

    let files = scan_dir(dir.path()).unwrap();
    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();

    let static_layer = plan
        .logical_manifest
        .layers
        .iter()
        .find(|layer| layer.name == "static-assets")
        .unwrap();
    assert_eq!(
        static_layer.target,
        SourceLogicalManifestLayerTarget::Static
    );
    assert_eq!(static_layer.root_path.as_deref(), Some("public"));

    let favicon = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "public/favicon.svg")
        .unwrap();
    assert_eq!(favicon.role, SourceLogicalManifestFileRole::Static);
    assert_eq!(favicon.layer_name.as_deref(), Some("static-assets"));

    let static_catch_all = plan
        .logical_manifest
        .routes
        .iter()
        .find(|route| route.pattern == "^/.*$" && route.layer_name == "static-assets")
        .unwrap();
    assert_eq!(static_catch_all.priority, Some(50));

    let server_catch_all = plan
        .logical_manifest
        .routes
        .iter()
        .find(|route| route.pattern == "^/.*$" && route.layer_name == "server")
        .unwrap();
    assert_eq!(server_catch_all.priority, Some(0));
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
            executable: false,
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

#[test]
fn canonical_logical_manifest_json_matches_source_bundle_v1_golden() {
    let manifest = SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_SCHEMA_VERSION.to_string(),
        capabilities: vec![],
        files: vec![
            SourceLogicalManifestFile {
                path: "api/handler.js".into(),
                sha256: "b".repeat(64),
                size: 128,
                entry_type: None,
                link_target: None,
                content_type: Some("application/javascript; charset=utf-8".into()),
                role: SourceLogicalManifestFileRole::Compute,
                layer_name: Some("api".into()),
                executable: true,
            },
            SourceLogicalManifestFile {
                path: "index.html".into(),
                sha256: "a".repeat(64),
                size: 5,
                entry_type: None,
                link_target: None,
                content_type: Some("text/html; charset=utf-8".into()),
                role: SourceLogicalManifestFileRole::Static,
                layer_name: Some("static".into()),
                executable: false,
            },
            SourceLogicalManifestFile {
                path: "link.html".into(),
                sha256: sha256_hex(b"index.html"),
                size: 0,
                entry_type: Some(SourceLogicalManifestEntryType::Symlink),
                link_target: Some("index.html".into()),
                content_type: None,
                role: SourceLogicalManifestFileRole::Static,
                layer_name: Some("static".into()),
                executable: false,
            },
        ],
        layers: vec![
            SourceLogicalManifestLayer {
                name: "api".into(),
                target: SourceLogicalManifestLayerTarget::Compute,
                root_path: Some("api".into()),
                entrypoint: Some("api/handler.js".into()),
                runtime_config: Some(serde_json::json!({ "timeoutMs": 10000, "memoryMb": 256 })),
            },
            SourceLogicalManifestLayer {
                name: "static".into(),
                target: SourceLogicalManifestLayerTarget::Static,
                root_path: None,
                entrypoint: None,
                runtime_config: None,
            },
        ],
        routes: vec![SourceLogicalManifestRoute {
            pattern: "/api/*".into(),
            layer_name: "api".into(),
            priority: Some(10),
            methods: Some(vec!["GET".into(), "POST".into()]),
            fallthrough_when: None,
        }],
        entrypoints: vec!["api/handler.js".into()],
    };

    let canonical = canonical_logical_manifest_json(&manifest).unwrap();
    assert_eq!(
        canonical,
        r#"{"capabilities":[],"entrypoints":["api/handler.js"],"files":[{"contentType":"application/javascript; charset=utf-8","executable":true,"layerName":"api","path":"api/handler.js","role":"compute","sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","size":128},{"contentType":"text/html; charset=utf-8","layerName":"static","path":"index.html","role":"static","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":5},{"entryType":"symlink","layerName":"static","linkTarget":"index.html","path":"link.html","role":"static","sha256":"0eb547304658805aad788d320f10bf1f292797b5e6d745a3bf617584da017051","size":0}],"layers":[{"entrypoint":"api/handler.js","name":"api","rootPath":"api","runtimeConfig":{"memoryMb":256,"timeoutMs":10000},"target":"COMPUTE"},{"name":"static","target":"STATIC"}],"routes":[{"layerName":"api","methods":["GET","POST"],"pattern":"/api/*","priority":10}],"schemaVersion":"SOURCE_BUNDLE_V1.0"}"#
    );
    assert_eq!(
        compute_logical_manifest_sha256(&manifest).unwrap(),
        "0ad27552cb64fab088d6e4c4b44c2cd83df43dbd2f5ee76add074e6832aad62a"
    );
    assert_eq!(
        compute_logical_manifest_sha256(&manifest).unwrap(),
        sha256_hex(canonical.as_bytes())
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

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
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
    assert_eq!(symlink.sha256, sha256_hex(b".pnpm/pkg"));

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
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

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
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
#[test]
fn source_bundle_plan_rejects_recursive_directory_symlink() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("nested")).unwrap();
    fs::write(dir.path().join("nested/file.txt"), b"file").unwrap();
    std::os::unix::fs::symlink(".", dir.path().join("nested/loop")).unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let error = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap_err();

    assert!(error.to_string().contains("recursive symlink"), "{error}");
}

#[cfg(unix)]
#[test]
fn source_bundle_archive_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("secret.txt"), b"secret").unwrap();
    let manifest = static_manifest();
    let files = scan_dir(dir.path()).unwrap();

    let plan = build_source_bundle_plan(dir.path(), &manifest, &files).unwrap();
    let mode = fs::metadata(plan.source_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(mode, 0o600);
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

    let compressed = tokio::fs::read(plan.source_path()).await.unwrap();
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
        content_hash: sha256_hex(target.as_bytes()),
        kind: crate::artifact::ArtifactFileKind::Symlink,
        symlink_resolved_path: None,
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
        err.to_string().contains(
            "manifest middleware is no longer supported; declare HTTP function wiring in onreza.rules.toml with a pipeline action"
        ),
        "{err}"
    );
}
