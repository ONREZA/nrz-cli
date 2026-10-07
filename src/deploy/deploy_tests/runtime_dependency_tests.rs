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
