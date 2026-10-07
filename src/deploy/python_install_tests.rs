use super::*;
#[cfg(unix)]
use clap::Parser as _;

#[cfg(unix)]
#[tokio::test]
async fn authored_python_install_cannot_publish_unqualified_native_dependencies() {
    let mut pe = [0; 128];
    pe[..2].copy_from_slice(b"MZ");
    pe[60..64].copy_from_slice(&64_u32.to_le_bytes());
    pe[64..68].copy_from_slice(b"PE\0\0");
    let mut elf = [0; 64];
    elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    elf[16..18].copy_from_slice(&3_u16.to_le_bytes());
    elf[18..20].copy_from_slice(&62_u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
    elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
    for minor in PythonMinor::ALL {
        for bytes in [
            b"\xcf\xfa\xed\xfeHOST_MACH_O".as_slice(),
            pe.as_slice(),
            elf.as_slice(),
        ] {
            let project = tempfile::tempdir().unwrap();
            std::fs::write(project.path().join("main.py"), "print('APP')\n").unwrap();
            std::fs::write(project.path().join("host.bin"), bytes).unwrap();
            let mut config = nrz::config::ProjectConfig::default();
            config.build.output_directory = Some(".".into());
            config.build.install_command = Some(format!(
                "mkdir -p {stage}/demo && cp host.bin {stage}/demo/native.data && printf INSTALLED > installer-marker",
                stage = minor.site_packages_root()
            ));
            config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Python);
            config.deploy.entry = Some("main.py".into());
            config.deploy.python_version = Some(minor);
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
            let args = crate::cli::DeployArgs::try_parse_from([
                "deploy",
                project.path().to_str().unwrap(),
                "--dry",
            ])
            .unwrap();
            let result = super::super::plan::build(super::super::plan::DeployPlanRequest {
                args: &args,
                command: &command,
                explicit_compute: None,
                build_logs: None,
                execution_env: &[],
                target_production: None,
                platform_runner: false,
            })
            .await;
            assert_eq!(
                std::fs::read(
                    project
                        .path()
                        .join(minor.site_packages_root())
                        .join("demo/native.data")
                )
                .unwrap(),
                bytes
            );
            assert!(
                project.path().join("installer-marker").is_file(),
                "authored installer was not executed"
            );
            let error = result.err().expect("custom install published unqualified reserved native dependencies without a Python build command");
            assert!(error.to_string().contains("native payload"), "{error:#}");
            assert!(
                error.to_string().contains("ONREZA Cloud Builder"),
                "{error:#}"
            );
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn authored_python_install_preserves_pure_prebuilt_and_static_stage_boundaries() {
    for minor in PythonMinor::ALL {
        for mode in ["pure", "skip-install", "skip-build", "empty", "static"] {
            let project = tempfile::tempdir().unwrap();
            std::fs::write(project.path().join("main.py"), "print('APP')\n").unwrap();
            // Target-platform header admission is deliberately separate from
            // runtime loading, qualified by the actual ELF fixture CLI checks.
            let mut target_elf = [0u8; 64];
            target_elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
            target_elf[16..18].copy_from_slice(&3u16.to_le_bytes()); // ET_DYN, entry0
            target_elf[18..20].copy_from_slice(&62u16.to_le_bytes()); // EM_X86_64
            target_elf[20..24].copy_from_slice(&1u32.to_le_bytes());
            target_elf[52..54].copy_from_slice(&64u16.to_le_bytes());
            let bytes = if mode == "pure" {
                b"VALUE='PURE'\n".as_slice()
            } else if mode == "static" {
                b"\xcf\xfa\xed\xfeSTATIC_DOWNLOAD".as_slice()
            } else {
                &target_elf
            };
            std::fs::write(project.path().join("host.bin"), bytes).unwrap();
            let relative = format!("{}/demo/native.data", minor.site_packages_root());
            let mut config = nrz::config::ProjectConfig::default();
            config.build.output_directory = Some(".".into());
            config.build.toolchain = Some(nrz_source_bundle::BuildToolchainFamily::Python);
            config.build.python_version = Some(minor);
            config.build.install_command = Some(if mode == "empty" {
                String::new()
            } else {
                format!(
                    "mkdir -p {stage}/demo && cp host.bin {stage}/demo/native.data && printf INSTALLED > installer-marker",
                    stage = minor.site_packages_root()
                )
            });
            if mode == "static" {
                config.deploy.compute = Some("static".into());
            } else {
                config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Python);
                config.deploy.python_version = Some(minor);
                config.deploy.entry = Some("main.py".into());
            }
            if matches!(mode, "skip-install" | "skip-build" | "empty") {
                std::fs::create_dir_all(project.path().join(&relative).parent().unwrap()).unwrap();
                std::fs::write(project.path().join(&relative), bytes).unwrap();
            }
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
            let mut args = crate::cli::DeployArgs::try_parse_from([
                "deploy",
                project.path().to_str().unwrap(),
                "--dry",
            ])
            .unwrap();
            args.skip_install = mode == "skip-install";
            args.skip_build = mode == "skip-build";
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
            assert_eq!(
                project.path().join("installer-marker").exists(),
                matches!(mode, "pure" | "static")
            );
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
                .find(|file| file.path == relative);
            if mode == "static" {
                assert!(
                    published.is_none(),
                    "Python compiler-only stage became serving dependencies"
                );
            } else {
                let published = published.expect("pure/prebuilt stage was omitted");
                assert_eq!(
                    published.role,
                    crate::artifact::source_bundle_v1::SourceLogicalManifestFileRole::Compute
                );
                let unpacked = tempfile::tempdir().unwrap();
                let decoder = zstd::stream::read::Decoder::new(
                    std::fs::File::open(source.source_path()).unwrap(),
                )
                .unwrap();
                tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
                assert_eq!(
                    std::fs::read(unpacked.path().join(&relative)).unwrap(),
                    bytes
                );
            }
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn authored_install_qualifies_python_sibling_dependencies_under_node_primary() {
    for compiler in [
        nrz_source_bundle::BuildToolchainFamily::Python,
        nrz_source_bundle::BuildToolchainFamily::Node,
    ] {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join("api")).unwrap();
        std::fs::write(
            project.path().join("api/server.js"),
            "console.log('NODE')\n",
        )
        .unwrap();
        std::fs::write(project.path().join("worker.py"), "print('PYTHON')\n").unwrap();
        std::fs::write(
            project.path().join("host.bin"),
            b"\xcf\xfa\xed\xfeHOST_MACH_O",
        )
        .unwrap();
        std::fs::create_dir(project.path().join(".onreza")).unwrap();
        std::fs::write(project.path().join(".onreza/manifest.json"),serde_json::to_vec(&serde_json::json!({
            "version":1,"routes":[{"pattern":"^/.*$","layer":"node"}],"layers":[
                {"name":"node","target":"COMPUTE","directory":"api","entry":"server.js","runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24"}},
                {"name":"python","target":"COMPUTE","directory":".","entry":"worker.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.14"}}
            ]
        })).unwrap()).unwrap();
        let mut config = nrz::config::ProjectConfig::default();
        config.build.toolchain = Some(compiler);
        config.build.output_directory = Some(".".into());
        config.build.install_command=Some("mkdir -p .onreza/python/3.14/site-packages/demo && cp host.bin .onreza/python/3.14/site-packages/demo/native.data && printf INSTALLED > installer-marker".into());
        config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Node);
        config.deploy.entry = Some("api/server.js".into());
        std::fs::write(
            project.path().join("onreza.toml"),
            toml::to_string(&config).unwrap(),
        )
        .unwrap();
        let mut command =
            crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
                .unwrap();
        command
            .effective
            .bind_admitted_node_version("NODE_24")
            .unwrap();
        let args = crate::cli::DeployArgs::try_parse_from([
            "deploy",
            project.path().to_str().unwrap(),
            "--dry",
        ])
        .unwrap();
        // The authored command never runs Node; isolate its version-probe
        // boundary so this custody regression does not require a host Node.
        use std::os::unix::fs::PermissionsExt as _;
        let tools = tempfile::tempdir().unwrap();
        std::fs::write(
            tools.path().join("node"),
            "#!/bin/sh\nprintf 'v24.12.0\\n'\n",
        )
        .unwrap();
        std::fs::set_permissions(
            tools.path().join("node"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        let path = std::env::join_paths(std::iter::once(tools.path().to_owned()).chain(
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
        ))
        .unwrap();
        let environment = vec![("PATH".to_string(), path.to_string_lossy().into_owned())];
        let error = super::super::plan::build(super::super::plan::DeployPlanRequest {
            args: &args,
            command: &command,
            explicit_compute: None,
            build_logs: None,
            execution_env: &environment,
            target_production: None,
            platform_runner: false,
        })
        .await
        .err()
        .expect("Node primary erased Python dependency qualification");
        assert!(
            project.path().join("installer-marker").exists(),
            "custom install was not executed"
        );
        if compiler == nrz_source_bundle::BuildToolchainFamily::Python {
            assert!(error.to_string().contains("native payload"), "{error:#}");
        } else {
            assert!(
                error
                    .to_string()
                    .contains("matching build and serving Python minors"),
                "{error:#}"
            );
        }
    }
}

#[test]
fn authored_python_dependency_guard_preserves_frozen_owners_and_builder_mode() {
    use crate::artifact::{ArtifactRootScope, RuntimeArtifact, RuntimeArtifactScan};
    for minor in PythonMinor::ALL {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("worker.py"), "print('WORKER')\n").unwrap();
        let relative = format!("{}/demo/native.data", minor.site_packages_root());
        std::fs::create_dir_all(project.path().join(&relative).parent().unwrap()).unwrap();
        std::fs::write(project.path().join(&relative), b"\xcf\xfa\xed\xfeNATIVE").unwrap();
        let manifest=serde_json::from_value(serde_json::json!({
            "version":1,"routes":[],"layers":[
                {"name":"node","target":"COMPUTE","directory":"api","entry":"server.js","runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24"}},
                {"name":"python","target":"COMPUTE","directory":".","entry":"worker.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":minor.target()}}
            ]
        })).unwrap();
        let artifact = RuntimeArtifact {
            root_dir: project.path().into(),
            manifest,
            scan: RuntimeArtifactScan::PythonRuntimeRoot(minor),
        };
        let scanned =
            super::super::scan::scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
        let files = crate::artifact::classify_artifact_files(
            &artifact.manifest,
            scanned,
            &crate::detect::detect(project.path()),
            ArtifactRootScope::ProjectRoot,
            &artifact.scan,
        );
        assert!(validate_retained_python_native_platform(&artifact, &files).is_err());
        assert!(
            validate_authored_python_dependency_output(
                &artifact,
                &files,
                PythonInstallMode::ManagedLocal
            )
            .is_err()
        );
        assert!(
            validate_authored_python_dependency_output(
                &artifact,
                &files,
                PythonInstallMode::PinnedPlatform
            )
            .is_ok()
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn retained_python_native_platform_is_checked_without_install_or_build() {
    for minor in PythonMinor::ALL {
        for placement in ["dependency", "application"] {
            for invocation in ["skip-install", "skip-build", "empty-install"] {
                let project = tempfile::tempdir().unwrap();
                std::fs::write(project.path().join("main.py"), "print('APP')\n").unwrap();
                let relative = if placement == "dependency" {
                    format!("{}/demo/native.data", minor.site_packages_root())
                } else {
                    "native.data".to_string()
                };
                std::fs::create_dir_all(project.path().join(&relative).parent().unwrap()).unwrap();
                std::fs::write(
                    project.path().join(&relative),
                    b"\xcf\xfa\xed\xfeFOREIGN_MACH_O",
                )
                .unwrap();
                let requirements = if placement == "dependency" && invocation == "skip-install" {
                    "colorama; sys_platform == 'win32'\n"
                } else if placement == "dependency" && invocation == "empty-install" {
                    "demo==1.0\n"
                } else {
                    ""
                };
                std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
                let mut config = nrz::config::ProjectConfig::default();
                config.build.output_directory = Some(".".into());
                config.build.python_version = Some(minor);
                config.build.install_command = Some(if invocation == "empty-install" {
                    String::new()
                } else {
                    "printf UNEXPECTED_INSTALL > installer-marker".into()
                });
                config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Python);
                config.deploy.python_version = Some(minor);
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
                let mut args = crate::cli::DeployArgs::try_parse_from([
                    "deploy",
                    project.path().to_str().unwrap(),
                    "--dry",
                ])
                .unwrap();
                args.skip_install = invocation == "skip-install";
                args.skip_build = invocation == "skip-build";
                let error = super::super::plan::build(super::super::plan::DeployPlanRequest {
                    args: &args, command: &command, explicit_compute: None, build_logs: None,
                    execution_env: &[], target_production: None, platform_runner: false,
                }).await.err().unwrap_or_else(|| panic!("{minor:?}/{placement}/{invocation}: foreign native payload published without any installer/build execution"));
                assert!(!project.path().join("installer-marker").exists());
                assert!(
                    error
                        .chain()
                        .filter_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
                        .any(|coded| coded.code == "PYTHON_PLATFORM_UNSUPPORTED"),
                    "{minor:?}/{placement}/{invocation}: {error:#}"
                );
            }
        }
    }
}
