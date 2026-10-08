use nrz_source_bundle::{ApplicationRuntimeFamily, PythonMinor};

#[tokio::test]
async fn skipped_python_install_requires_staged_bare_requirement_dependencies() {
    let project = tempfile::tempdir().unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_directory = Some(".".into());
    config.deploy.runtime = Some(ApplicationRuntimeFamily::Python);
    config.deploy.python_version = Some(PythonMinor::Python314);
    config.deploy.entry = Some("main.py".into());
    crate::deploy::test_support::write_project_config(project.path(), &config);
    std::fs::write(project.path().join("main.py"), "import acme\n").unwrap();
    let (command, args) = crate::deploy::test_support::deploy_context(
        project.path(),
        &config,
        &["--skip-install", "--skip-build"],
    );
    let build_plan = || crate::deploy::test_support::build_plan(&args, &command, &[]);
    for requirement in [
        "./wheels/acme-1.0-py3-none-any.whl",
        "../localproject",
        "/opt/localproject",
        "localproject/",
        "https://packages.example/acme.whl",
        "git+https://git.example/acme.git",
    ] {
        std::fs::write(project.path().join("requirements.txt"), requirement).unwrap();
        let error = build_plan().await.err().unwrap_or_else(|| {
            panic!(
                "{requirement}: dependency-free plan silently accepted an uninstalled requirement"
            )
        });
        assert!(
            error
                .chain()
                .filter_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
                .any(|coded| coded.code == "MISSING_RUNTIME_DEPENDENCIES"),
            "{requirement}: {error:#}"
        );
    }
    let package = project
        .path()
        .join(PythonMinor::Python314.site_packages_root())
        .join("acme");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("__init__.py"),
        "VALUE = 'PUBLISHED_REQUIREMENT'\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join("requirements.txt"),
        "./wheels/acme-1.0-py3-none-any.whl",
    )
    .unwrap();
    let plan = build_plan().await.unwrap();
    let source = plan
        .materialize_source_bundle(
            true,
            crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
        )
        .unwrap();
    let published = source
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path.ends_with("site-packages/acme/__init__.py"))
        .expect("preinstalled local requirement must remain in the published dependency closure");
    assert_eq!(
        published.role,
        crate::artifact::source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
    assert_eq!(published.layer_name.as_deref(), Some("server"));
    let unpacked = crate::test_support::unpack_source_bundle(&source);
    assert_eq!(
        std::fs::read_to_string(unpacked.path().join(&published.path)).unwrap(),
        "VALUE = 'PUBLISHED_REQUIREMENT'\n"
    );
}

#[tokio::test]
async fn dependency_free_python_requirements_preserve_skip_install_publication() {
    let project = tempfile::tempdir().unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_directory = Some(".".into());
    config.deploy.runtime = Some(ApplicationRuntimeFamily::Python);
    config.deploy.entry = Some("main.py".into());
    crate::deploy::test_support::write_project_config(project.path(), &config);
    std::fs::write(project.path().join("main.py"), "print('DEPENDENCY_FREE')\n").unwrap();
    let (command, args) = crate::deploy::test_support::deploy_context(
        project.path(),
        &config,
        &["--skip-install", "--skip-build"],
    );
    for requirements in [
        None,
        Some(""),
        Some("# no dependencies\n"),
        Some("--index-url https://packages.example/simple\n--no-index\n"),
    ] {
        if let Some(requirements) = requirements {
            std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
        }
        let plan = crate::deploy::test_support::build_plan(&args, &command, &[])
            .await
            .unwrap();
        let source = plan
            .materialize_source_bundle(
                true,
                crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
            )
            .unwrap();
        assert!(source.logical_manifest.files.iter().all(|file| file.role
            != crate::artifact::source_bundle_v1::SourceLogicalManifestFileRole::Dependency));
        assert!(
            source
                .logical_manifest
                .files
                .iter()
                .any(|file| file.path == "main.py")
        );
    }
}

