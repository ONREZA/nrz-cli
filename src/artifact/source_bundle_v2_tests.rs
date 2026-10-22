use super::*;
use tempfile::tempdir;

fn static_manifest() -> Manifest {
    serde_json::from_value(serde_json::json!({"version":2,
        "layers":[{"name":"static","target":"STATIC","directory":"."}],
        "routes":[{"pattern":"^/.*$","layer":"static"}]
    }))
    .unwrap()
}

fn file(path: &str, bytes: &[u8]) -> FileEntry {
    FileEntry {
        path: path.into(),
        size: bytes.len() as u64,
        content_hash: nrz_source_bundle::sha256_hex(bytes),
        kind: super::super::ArtifactFileKind::File,
        symlink_resolved_path: None,
        symlink_target: None,
    }
}

#[test]
fn freezes_zero_byte_index_without_a_logical_size_minimum() -> anyhow::Result<()> {
    let source = tempdir()?;
    let output = tempdir()?;
    fs::write(source.path().join("index.html"), b"")?;
    let destination = output.path().join("frozen");
    let (_, verified) = freeze_source_bundle(
        &ArtifactRoot::open(source.path())?,
        &static_manifest(),
        &[file("index.html", b"")],
        &RuntimeArtifactScan::All,
        RuntimeDependencyPackaging::Embedded,
        &destination,
        SourceRuntimeMetadata::default(),
    )?;
    assert_eq!(verified.summary().logical_static_bytes, 0);
    assert_eq!(verified.summary().max_static_file_size_bytes, 0);
    assert_eq!(fs::read(destination.join("index.html"))?, b"");
    assert!(
        matches!(&verified.wire().files[0], wire::FileEntry::File { size: 0, blob_digest, .. }
        if blob_digest.value == blake3::hash(b"").to_hex().as_str())
    );
    Ok(())
}

