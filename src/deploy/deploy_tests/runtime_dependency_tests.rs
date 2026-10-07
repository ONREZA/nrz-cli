use super::*;

#[tokio::test]
async fn mixed_python_javascript_published_bundle_keeps_actual_dependency_roots() {
    for (family, target, executable) in [("NODE", "node-24", "node"), ("BUN", "bun-1.4.2", "bun")] {
        for (dependency_directory, js_name, python_name) in [
            ("apps/site/dist/api/node_modules", "api", "side"),
            ("apps/site/dist/api/node_modules", "zzz-node", "aaa-python"),
            ("apps/site/dist/node_modules", "api", "side"),
            ("apps/site/dist/node_modules", "zzz-node", "aaa-python"),
            ("apps/site/node_modules", "api", "side"),
            ("apps/site/node_modules", "zzz-node", "aaa-python"),
            ("node_modules", "api", "side"),
            ("node_modules", "zzz-node", "aaa-python"),
        ] {
            let workspace = tempdir().unwrap();
            let project = workspace.path().join("apps/site");
            let js_directory = if dependency_directory == "apps/site/dist/node_modules" {
                "."
            } else {
                "api"
            };
            fs::create_dir_all(project.join("dist/api")).unwrap();
            fs::create_dir_all(project.join("dist/side")).unwrap();
            fs::create_dir_all(project.join("dist/public")).unwrap();
            fs::write(
                project.join("package.json"),
                r#"{"dependencies":{"demo":"1.0.0"}}"#,
            )
            .unwrap();
            fs::write(
                project.join("dist").join(js_directory).join("server.js"),
                "console.log(require('demo'))\n",
            )
            .unwrap();
            fs::write(project.join("dist/side/main.py"), "print('CODE_ONLY')\n").unwrap();
            fs::write(project.join("dist/public/index.html"), "STATIC_ASSET").unwrap();
            let dependencies = workspace.path().join(dependency_directory);
            fs::create_dir_all(dependencies.join("demo")).unwrap();
            fs::write(
                dependencies.join("demo/package.json"),
                r#"{"name":"demo","version":"1.0.0","main":"index.js"}"#,
            )
            .unwrap();
            fs::write(
                dependencies.join("demo/index.js"),
                "module.exports = 'PUBLISHED_JS_DEPENDENCY'\n",
            )
            .unwrap();
            if dependency_directory == "apps/site/dist/node_modules" {
                for (root, value) in [
                    ("apps/site/node_modules/project-helper", "PUBLISHED_JS_"),
                    ("node_modules/workspace-helper", "DEPENDENCY"),
                ] {
                    fs::create_dir_all(workspace.path().join(root)).unwrap();
                    fs::write(
                        workspace.path().join(root).join("index.js"),
                        format!("module.exports = '{value}'\n"),
                    )
                    .unwrap();
                }
                fs::write(
                    dependencies.join("demo/index.js"),
                    "module.exports = require('project-helper') + require('workspace-helper')\n",
                )
                .unwrap();
            }
            let detection = make_detection("other", None);
            let mut manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
                "version":1, "routes":[{"pattern":"^/.*$", "layer":js_name}], "layers":[
                    {"name":js_name,"target":"COMPUTE","directory":js_directory,"entry":"server.js","runtime":{"applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":target}},
                    {"name":python_name,"target":"COMPUTE","directory":"side","entry":"main.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.12"}},
                    {"name":"public","target":"STATIC","directory":"public"}
                ]
            })).unwrap();
            let artifact = resolve_runtime_artifact(
                workspace.path(),
                &project,
                project.join("dist"),
                serde_json::from_value(serde_json::to_value(&manifest).unwrap()).unwrap(),
                &detection,
                true,
            )
            .unwrap();
            let files = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
            let source = source_bundle_v1::build_source_bundle_plan_with_scan(
                &artifact.root_dir,
                &artifact.manifest,
                &files,
                &artifact.scan,
                source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
                None,
            )
            .unwrap();
            let dependency_path = relative_runtime_artifact_path(
                &artifact.root_dir,
                &dependencies.join("demo/index.js"),
            )
            .unwrap();
            let dependency = source
                .logical_manifest
                .files
                .iter()
                .find(|file| file.path == dependency_path)
                .expect("actual JS dependency bytes must be included in the published bundle");
            assert_eq!(
                dependency.role,
                source_bundle_v1::SourceLogicalManifestFileRole::Dependency
            );
            assert_eq!(dependency.layer_name.as_deref(), Some(js_name));
            let html = source
                .logical_manifest
                .files
                .iter()
                .find(|file| file.path.ends_with("public/index.html"))
                .unwrap();
            assert_eq!(
                html.role,
                source_bundle_v1::SourceLogicalManifestFileRole::Static
            );
            assert_eq!(html.layer_name.as_deref(), Some("public"));
            let logical: nrz_source_bundle::SourceLogicalManifest =
                serde_json::from_value(serde_json::to_value(&source.logical_manifest).unwrap())
                    .unwrap();
            nrz_runtime_artifact::validate_source_bundle_application_graph(
                &source.logical_manifest_sha256,
                &source.source_sha256,
                source.source_size_bytes,
                &logical,
            )
            .unwrap();
            let owner = uuid::Uuid::nil().to_string();
            nrz_source_bundle::verify_source_bundle_bytes(
                nrz_source_bundle::SourceBundleVerificationInput {
                    owner_workspace_id: owner.clone(),
                    source_artifact_id: nrz_source_bundle::compute_source_artifact_id(
                        &owner,
                        &source.logical_manifest_sha256,
                        &source.source_sha256,
                        None,
                    ),
                    source_sha256: source.source_sha256.clone(),
                    logical_manifest_sha256: source.logical_manifest_sha256.clone(),
                    budget: nrz_source_bundle::SourceBundleVerificationBudget::from_manifest(
                        &logical,
                    )
                    .unwrap(),
                },
                fs::read(source.source_path()).unwrap().into(),
            )
            .await
            .unwrap();
            let entry = logical
                .layers
                .iter()
                .find(|layer| layer.name == js_name)
                .unwrap()
                .entrypoint
                .as_deref()
                .unwrap();
            let unpacked = tempdir().unwrap();
            let decoder =
                zstd::stream::read::Decoder::new(fs::File::open(source.source_path()).unwrap())
                    .unwrap();
            tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
            if dependency_directory == "apps/site/dist/api/node_modules" {
                fs::create_dir_all(project.join("dist/extra")).unwrap();
                fs::write(
                    project.join("dist/extra/server.js"),
                    "console.log('CODE_ONLY')\n",
                )
                .unwrap();
                manifest.layers.push(serde_json::from_value(serde_json::json!({
                    "name":"zzzz-extra","target":"COMPUTE","directory":"extra","entry":"server.js",
                    "runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-22"}
                })).unwrap());
                fs::create_dir_all(project.join("dist/extra/node_modules/independent")).unwrap();
                fs::write(
                    project.join("dist/extra/node_modules/independent/index.js"),
                    "module.exports = 1",
                )
                .unwrap();
                let error = resolve_runtime_artifact(
                    workspace.path(),
                    &project,
                    project.join("dist"),
                    manifest,
                    &detection,
                    true,
                )
                .unwrap_err();
                expect_code(&error, "APPLICATION_RUNTIME_INVALID");
                assert!(
                    error.to_string().contains("single owning layer"),
                    "{error:#}"
                );
            }
            fs::remove_dir_all(workspace.path()).unwrap();
            let mut command = assert_cmd::Command::new(executable);
            if executable == "bun" {
                command.arg("--no-install");
            }
            let output = command
                .arg(entry)
                .current_dir(unpacked.path())
                .env_remove("NODE_PATH")
                .env("BUN_INSTALL_CACHE_DIR", unpacked.path().join("empty-cache"))
                .timeout(std::time::Duration::from_secs(10))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{family} {dependency_directory}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().trim(),
                "PUBLISHED_JS_DEPENDENCY"
            );
        }
    }
}