fn staged_requirement_project(
    minor: PythonMinor,
    output: &str,
) -> (
    tempfile::TempDir,
    crate::context::CommandContext,
    crate::cli::DeployArgs,
) {
    let project = tempfile::tempdir().unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_directory = Some(output.into());
    config.deploy.runtime = Some(ApplicationRuntimeFamily::Python);
    config.deploy.python_version = Some(minor);
    config.deploy.entry = Some("main.py".into());
    std::fs::create_dir_all(project.path().join(output)).unwrap();
    std::fs::write(project.path().join(output).join("main.py"), "import acme\n").unwrap();
    std::fs::write(project.path().join("requirements.txt"), "acme==1.0\n").unwrap();
    crate::deploy::test_support::write_project_config(project.path(), &config);
    let (command, args) = crate::deploy::test_support::deploy_context(
        project.path(),
        &config,
        &["--skip-install", "--skip-build"],
    );
    (project, command, args)
}

#[tokio::test]
async fn target_excluded_python_requirements_publish_without_a_dependency_stage() {
    for minor in PythonMinor::ALL {
        for output in [".", "dist"] {
            for selectors in [
                None,
                Some("platform='win32'"),
                Some("platform='freebsd'"),
                Some("python='<3.12'"),
            ] {
                let (project, command, args) = staged_requirement_project(minor, output);
                std::fs::write(
                    project.path().join("requirements.txt"),
                    "colorama; sys_platform == 'win32'\n",
                )
                .unwrap();
                if let Some(selectors) = selectors {
                    std::fs::remove_file(project.path().join("requirements.txt")).unwrap();
                    std::fs::write(project.path().join("pyproject.toml"), format!("[tool.poetry]\nname='app'\npackage-mode=false\n[tool.poetry.dependencies]\npython='*'\ncolorama={{version='*', {selectors}}}\n")).unwrap();
                    std::fs::write(project.path().join("poetry.lock"), "").unwrap();
                }
                std::fs::write(
                    project.path().join(output).join("main.py"),
                    "print('TARGET_DEPENDENCY_FREE')\n",
                )
                .unwrap();
                let plan = crate::deploy::test_support::build_plan(&args, &command, &[])
                    .await
                    .unwrap();
                let source = plan
                    .materialize_source_bundle(
                        true,
                        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
                    )
                    .unwrap();
                assert!(
                    source
                        .logical_manifest
                        .files
                        .iter()
                        .all(|file| !file.path.contains("site-packages/"))
                );
                let unpacked = crate::test_support::unpack_source_bundle(&source);
                let entry = &source.logical_manifest.entrypoints[0];
                assert_eq!(
                    std::fs::read_to_string(unpacked.path().join(entry)).unwrap(),
                    "print('TARGET_DEPENDENCY_FREE')\n"
                );
                let launch = nrz_runtime_artifact::python_script_launch_arguments(
                    entry,
                    minor.site_packages_root(),
                    ".",
                );
                let execution = std::process::Command::new("python3")
                    .args(launch)
                    .current_dir(unpacked.path())
                    .output()
                    .unwrap();
                assert!(
                    execution.status.success(),
                    "{}",
                    String::from_utf8_lossy(&execution.stderr)
                );
                assert_eq!(
                    String::from_utf8_lossy(&execution.stdout),
                    "TARGET_DEPENDENCY_FREE\n"
                );
            }
        }
    }
}

#[tokio::test]
async fn skipped_python_install_rejects_stages_without_retained_files() {
    for minor in PythonMinor::ALL {
        for output in [".", "dist"] {
            for state in ["empty", "directories", "vcs-only"] {
                let (project, command, args) = staged_requirement_project(minor, output);
                let stage = project.path().join(minor.site_packages_root());
                std::fs::create_dir_all(&stage).unwrap();
                match state {
                    "directories" => std::fs::create_dir_all(stage.join("acme/empty")).unwrap(),
                    "vcs-only" => {
                        std::fs::create_dir(stage.join(".git")).unwrap();
                        std::fs::write(stage.join(".git/metadata"), "EXCLUDED").unwrap();
                    }
                    _ => {}
                }
                let error = crate::deploy::test_support::build_plan(&args, &command, &[]).await.err().unwrap_or_else(|| panic!("{minor:?}/{output}/{state}: published a required stage with no retained files"));
                assert!(
                    error
                        .chain()
                        .filter_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
                        .any(|coded| coded.code == "MISSING_RUNTIME_DEPENDENCIES"),
                    "{minor:?}/{output}/{state}: {error:#}"
                );
            }
        }
    }
}

