use super::*;

#[test]
fn published_python_bundle_excludes_installer_staging_and_retains_installed_assets() {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let project = tempfile::tempdir().unwrap();
        let package = format!("{}/demo", minor.site_packages_root());
        let retained = [
            ("main.py".to_string(), "print('ready')"),
            ("requirements.txt".to_string(), "# authored build input"),
            (".onreza/python/launch.py".to_string(), "print('bootstrap')"),
            (format!("{package}/__init__.py"), "# installed package"),
            (
                format!("{package}/build/wheels/data.whl"),
                "installed wheel asset",
            ),
            (
                format!("{package}/.onreza/python/build/payload.txt"),
                "installed namespace asset",
            ),
        ];
        let staging = [
            ".onreza/python/build/requirements.txt",
            ".onreza/python/build/wheels/demo-1.0-py3-none-any.whl",
        ];
        for (path, bytes) in &retained {
            let file = project.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        for path in staging {
            let file = project.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, "INSTALLER_INTERMEDIATE").unwrap();
        }
        let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
            "version": 1, "routes": [], "layers": [{
                "name": "python", "target": "COMPUTE", "directory": ".",
                "entry": ".onreza/python/launch.py",
                "runtime": {"applicationRuntime": {"family": "PYTHON", "args": []},
                    "buildRuntimeVersion": minor.target()}
            }]
        }))
        .unwrap();
        let detection =
            crate::detect::detect_with_framework_override(project.path(), Some("python"));
        for scan in [
            RuntimeArtifactScan::PythonRuntimeRoot(minor),
            RuntimeArtifactScan::All,
            RuntimeArtifactScan::NodeRuntimeRoot,
        ] {
            // SOURCE_BUNDLE callers can consume the scanner directly, before
            // deployment's framework/platform classification.
            let scanned = scan_runtime_artifact(project.path(), &scan).unwrap();
            let raw = crate::artifact::source_bundle_v1::build_source_bundle_plan_with_scan(
                project.path(),
                &manifest,
                &scanned,
                &scan,
                crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
                None,
            )
            .unwrap();
            for path in staging {
                assert!(
                    !raw.logical_manifest
                        .files
                        .iter()
                        .any(|file| file.path == path),
                    "raw SOURCE_BUNDLE includes installer staging with {scan:?}: {path}"
                );
            }
            let files = prepare_artifact_files(
                &manifest,
                scanned,
                &detection,
                ArtifactRootScope::ProjectRoot,
                &scan,
                true,
            )
            .deployable_entries();
            let source = crate::artifact::source_bundle_v1::build_source_bundle_plan_with_scan(
                project.path(),
                &manifest,
                &files,
                &scan,
                crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
                None,
            )
            .unwrap();
            let unpacked = tempfile::tempdir().unwrap();
            let decoder = zstd::stream::read::Decoder::new(
                std::fs::File::open(source.source_path()).unwrap(),
            )
            .unwrap();
            tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
            for path in staging {
                assert!(
                    !source
                        .logical_manifest
                        .files
                        .iter()
                        .any(|file| file.path == path),
                    "installer staging published with {scan:?}: {path}"
                );
                assert!(!unpacked.path().join(path).exists());
            }
            for (path, bytes) in &retained {
                assert!(
                    source
                        .logical_manifest
                        .files
                        .iter()
                        .any(|file| file.path == *path),
                    "required input absent with {scan:?}: {path}"
                );
                assert_eq!(
                    std::fs::read_to_string(unpacked.path().join(path)).unwrap(),
                    *bytes
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn installer_staging_symlinks_do_not_block_python_runtime_scan() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".onreza/python/build/wheels")).unwrap();
    std::fs::write(project.path().join("main.py"), "print('CODE_ONLY')").unwrap();
    std::os::unix::fs::symlink(
        "/installer/temporary/output",
        project.path().join(".onreza/python/build/wheels/stale.whl"),
    )
    .unwrap();
    for scan in [
        RuntimeArtifactScan::PythonRuntimeRoot(nrz_source_bundle::PythonMinor::default()),
        RuntimeArtifactScan::All,
    ] {
        let files = scan_runtime_artifact(project.path(), &scan)
            .expect("installer-local symlinks are not runtime inputs");
        assert_eq!(
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["main.py"]
        );
    }
}

#[cfg(unix)]
#[test]
fn workspace_selected_output_excludes_its_own_python_installer_state() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("apps/site");
    std::fs::create_dir_all(project.join(".onreza/python/build/wheels")).unwrap();
    std::fs::write(project.join("main.py"), "print('CODE_ONLY')").unwrap();
    std::os::unix::fs::symlink(
        "/installer/temporary/output",
        project.join(".onreza/python/build/wheels/stale.whl"),
    )
    .unwrap();
    let package_asset = project.join("node_modules/demo/.onreza/python/build/payload.txt");
    std::fs::create_dir_all(package_asset.parent().unwrap()).unwrap();
    std::fs::write(package_asset, "INSTALLED_ASSET").unwrap();
    let scan = RuntimeArtifactScan::Selected {
        roots: vec![crate::artifact::RuntimeArtifactScanRoot {
            path: "apps/site".into(),
            kind: crate::artifact::RuntimeArtifactScanRootKind::BuildOutput,
        }],
        symlink_roots: vec![],
    };
    let files = scan_runtime_artifact(workspace.path(), &scan)
        .expect("workspace project installer state is not runtime input");
    assert_eq!(
        files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        [
            "apps/site/main.py",
            "apps/site/node_modules/demo/.onreza/python/build/payload.txt"
        ]
    );
    assert_eq!(files[1].content_hash, sha256_hex(b"INSTALLED_ASSET"));
}