#[tokio::test]
async fn relocated_python_dependencies_keep_frozen_primary_owner() {
    use nrz_source_bundle::{
        ApplicationRuntimeDeclaration, ApplicationRuntimeFamily, BuildToolchainDeclaration,
        BuildToolchainFamily, PythonMinor, SourceBuildContext,
    };
    for (primary_name, sibling_name) in [
        ("zzz-primary", "aaa-sibling"),
        ("aaa-primary", "zzz-sibling"),
    ] {
        for sibling_target in ["python-3.14", "python-3.12"] {
            for reverse_order in [false, true] {
                for output_prefix in ["dist", "."] {
                    for scope in ["scoped", "equal", "primary-root", "sibling-root"] {
                        let project = tempdir().unwrap();
                        let output_dir = project.path().join(output_prefix);
                        let prefix = if output_prefix == "." { "" } else { "dist/" };
                        let api_entry = format!("{prefix}api/main.py");
                        let side_entry = format!("{prefix}side/main.py");
                        let static_entry = format!("{prefix}public/index.html");
                        for directory in ["api", "side", "public"] {
                            fs::create_dir_all(output_dir.join(directory)).unwrap();
                        }
                        fs::write(
                            output_dir.join("api/main.py"),
                            "import demo\nprint(demo.VALUE)\n",
                        )
                        .unwrap();
                        fs::write(output_dir.join("side/main.py"), "print('CODE_ONLY')\n").unwrap();
                        fs::write(output_dir.join("public/index.html"), "STATIC_ASSET").unwrap();
                        let minor = PythonMinor::Python314;
                        let dependencies = project.path().join(minor.site_packages_root());
                        fs::create_dir_all(&dependencies).unwrap();
                        fs::write(
                            dependencies.join("demo.py"),
                            "VALUE='PUBLISHED_PYTHON_DEPENDENCY'\n",
                        )
                        .unwrap();
                        // An unselected previous build does not claim a code-only sibling's ABI.
                        let stale = project
                            .path()
                            .join(PythonMinor::Python312.site_packages_root());
                        fs::create_dir_all(&stale).unwrap();
                        fs::write(stale.join("stale.py"), "VALUE='STALE'\n").unwrap();
                        let mut detection = make_detection("other", None);
                        detection.metadata.source_build_context = Some(SourceBuildContext {
                            schema_version: 1,
                            build_toolchain: BuildToolchainDeclaration {
                                family: BuildToolchainFamily::Python,
                                python_version: Some(minor),
                            },
                            application_runtime: Some(ApplicationRuntimeDeclaration {
                                family: ApplicationRuntimeFamily::Python,
                                python_version: Some(minor),
                                entry: Some(api_entry.clone()),
                                args: vec![],
                            }),
                        });
                        let mut manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
                    "version":1,"routes":[{"pattern":"^/.*$","layer":primary_name}],"layers":[
                        {"name":primary_name,"target":"COMPUTE","directory":if matches!(scope, "equal" | "primary-root") { "." } else { "api" },"entry":if matches!(scope, "equal" | "primary-root") { "api/main.py" } else { "main.py" },"runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.14"}},
                        {"name":sibling_name,"target":"COMPUTE","directory":if matches!(scope, "equal" | "sibling-root") { "." } else { "side" },"entry":if matches!(scope, "equal" | "sibling-root") { "side/main.py" } else { "main.py" },"runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":sibling_target}},
                        {"name":"public","target":"STATIC","directory":"public"}
                    ]
                })).unwrap();
                        if reverse_order {
                            manifest.layers.swap(0, 1);
                        }
                        let artifact = resolve_runtime_artifact(
                            project.path(),
                            project.path(),
                            output_dir,
                            manifest,
                            &detection,
                            true,
                        )
                        .unwrap();
                        let files =
                            scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
                        let source = source_bundle_v1::build_source_bundle_plan_with_scan(
                            &artifact.root_dir,
                            &artifact.manifest,
                            &files,
                            &artifact.scan,
                            source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
                            None,
                        )
                        .unwrap();
                        let dependency = source
                            .logical_manifest
                            .files
                            .iter()
                            .find(|file| file.path.ends_with("site-packages/demo.py"))
                            .unwrap();
                        assert_eq!(
                            dependency.layer_name.as_deref(),
                            Some(primary_name),
                            "{sibling_target}, reverse={reverse_order}, output={output_prefix}, scope={scope}"
                        );
                        assert_eq!(
                            dependency.role,
                            source_bundle_v1::SourceLogicalManifestFileRole::Dependency
                        );
                        assert!(
                            !source
                                .logical_manifest
                                .files
                                .iter()
                                .any(|file| file.path.ends_with("stale.py"))
                        );
                        for (path, owner) in [
                            (api_entry.as_str(), primary_name),
                            (side_entry.as_str(), sibling_name),
                            (static_entry.as_str(), "public"),
                        ] {
                            let file = source
                                .logical_manifest
                                .files
                                .iter()
                                .find(|file| file.path == path)
                                .unwrap();
                            assert_eq!(file.layer_name.as_deref(), Some(owner));
                        }
                        let logical: nrz_source_bundle::SourceLogicalManifest =
                            serde_json::from_value(
                                serde_json::to_value(&source.logical_manifest).unwrap(),
                            )
                            .unwrap();
                        nrz_runtime_artifact::validate_source_bundle_application_graph(
                            &source.logical_manifest_sha256,
                            &source.source_sha256,
                            source.source_size_bytes,
                            &logical,
                        )
                        .unwrap();
                        for layer in logical
                            .layers
                            .iter()
                            .filter(|layer| layer.target == "COMPUTE")
                        {
                            let target =
                                layer.runtime_config.as_ref().unwrap()["buildRuntimeVersion"]
                                    .as_str()
                                    .unwrap();
                            let runtime =
                                nrz_runtime_artifact::compile_source_runtime_layer_for_target(
                                    layer,
                                    &[],
                                    Some(target),
                                )
                                .unwrap();
                            let entry = join_runtime_artifact_paths(
                                runtime["applicationRoot"].as_str().unwrap(),
                                runtime["entrypoint"].as_str().unwrap(),
                            )
                            .unwrap();
                            assert_eq!(Some(entry.as_str()), layer.entrypoint.as_deref());
                        }
                        let owner = uuid::Uuid::nil().to_string();
                        nrz_source_bundle::verify_source_bundle_bytes(
                    nrz_source_bundle::SourceBundleVerificationInput {
                        owner_workspace_id: owner.clone(),
                        source_artifact_id: nrz_source_bundle::compute_source_artifact_id(
                            &owner,
                            &source.logical_manifest_sha256,
                            &source.source_sha256,
                            None,
                        ),
                        source_sha256: source.source_sha256.clone(),
                        logical_manifest_sha256: source.logical_manifest_sha256.clone(),
                        budget: nrz_source_bundle::SourceBundleVerificationBudget::from_manifest(
                            &logical,
                        )
                        .unwrap(),
                    },
                    fs::read(source.source_path()).unwrap().into(),
                )
                .await
                .unwrap();
                        let unpacked = tempdir().unwrap();
                        tar::Archive::new(
                            zstd::stream::read::Decoder::new(
                                fs::File::open(source.source_path()).unwrap(),
                            )
                            .unwrap(),
                        )
                        .unpack(unpacked.path())
                        .unwrap();
                        fs::remove_dir_all(project.path()).unwrap();
                        let output = assert_cmd::Command::new("python3")
                            .arg(&api_entry)
                            .current_dir(unpacked.path())
                            .env(
                                "PYTHONPATH",
                                unpacked.path().join(minor.site_packages_root()),
                            )
                            .env("PYTHONNOUSERSITE", "1")
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
                            "PUBLISHED_PYTHON_DEPENDENCY"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn relocated_python_dependencies_refuse_ambiguous_or_incompatible_primary() {
    use nrz_source_bundle::{
        ApplicationRuntimeDeclaration, ApplicationRuntimeFamily, BuildToolchainDeclaration,
        BuildToolchainFamily, PythonMinor, SourceBuildContext,
    };
    for output_prefix in ["dist", "."] {
        let project = tempdir().unwrap();
        let output_dir = project.path().join(output_prefix);
        let prefix = if output_prefix == "." { "" } else { "dist/" };
        for directory in [
            &format!("{prefix}api"),
            &format!("{prefix}side"),
            PythonMinor::Python314.site_packages_root(),
        ] {
            fs::create_dir_all(project.path().join(directory)).unwrap();
        }
        fs::write(output_dir.join("api/main.py"), "import demo").unwrap();
        fs::write(output_dir.join("side/main.py"), "print(42)").unwrap();
        fs::write(
            project
                .path()
                .join(PythonMinor::Python314.site_packages_root())
                .join("demo.py"),
            "VALUE=42",
        )
        .unwrap();
        for scope in ["equal", "primary-root", "sibling-root"] {
            for primary_entry in [
                None,
                Some(format!("{prefix}missing.py")),
                Some(format!("{prefix}side/main.py")),
            ] {
                let implicit_root_owner =
                    primary_entry.is_none() && scope == "primary-root" && output_prefix == ".";
                let mut detection = make_detection("other", None);
                detection.metadata.source_build_context = Some(SourceBuildContext {
                    schema_version: 1,
                    build_toolchain: BuildToolchainDeclaration {
                        family: BuildToolchainFamily::Python,
                        python_version: Some(PythonMinor::Python314),
                    },
                    application_runtime: primary_entry.map(|entry| ApplicationRuntimeDeclaration {
                        family: ApplicationRuntimeFamily::Python,
                        python_version: Some(PythonMinor::Python314),
                        entry: Some(entry),
                        args: vec![],
                    }),
                });
                let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
            "version":1,"routes":[{"pattern":"^/.*$","layer":"aaa-primary"}],"layers":[
                {"name":"aaa-primary","target":"COMPUTE","directory":if scope == "sibling-root" { "api" } else { "." },"entry":if scope == "sibling-root" { "main.py" } else { "api/main.py" },"runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.14"}},
                {"name":"zzz-sibling","target":"COMPUTE","directory":if scope == "primary-root" { "side" } else { "." },"entry":if scope == "primary-root" { "main.py" } else { "side/main.py" },"runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.12"}}
            ]
        })).unwrap();
                let result = resolve_runtime_artifact(
                    project.path(),
                    project.path(),
                    output_dir.clone(),
                    manifest,
                    &detection,
                    true,
                );
                if implicit_root_owner {
                    result.expect("null-primary original unique root owner remains authoritative");
                } else {
                    let error = result.expect_err(
                        "dependency owner must be identifiable and match the installer ABI",
                    );
                    expect_code(&error, "APPLICATION_RUNTIME_INVALID");
                }
            }
        }
    }
}