#[test]
fn freezes_only_planned_bytes_and_preserves_prerender_roles() -> anyhow::Result<()> {
    let source = tempdir()?;
    let output = tempdir()?;
    fs::write(source.path().join("index.html"), b"index")?;
    fs::write(source.path().join("unplanned-secret"), b"secret")?;
    let mut manifest = static_manifest();
    manifest.prerender = Some(serde_json::from_value(serde_json::json!({
        "layer":"static","pages":{"/":{"html":"index.html"}}
    }))?);
    let destination = output.path().join("frozen");
    let (bytes, verified) = freeze_source_bundle(
        &ArtifactRoot::open(source.path())?,
        &manifest,
        &[file("index.html", b"index")],
        &RuntimeArtifactScan::All,
        RuntimeDependencyPackaging::Embedded,
        &destination,
        SourceRuntimeMetadata::default(),
    )?;
    assert_eq!(
        verified.manifest_blake3(),
        blake3::hash(&bytes).to_hex().as_str()
    );
    assert!(
        matches!(&verified.wire().files[0], wire::FileEntry::File {role: wire::FileRole::Prerender, blob_digest, ..}
        if blob_digest.value == blake3::hash(b"index").to_hex().as_str())
    );
    fs::write(source.path().join("index.html"), b"changed")?;
    assert_eq!(fs::read(destination.join("index.html"))?, b"index");
    assert!(!destination.join("unplanned-secret").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn freezes_workspace_projection_when_it_expands_the_planned_inventory() -> anyhow::Result<()> {
    let source = tempdir()?;
    let output = tempdir()?;
    fs::create_dir_all(source.path().join("packages/pkg"))?;
    fs::create_dir(source.path().join("node_modules"))?;
    for name in ["index.js", "package.json"] {
        fs::write(source.path().join("packages/pkg").join(name), b"package")?;
    }
    std::os::unix::fs::symlink("../packages/pkg", source.path().join("node_modules/pkg"))?;
    let files = crate::deploy::scan_dir(source.path())?;
    let scan = RuntimeArtifactScan::Selected {
        roots: vec![
            super::super::RuntimeArtifactScanRoot {
                path: ".".into(),
                kind: super::super::RuntimeArtifactScanRootKind::BuildOutput,
            },
            super::super::RuntimeArtifactScanRoot {
                path: "node_modules".into(),
                kind: super::super::RuntimeArtifactScanRootKind::NodeModules,
            },
        ],
        symlink_roots: vec!["packages/pkg".into()],
    };
    let destination = output.path().join("frozen");
    let (_, verified) = freeze_source_bundle(
        &ArtifactRoot::open(source.path())?,
        &static_manifest(),
        &files,
        &scan,
        RuntimeDependencyPackaging::Embedded,
        &destination,
        SourceRuntimeMetadata::default(),
    )?;
    assert_eq!(verified.summary().file_count, 4);
    assert!(verified.summary().file_count > files.len() as u64);
    for root in ["packages/pkg", "node_modules/pkg"] {
        for name in ["index.js", "package.json"] {
            assert_eq!(fs::read(destination.join(root).join(name))?, b"package");
        }
    }
    Ok(())
}

#[test]
fn rejects_inventory_mutation_before_output_copy() -> anyhow::Result<()> {
    let source = tempdir()?;
    let output = tempdir()?;
    fs::write(source.path().join("index.html"), b"changed")?;
    let owner = ArtifactRoot::open(source.path())?;
    let files = [file("index.html", b"index")];
    let destination = output.path().join("frozen");
    assert!(
        freeze_source_bundle(
            &owner,
            &static_manifest(),
            &files,
            &RuntimeArtifactScan::All,
            RuntimeDependencyPackaging::Embedded,
            &destination,
            SourceRuntimeMetadata::default(),
        )
        .is_err()
    );
    assert!(!destination.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn preserves_contained_symlink_and_refuses_root_substitution() -> anyhow::Result<()> {
    let source = tempdir()?;
    let output = tempdir()?;
    let foreign = tempdir()?;
    fs::write(source.path().join("index.html"), b"index")?;
    std::os::unix::fs::symlink("index.html", source.path().join("alias.html"))?;
    let mut alias = file("alias.html", b"");
    alias.kind = super::super::ArtifactFileKind::Symlink;
    alias.content_hash = nrz_source_bundle::sha256_hex(b"index.html");
    let files = [alias, file("index.html", b"index")];
    let owner = ArtifactRoot::open(source.path())?;
    let destination = output.path().join("frozen");
    let (_, verified) = freeze_source_bundle(
        &owner,
        &static_manifest(),
        &files,
        &RuntimeArtifactScan::All,
        RuntimeDependencyPackaging::Embedded,
        &destination,
        SourceRuntimeMetadata::default(),
    )?;
    assert!(
        matches!(&verified.wire().files[0], wire::FileEntry::Symlink {link_target,..} if link_target.0 == "index.html")
    );
    assert_eq!(
        fs::read_link(destination.join("alias.html"))?,
        Path::new("index.html")
    );
    fs::remove_file(source.path().join("index.html"))?;
    fs::write(foreign.path().join("index.html"), b"index")?;
    std::os::unix::fs::symlink(
        foreign.path().join("index.html"),
        source.path().join("index.html"),
    )?;
    assert!(
        freeze_source_bundle(
            &owner,
            &static_manifest(),
            &files,
            &RuntimeArtifactScan::All,
            RuntimeDependencyPackaging::Embedded,
            &output.path().join("rejected"),
            SourceRuntimeMetadata::default(),
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn dependency_roles_follow_explicit_direct_cli_or_builder_packaging() -> anyhow::Result<()> {
    for (family, target, entry, dependency_path, scan) in [
        (
            "NODE",
            "node-24",
            "server.js",
            "node_modules/pkg/index.js",
            RuntimeArtifactScan::NodeRuntimeRoot,
        ),
        (
            "EXECUTABLE",
            "native-linux-x86_64-glibc",
            "server",
            "lib/resource",
            RuntimeArtifactScan::All,
        ),
        (
            "PYTHON",
            "python-3.13",
            "server.py",
            ".onreza/python/3.13/site-packages/pkg/__init__.py",
            RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::Python313),
        ),
    ] {
        let source = tempdir()?;
        let output = tempdir()?;
        fs::create_dir_all(source.path().join(dependency_path).parent().unwrap())?;
        if family == "EXECUTABLE" {
            use crate::artifact::native_test_support::native_elf_fixture;
            fs::write(
                source.path().join(entry),
                native_elf_fixture(Some("$ORIGIN/lib")),
            )?;
            fs::write(
                source.path().join("lib/libfoo.so"),
                native_elf_fixture(None),
            )?;
        } else {
            fs::write(source.path().join(entry), b"server")?;
        }
        fs::write(source.path().join(dependency_path), b"dependency")?;
        let manifest: Manifest = serde_json::from_value(serde_json::json!({"version":2,
            "layers":[{"name":"server","target":"COMPUTE","directory":".","entry":entry,
            "runtime":{"network":{"schemaVersion":"COMPUTE_NETWORK_V1","exposure":"PUBLIC",
            "ports":[{"name":"http","protocol":"http","port":3000,"binding":"primary"}],
            "readiness":{"protocol":"tcp","port":"http"},"allowFrom":[]},
            "applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":target}}],
            "routes":[{"pattern":"^/.*$","layer":"server"}]
        }))?;
        let files = crate::deploy::scan_dir(source.path())?;
        for (name, packaging, dependency_role) in [
            (
                "direct",
                RuntimeDependencyPackaging::Embedded,
                wire::FileRole::Compute,
            ),
            (
                "builder",
                RuntimeDependencyPackaging::TrustedMaterialization,
                if family == "EXECUTABLE" {
                    wire::FileRole::Compute
                } else {
                    wire::FileRole::Dependency
                },
            ),
        ] {
            let destination = output.path().join(name);
            let (_, verified) = freeze_source_bundle(
                &ArtifactRoot::open(source.path())?,
                &manifest,
                &files,
                &scan,
                packaging,
                &destination,
                SourceRuntimeMetadata::default(),
            )?;
            if family == "EXECUTABLE" {
                assert!(
                    verified.wire().files.iter().any(|file| matches!(file,
                        wire::FileEntry::File { path, executable: true, .. } if path.0 == entry
                    )),
                    "native entry must retain executable authority from shared preparation"
                );
            }
            for (path, role) in [
                (entry, wire::FileRole::Compute),
                (dependency_path, dependency_role),
            ] {
                assert!(verified.wire().files.iter().any(|entry| matches!(entry,
                    wire::FileEntry::File { path: actual, role: actual_role, layer_name, blob_digest, .. }
                    if actual.0 == path && *actual_role == role
                        && layer_name.as_ref().is_some_and(|layer| layer.0 == "server")
                        && blob_digest.value == blake3::hash(&fs::read(destination.join(path)).unwrap()).to_hex().as_str()
                )), "{family}/{name}: {path}");
            }
            assert_eq!(verified.wire().entrypoints[0].0, entry);
        }
    }
    Ok(())
}
