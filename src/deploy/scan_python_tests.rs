use super::*;

#[tokio::test]
async fn selected_python_output_named_like_backend_state_survives_verified_publication() {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        for output in [
            "build/lib",
            "build/lib/authored",
            "build/lib.linux-x86_64-cpython-314",
            "build/lib.linux-x86_64-cpython-314/authored",
            "build/bdist.fixture/runtime",
        ] {
            let project = tempfile::tempdir().unwrap();
            std::fs::write(project.path().join("pyproject.toml"), "[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n[project]\nname='demo'\nversion='1.0'\n").unwrap();
            std::fs::write(project.path().join("onreza.toml"), format!("[build]\ntoolchain='python'\npython_version='{}'\noutput_directory='{output}'\n[deploy]\nruntime='python'\npython_version='{}'\nentry='{output}/main.py'\n", minor.version(), minor.version())).unwrap();
            let retained = [
                (format!("{output}/main.py"), "print('AUTHORED_OUTPUT')"),
                (format!("{output}/data/payload.txt"), "AUTHORED_RESOURCE"),
                (
                    format!("{}/demo/__init__.py", minor.site_packages_root()),
                    "# INSTALLED_PROJECT_WHEEL",
                ),
                (
                    format!(
                        "{}/demo/build/lib.linux-x86_64-cpython-314/resource.txt",
                        minor.site_packages_root()
                    ),
                    "INSTALLED_PACKAGE_RESOURCE",
                ),
                (
                    "resources/build/lib.linux-x86_64-cpython-314/source.txt".to_string(),
                    "PROJECT_SOURCE_RESOURCE",
                ),
                ("build/lib.data".to_string(), "AUTHORED_LITERAL_FILE"),
            ];
            let excluded = [
                "build/bdist.unselected/installer.txt",
                "build/lib.win-amd64-cpython-312/demo/native.pyd",
                "src/demo.egg-info/PKG-INFO",
                ".venv/pyvenv.cfg",
                ".onreza/python/build/wheels/debris.whl",
                ".env.defaults",
            ];
            for (path, body) in &retained {
                let file = project.path().join(path);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, body).unwrap();
            }
            for path in excluded {
                let file = project.path().join(path);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, "PROJECT_BUILD_STATE").unwrap();
            }
            // Only this nested selected subtree survives, not its backend siblings.
            if output != "build/lib" {
                std::fs::create_dir_all(project.path().join("build/lib")).unwrap();
                std::fs::write(
                    project.path().join("build/lib/stale.py"),
                    "BACKEND_DUPLICATE",
                )
                .unwrap();
            }
            let dependency = crate::detect::python::dependency_plan(
                &crate::detect::fs::LocalFs::new(project.path()),
            )
            .unwrap()
            .unwrap();
            assert!(dependency.install_project);
            let mut detection = crate::detect::detect_with_framework_override(project.path(), None);
            crate::detect::application_runtime::resolve_and_bind_detection(
                &crate::detect::fs::LocalFs::new(project.path()),
                &mut detection,
            )
            .unwrap();
            let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
                "version":1,"routes":[{"pattern":"^/.*$","layer":"python"}],"layers":[{"name":"python","target":"COMPUTE","directory":".","entry":"main.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":minor.target()}}]
            })).unwrap();
            let artifact = resolve_runtime_artifact(
                project.path(),
                project.path(),
                project.path().join(output),
                manifest,
                &detection,
                true,
            )
            .unwrap();
            assert_eq!(
                artifact.scan.explain()["sourceOwnershipBuildOutputPrefix"],
                output
            );
            assert_python_scan_archive(&artifact, &detection, &retained, &excluded).await;
            if output != "build/lib" {
                let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
                assert!(!scanned.iter().any(|file| file.path == "build/lib/stale.py"));
            }
        }
    }
}

#[tokio::test]
async fn unpackaged_python_backend_named_resources_remain_application_assets() {
    let project = tempfile::tempdir().unwrap();
    let retained = [
        ("main.py".to_string(), "print('UNPACKAGED_SCRIPT')"),
        (
            "build/lib.linux-x86_64-cpython-314/resource.txt".to_string(),
            "AUTHORED_APPLICATION_ASSET",
        ),
    ];
    for (path, body) in &retained {
        let path = project.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    let mut detection = crate::detect::detect(project.path());
    crate::detect::application_runtime::resolve_and_bind_detection(
        &crate::detect::fs::LocalFs::new(project.path()),
        &mut detection,
    )
    .unwrap();
    let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version":1,"routes":[{"pattern":"^/.*$","layer":"python"}],"layers":[{"name":"python","target":"COMPUTE","directory":".","entry":"main.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":nrz_source_bundle::PythonMinor::default().target()}}]
    })).unwrap();
    let artifact = resolve_runtime_artifact(
        project.path(),
        project.path(),
        project.path().into(),
        manifest,
        &detection,
        true,
    )
    .unwrap();
    assert_python_scan_archive(&artifact, &detection, &retained, &[]).await;
}

