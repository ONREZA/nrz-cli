use super::*;
use nrz_source_bundle::{ApplicationRuntimeFamily, BuildToolchainFamily, PythonMinor};

#[tokio::test]
async fn python_plan_retains_owned_dependencies_and_ignores_stale_unselected_minors() {
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
            config.deploy.python_version = Some("4.x".into());
            config.deploy.entry = Some("main.py".into());
            crate::deploy::test_support::write_project_config(project.path(), &config);
            let mut manifest = crate::build::manifest::generate_compute_manifest("main.py");
            let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
                family: ApplicationRuntimeFamily::Python,
                python_version: Some("4.x".into()),
                entry: Some("main.py".into()),
                args: vec![],
            };
            super::super::apply_application_runtime_manifest(
                &mut manifest,
                Some(&declaration),
                Some(&selected.target()),
                "python",
            )
            .unwrap();
            std::fs::create_dir_all(project.path().join(".onreza")).unwrap();
            std::fs::write(
                project.path().join(".onreza/manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            let (command, args) = crate::deploy::test_support::deploy_context(
                project.path(),
                &config,
                &["--skip-build", "--skip-install"],
            );
            let build_plan = || crate::deploy::test_support::build_plan(&args, &command, &[]);
            let plan = build_plan().await.unwrap_or_else(|error| {
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
            let unpacked = crate::test_support::unpack_source_bundle(&source);
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
            let source = build_plan()
                .await
                .unwrap()
                .materialize_source_bundle(true, RuntimeDependencyPackaging::Embedded)
                .unwrap();
            let unpacked = crate::test_support::unpack_source_bundle(&source);
            assert_eq!(
                std::fs::read_to_string(
                    unpacked
                        .path()
                        .join(selected.site_packages_root())
                        .join("active/__init__.py")
                )
                .unwrap(),
                "RETAINED_RUNTIME_DEPENDENCY"
            );
        }
    }
}
