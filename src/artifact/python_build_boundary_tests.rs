use super::*;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn published_build_python_dependencies_follow_serving_owner() {
    use nrz_source_bundle::{ApplicationRuntimeFamily, PythonMinor};
    for (family, target, entry, program) in [
        ("NODE", "node-24", "server.js", "node"),
        ("BUN", "bun-1.4.2", "server.js", "bun"),
        ("PYTHON", "python-3.14", "main.py", "python3"),
    ] {
        for project_prefix in [".", "apps/site"] {
            if family == "PYTHON" && project_prefix != "." {
                continue;
            }
            let workspace = tempdir().unwrap();
            let project = workspace.path().join(project_prefix);
            fs::create_dir_all(&project).unwrap();
            fs::write(
                project.join("package.json"),
                r#"{"dependencies":{"demo":"1.0.0"}}"#,
            )
            .unwrap();
            fs::write(
                project.join(entry),
                if family == "PYTHON" {
                    "import demo\nprint(demo.VALUE)\n"
                } else {
                    "console.log(require('demo'))\n"
                },
            )
            .unwrap();
            fs::write(project.join("onreza.toml"), format!("[project]\nframework='other'\n[build]\ntoolchain='python'\npython_version='3.14'\noutput_directory='.'\n[deploy]\nruntime='{}'\nentry='{entry}'\n", family.to_ascii_lowercase())).unwrap();
            fs::create_dir_all(project.join("dist/.onreza/python/build")).unwrap();
            fs::write(
                project.join("dist/.onreza/python/build/authored.txt"),
                "AUTHORED_OUTPUT_ASSET",
            )
            .unwrap();
            let minor = PythonMinor::Python314;
            let stage = project.join(minor.site_packages_root());
            fs::create_dir_all(stage.join("demo/node_modules")).unwrap();
            fs::write(
                stage.join("demo/__init__.py"),
                "VALUE='PUBLISHED_DEPENDENCY'\n",
            )
            .unwrap();
            fs::write(
                stage.join("demo/node_modules/schema.json"),
                "LEGITIMATE_WHEEL_ASSET",
            )
            .unwrap();
            fs::write(
                project.join(".onreza/python/launch.py"),
                "print('UNUSED_GENERATED_BOOTSTRAP')\n",
            )
            .unwrap();
            fs::create_dir_all(workspace.path().join("node_modules/demo")).unwrap();
            fs::write(
                workspace.path().join("node_modules/demo/index.js"),
                "module.exports='PUBLISHED_DEPENDENCY'\n",
            )
            .unwrap();
            let mut detection = crate::detect::detect_with_framework_override(&project, None);
            crate::detect::application_runtime::resolve_and_bind_detection(
                &crate::detect::fs::LocalFs::new(&project),
                &mut detection,
            )
            .unwrap();
            assert_eq!(
                detection.metadata.runtime.runtime_type,
                crate::detect::types::RuntimeType::Python
            );
            assert_eq!(
                detection.metadata.application_runtime().unwrap().family,
                match family {
                    "NODE" => ApplicationRuntimeFamily::Node,
                    "BUN" => ApplicationRuntimeFamily::Bun,
                    _ => ApplicationRuntimeFamily::Python,
                }
            );
            let authored: Manifest = serde_json::from_value(serde_json::json!({
                "version":1,"routes":[],"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":entry,"runtime":{"applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":target}}]
            })).unwrap();
            let (manifest, scan) = if project_prefix == "." {
                (
                    authored,
                    if family == "PYTHON" {
                        RuntimeArtifactScan::PythonRuntimeRoot(minor)
                    } else {
                        RuntimeArtifactScan::NodeRuntimeRoot
                    },
                )
            } else {
                let mut relocated: Manifest =
                    serde_json::from_value(serde_json::to_value(&authored).unwrap()).unwrap();
                relocated.layers[0].entry = Some(format!("{project_prefix}/{entry}"));
                (
                    relocated,
                    RuntimeArtifactScan::Relocated {
                        base: Box::new(RuntimeArtifactScan::Selected {
                            roots: vec![
                                RuntimeArtifactScanRoot {
                                    path: project_prefix.into(),
                                    kind: RuntimeArtifactScanRootKind::BuildOutput,
                                },
                                RuntimeArtifactScanRoot {
                                    path: "node_modules".into(),
                                    kind: RuntimeArtifactScanRootKind::NodeModules,
                                },
                                RuntimeArtifactScanRoot {
                                    path: format!("{project_prefix}/package.json"),
                                    kind: RuntimeArtifactScanRootKind::Metadata,
                                },
                            ],
                            symlink_roots: vec!["node_modules".into()],
                        }),
                        ownership: RuntimeArtifactSourceOwnership {
                            build_output_prefix: project_prefix.into(),
                            layers: authored.layers,
                            javascript_dependency_owner: Some("server".into()),
                            python_dependency_owner: None,
                            python_primary_declared: false,
                        },
                    },
                )
            };
            let scanned = crate::deploy::scan_dir(workspace.path()).unwrap();
            let collection = classify_artifact_files(
                &manifest,
                scanned,
                &detection,
                ArtifactRootScope::ProjectRoot,
                &scan,
            );
            let source = source_bundle_v1::build_source_bundle_plan_with_scan(
                workspace.path(),
                &manifest,
                &collection.deployable_entries(),
                &scan,
                source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
                None,
            )
            .unwrap();
            let asset_path = if project_prefix == "." {
                "dist/.onreza/python/build/authored.txt".to_string()
            } else {
                format!("{project_prefix}/dist/.onreza/python/build/authored.txt")
            };
            let authored = source
                .logical_manifest
                .files
                .iter()
                .find(|file| file.path == asset_path)
                .expect("authored output assets must survive project state filtering");
            assert_eq!(authored.layer_name.as_deref(), Some("server"));
            let python_files = source
                .logical_manifest
                .files
                .iter()
                .filter(|file| {
                    file.path.starts_with(
                        if project_prefix == "." {
                            ".onreza/python/".to_string()
                        } else {
                            format!("{project_prefix}/.onreza/python/")
                        }
                        .as_str(),
                    )
                })
                .collect::<Vec<_>>();
            if family == "PYTHON" {
                assert_eq!(
                    python_files.len(),
                    2,
                    "{family}/{project_prefix}: {:?}",
                    python_files
                        .iter()
                        .map(|file| &file.path)
                        .collect::<Vec<_>>()
                );
                assert!(python_files.iter().all(|file| file.layer_name.as_deref()
                    == Some("server")
                    && file.role == source_bundle_v1::SourceLogicalManifestFileRole::Dependency));
            } else {
                assert!(
                    python_files.is_empty(),
                    "{family}/{project_prefix} shipped Python build state: {:?}",
                    python_files
                        .iter()
                        .map(|file| &file.path)
                        .collect::<Vec<_>>()
                );
                let node = source
                    .logical_manifest
                    .files
                    .iter()
                    .find(|file| file.path == "node_modules/demo/index.js")
                    .unwrap();
                assert_eq!(
                    node.role,
                    source_bundle_v1::SourceLogicalManifestFileRole::Dependency
                );
                assert_eq!(node.layer_name.as_deref(), Some("server"));
            }
            let logical = crate::test_support::validated_source_bundle_manifest(&source);
            crate::test_support::verify_source_bundle(&source, &logical).await;
            let unpacked = crate::test_support::unpack_source_bundle(&source);
            assert_eq!(
                fs::read_to_string(unpacked.path().join(&asset_path)).unwrap(),
                "AUTHORED_OUTPUT_ASSET"
            );
            if family != "PYTHON" {
                assert!(
                    !unpacked
                        .path()
                        .join(project_prefix)
                        .join(".onreza/python")
                        .exists()
                );
            }
            fs::remove_dir_all(workspace.path()).unwrap();
            let mut command = assert_cmd::Command::new(program);
            if program == "bun" {
                command.arg("--no-install");
            }
            let output = command
                .arg(logical.layers[0].entrypoint.as_deref().unwrap())
                .current_dir(unpacked.path())
                .env(
                    "PYTHONPATH",
                    unpacked.path().join(minor.site_packages_root()),
                )
                .env_remove("NODE_PATH")
                .timeout(std::time::Duration::from_secs(10))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().trim(),
                "PUBLISHED_DEPENDENCY"
            );
        }
    }
}