#[tokio::test]
async fn staged_python_dotenv_resources_survive_verified_publication() {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("main.py"), "print('ENTRY')").unwrap();
        std::fs::write(project.path().join("onreza.toml"), format!("[build]\ntoolchain='python'\npython_version='{}'\n[deploy]\nruntime='python'\npython_version='{}'\nentry='main.py'\n", minor.version(), minor.version())).unwrap();
        let package = format!("{}/demo", minor.site_packages_root());
        let retained = [
            ("main.py".to_string(), "print('ENTRY')"),
            (format!("{package}/__init__.py"), "# INSTALLED_PACKAGE"),
            (format!("{package}/.env.defaults"), "PACKAGE_DEFAULTS"),
            (
                format!("{package}/templates/.env.example"),
                "PACKAGE_EXAMPLE",
            ),
            (format!("{package}/.env"), "PACKAGE_RESOURCE"),
        ];
        let excluded = [
            ".env",
            "app/.env.production",
            ".onreza/python/build/.env.defaults",
        ];
        for (path, body) in &retained {
            let file = project.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, body).unwrap();
        }
        for path in excluded {
            let file = project.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, "PROJECT_SECRET").unwrap();
        }
        let mut detection =
            crate::detect::detect_with_framework_override(project.path(), Some("python"));
        crate::detect::application_runtime::resolve_and_bind_detection(
            &crate::detect::fs::LocalFs::new(project.path()),
            &mut detection,
        )
        .unwrap();
        let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
            "version":1,"routes":[{"pattern":"^/.*$","layer":"python"}],"layers":[{"name":"python","target":"COMPUTE","directory":".","entry":"main.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":minor.target()}}]
        })).unwrap();
        let artifact = resolve_runtime_artifact(
            project.path(),
            project.path(),
            project.path().into(),
            manifest,
            &detection,
            true,
        )
        .unwrap();
        assert_python_scan_archive(&artifact, &detection, &retained, &excluded).await;
    }
}

async fn assert_python_scan_archive(
    artifact: &RuntimeArtifact,
    detection: &crate::detect::types::DetectionResult,
    retained: &[(String, &str)],
    excluded: &[&str],
) {
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    for (path, _) in retained {
        assert!(
            scanned.iter().any(|file| file.path == *path),
            "scanner lost {path}"
        );
    }
    let files = prepare_artifact_files(
        &artifact.manifest,
        scanned,
        detection,
        ArtifactRootScope::ProjectRoot,
        &artifact.scan,
        true,
    )
    .deployable_entries();
    let source = crate::artifact::source_bundle_v1::build_source_bundle_plan_with_scan(
        &artifact.root_dir,
        &artifact.manifest,
        &files,
        &artifact.scan,
        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let logical: nrz_source_bundle::SourceLogicalManifest =
        serde_json::from_value(serde_json::to_value(&source.logical_manifest).unwrap()).unwrap();
    crate::test_support::assert_source_bundle_verified(&source, &logical).await;
    nrz_runtime_artifact::validate_source_bundle_application_graph(
        &source.logical_manifest_sha256,
        &source.source_sha256,
        source.source_size_bytes,
        &logical,
    )
    .unwrap();
    let unpacked = tempfile::tempdir().unwrap();
    let decoder =
        zstd::stream::read::Decoder::new(std::fs::File::open(source.source_path()).unwrap())
            .unwrap();
    tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
    for (path, bytes) in retained {
        let file = logical
            .files
            .iter()
            .find(|file| file.path == *path)
            .expect("retained logical file");
        assert_eq!(file.layer_name.as_deref(), Some("python"));
        assert_eq!(
            file.role,
            if path.starts_with(".onreza/python/") {
                "dependency"
            } else {
                "compute"
            }
        );
        assert_eq!(file.sha256, sha256_hex(bytes.as_bytes()));
        assert_eq!(
            std::fs::read_to_string(unpacked.path().join(path)).unwrap(),
            *bytes
        );
    }
    for path in excluded {
        assert!(
            !logical.files.iter().any(|file| file.path == *path),
            "published {path}"
        );
        assert!(!unpacked.path().join(path).exists());
    }
    let entry = logical.layers[0].entrypoint.as_ref().unwrap();
    assert!(unpacked.path().join(entry).is_file());
}

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
