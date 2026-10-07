use super::*;
use clap::Parser as _;
use nrz_source_bundle::{ApplicationRuntimeFamily, BuildToolchainFamily, PythonMinor};

#[tokio::test]
async fn code_only_python_plan_ignores_stale_unselected_minors_and_rejects_selected_dependencies() {
    for compiler in [
        BuildToolchainFamily::Node,
        BuildToolchainFamily::Bun,
        BuildToolchainFamily::Native,
    ] {
        for selected in PythonMinor::ALL {
            let project = tempfile::tempdir().unwrap();
            std::fs::write(project.path().join("main.py"), "print('CODE_ONLY')\n").unwrap();
            std::fs::write(
                project.path().join("requirements.txt"),
                "# no runtime dependencies\n",
            )
            .unwrap();
            for stale in PythonMinor::ALL
                .into_iter()
                .filter(|minor| *minor != selected)
            {
                let package = project.path().join(stale.site_packages_root()).join("old");
                std::fs::create_dir_all(&package).unwrap();
                std::fs::write(package.join("__init__.py"), "STALE_UNSELECTED_ABI").unwrap();
            }
            let mut config = nrz::config::ProjectConfig::default();
            config.build.toolchain = Some(compiler);
            config.build.output_directory = Some(".".into());
            config.deploy.runtime = Some(ApplicationRuntimeFamily::Python);
            config.deploy.python_version = Some(selected);
            config.deploy.entry = Some("main.py".into());
            std::fs::write(
                project.path().join("onreza.toml"),
                toml::to_string(&config).unwrap(),
            )
            .unwrap();
            let command = crate::context::CommandContext::resolve_platform_root(
                project.path(),
                &config,
                true,
            )
            .unwrap();
            let args = DeployArgs::try_parse_from([
                "deploy",
                project.path().to_str().unwrap(),
                "--dry",
                "--skip-build",
                "--skip-install",
            ])
            .unwrap();
            let request = || DeployPlanRequest {
                args: &args,
                command: &command,
                explicit_compute: None,
                build_logs: None,
                execution_env: &[],
                target_production: None,
                platform_runner: false,
            };
            let plan = build(request()).await.unwrap_or_else(|error| {
                panic!(
                    "stale ABI rejected {compiler:?}/{}: {error:#}",
                    selected.version()
                )
            });
            let source = plan
                .materialize_source_bundle(true, RuntimeDependencyPackaging::Embedded)
                .unwrap();
            assert!(
                source
                    .logical_manifest
                    .files
                    .iter()
                    .any(|file| file.path == "main.py")
            );
            assert!(source.logical_manifest.files.iter().all(|file| file.role
                != crate::artifact::source_bundle_v1::SourceLogicalManifestFileRole::Dependency));
            let unpacked = tempfile::tempdir().unwrap();
            let decoder = zstd::stream::read::Decoder::new(
                std::fs::File::open(source.source_path()).unwrap(),
            )
            .unwrap();
            tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
            assert_eq!(
                std::fs::read_to_string(unpacked.path().join("main.py")).unwrap(),
                "print('CODE_ONLY')\n"
            );
            for stale in PythonMinor::ALL
                .into_iter()
                .filter(|minor| *minor != selected)
            {
                assert!(!unpacked.path().join(stale.site_packages_root()).exists());
            }
            let active = project
                .path()
                .join(selected.site_packages_root())
                .join("active");
            std::fs::create_dir_all(&active).unwrap();
            std::fs::write(active.join("__init__.py"), "RETAINED_RUNTIME_DEPENDENCY").unwrap();
            let error = build(request())
                .await
                .err()
                .expect("selected dependencies require a Python installer ABI");
            assert!(
                error
                    .to_string()
                    .contains("matching build and serving Python minors"),
                "{error:#}"
            );
        }
    }
}
