use clap::Parser as _;
use nrz_source_bundle::{ApplicationRuntimeFamily, PythonMinor};

#[tokio::test]
async fn skipped_python_install_requires_staged_bare_requirement_dependencies() {
    let project = tempfile::tempdir().unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_directory = Some(".".into());
    config.deploy.runtime = Some(ApplicationRuntimeFamily::Python);
    config.deploy.python_version = Some(PythonMinor::Python314);
    config.deploy.entry = Some("main.py".into());
    std::fs::write(
        project.path().join("onreza.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(project.path().join("main.py"), "import acme\n").unwrap();
    let command =
        crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
            .unwrap();
    let args = crate::cli::DeployArgs::try_parse_from([
        "deploy",
        project.path().to_str().unwrap(),
        "--dry",
        "--skip-install",
        "--skip-build",
    ])
    .unwrap();
    let request = || super::super::plan::DeployPlanRequest {
        args: &args,
        command: &command,
        explicit_compute: None,
        build_logs: None,
        execution_env: &[],
        target_production: None,
        platform_runner: false,
    };
    for requirement in [
        "./wheels/acme-1.0-py3-none-any.whl",
        "../localproject",
        "/opt/localproject",
        "localproject/",
        "https://packages.example/acme.whl",
        "git+https://git.example/acme.git",
    ] {
        std::fs::write(project.path().join("requirements.txt"), requirement).unwrap();
        let error = super::super::plan::build(request()).await.err()
            .unwrap_or_else(|| panic!("{requirement}: dependency-free plan silently accepted an uninstalled requirement"));
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
    let plan = super::super::plan::build(request()).await.unwrap();
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
    let unpacked = tempfile::tempdir().unwrap();
    let decoder =
        zstd::stream::read::Decoder::new(std::fs::File::open(source.source_path()).unwrap())
            .unwrap();
    tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
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
    std::fs::write(
        project.path().join("onreza.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(project.path().join("main.py"), "print('DEPENDENCY_FREE')\n").unwrap();
    let command =
        crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
            .unwrap();
    let args = crate::cli::DeployArgs::try_parse_from([
        "deploy",
        project.path().to_str().unwrap(),
        "--dry",
        "--skip-install",
        "--skip-build",
    ])
    .unwrap();
    for requirements in [
        None,
        Some(""),
        Some("# no dependencies\n"),
        Some("--index-url https://packages.example/simple\n--no-index\n"),
    ] {
        if let Some(requirements) = requirements {
            std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
        }
        let plan = super::super::plan::build(super::super::plan::DeployPlanRequest {
            args: &args,
            command: &command,
            explicit_compute: None,
            build_logs: None,
            execution_env: &[],
            target_production: None,
            platform_runner: false,
        })
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