#[tokio::test]
async fn prebuilt_python_stage_retains_regular_metadata_cache_and_link_targets() {
    for minor in PythonMinor::ALL {
        for output in [".", "dist"] {
            let states = [
                "populated",
                "empty-file",
                "metadata-only",
                "cache-only",
                #[cfg(unix)]
                "stage-link",
                #[cfg(unix)]
                "member-link",
            ];
            for state in states {
                let (project, command, args) = staged_requirement_project(minor, output);
                let stage = project.path().join(minor.site_packages_root());
                if state != "stage-link" {
                    std::fs::create_dir_all(&stage).unwrap();
                }
                match state {
                    "populated" => {
                        std::fs::create_dir(stage.join("acme")).unwrap();
                        std::fs::write(stage.join("acme/__init__.py"), "VALUE='RETAINED'\n")
                            .unwrap();
                    }
                    "empty-file" => std::fs::write(stage.join("acme.py"), "").unwrap(),
                    "metadata-only" => {
                        std::fs::create_dir(stage.join("acme.dist-info")).unwrap();
                        std::fs::write(
                            stage.join("acme.dist-info/METADATA"),
                            "Name: acme\nVersion: 1.0\n",
                        )
                        .unwrap();
                    }
                    "cache-only" => {
                        std::fs::create_dir(stage.join("__pycache__")).unwrap();
                        std::fs::write(stage.join("__pycache__/acme.pyc"), "RETAINED_CACHE")
                            .unwrap();
                    }
                    #[cfg(unix)]
                    "stage-link" | "member-link" => {
                        std::fs::create_dir_all(project.path().join("vendor/acme")).unwrap();
                        std::fs::write(
                            project.path().join("vendor/acme/__init__.py"),
                            "VALUE='RETAINED'\n",
                        )
                        .unwrap();
                        if state == "stage-link" {
                            std::fs::create_dir_all(stage.parent().unwrap()).unwrap();
                            std::os::unix::fs::symlink("../../../vendor", &stage).unwrap();
                        } else {
                            std::os::unix::fs::symlink(
                                "../../../../vendor/acme",
                                stage.join("acme"),
                            )
                            .unwrap();
                        }
                    }
                    _ => unreachable!(),
                }
                let plan = crate::deploy::test_support::build_plan(&args, &command, &[])
                    .await
                    .unwrap_or_else(|error| panic!("{minor:?}/{output}/{state}: {error:#}"));
                let source = plan
                    .materialize_source_bundle(
                        true,
                        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
                    )
                    .unwrap();
                let logical: nrz_source_bundle::SourceLogicalManifest =
                    serde_json::from_value(serde_json::to_value(&source.logical_manifest).unwrap())
                        .unwrap();
                let input =
                    crate::test_support::source_bundle_verification_input(&source, &logical);
                nrz_source_bundle::verify_source_bundle_bytes(
                    input,
                    std::fs::read(source.source_path()).unwrap().into(),
                )
                .await
                .unwrap_or_else(|error| panic!("{minor:?}/{output}/{state}: {error:#}"));
                assert!(plan.artifact.files.files.iter().any(|file| {
                    file.kind == crate::artifact::ArtifactFileKind::File
                        && file.role == crate::artifact::ArtifactFileRole::Compute
                        && file.layer.as_deref() == Some("server")
                        && (file
                            .path
                            .starts_with(&format!("{}/", minor.site_packages_root()))
                            || file.path == "vendor/acme/__init__.py")
                }));
            }
        }
    }
}
