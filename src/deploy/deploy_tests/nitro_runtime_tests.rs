use super::*;

fn nitro_metadata() -> serde_json::Value {
    serde_json::json!({
        "preset": "node-server",
        "versions": {"nitro": "3.0.260603-beta"},
        "serverEntry": "server/index.mjs"
    })
}

fn tanstack_manifest(entry: &str) -> build_manifest::Manifest {
    serde_json::from_value(serde_json::json!({
        "version":2,"routes":[{"pattern":"^/.*$","layer":"server"}],
        "layers":[{"name":"server","target":"COMPUTE","directory":".","entry":entry,
            "runtime":{"network":crate::network_fixture::network(),"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24"}}]
    })).unwrap()
}

fn tanstack_project(project: &Path) -> PathBuf {
    fs::write(
        project.join("package.json"),
        r#"{"dependencies":{"@tanstack/react-start":"1.0.0","runtime-pkg":"1.0.0"}}"#,
    )
    .unwrap();
    let output = project.join(".output");
    fs::create_dir_all(output.join("server/_libs")).unwrap();
    output
}

#[tokio::test]
async fn tanstack_nitro_output_runs_after_packaging_without_project_dependencies() {
    for (external, source_install) in [(false, false), (false, true), (true, false)] {
        let project = tempdir().unwrap();
        let output = tanstack_project(project.path());
        fs::write(output.join("nitro.json"), nitro_metadata().to_string()).unwrap();
        let import = if external {
            fs::create_dir_all(output.join("server/node_modules/runtime-pkg")).unwrap();
            fs::write(
                output.join("server/node_modules/runtime-pkg/package.json"),
                r#"{"type":"module","main":"index.js"}"#,
            )
            .unwrap();
            fs::write(
                output.join("server/node_modules/runtime-pkg/index.js"),
                "export default 'NITRO_STANDALONE_OK'",
            )
            .unwrap();
            "runtime-pkg"
        } else {
            fs::write(
                output.join("server/_libs/runtime.mjs"),
                "export default 'NITRO_STANDALONE_OK'",
            )
            .unwrap();
            "./_libs/runtime.mjs"
        };
        fs::write(
            output.join("server/index.mjs"),
            format!("import value from '{import}'; console.log(value)"),
        )
        .unwrap();
        if source_install {
            fs::create_dir_all(project.path().join("node_modules/build-only")).unwrap();
            fs::write(
                project.path().join("node_modules/build-only/index.js"),
                "throw new Error('build only')",
            )
            .unwrap();
        }
        let detection = make_detection("tanstack-start", None);
        let artifact = resolve_runtime_artifact(
            project.path(),
            project.path(),
            output.clone(),
            tanstack_manifest("server/index.mjs"),
            &detection,
            true,
        )
        .unwrap();
        assert_eq!(artifact.root_dir, output);
        assert_eq!(
            artifact.manifest.layers[0].entry.as_deref(),
            Some("server/index.mjs")
        );
        let source = runtime_source_bundle_with_checked_entrypoints(&artifact);
        assert!(
            source
                .logical_manifest
                .files
                .iter()
                .all(|file| !file.path.contains("build-only"))
        );
        let logical = crate::test_support::validated_source_bundle_manifest(&source);
        crate::test_support::verify_source_bundle(&source, &logical).await;
        let restored = crate::test_support::unpack_source_bundle(&source);
        fs::remove_dir_all(project.path()).unwrap();
        let result = assert_cmd::Command::new("node")
            .arg(&source.logical_manifest.entrypoints[0])
            .current_dir(restored.path())
            .env_remove("NODE_PATH")
            .env_remove("NODE_OPTIONS")
            .timeout(std::time::Duration::from_secs(10))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&result.stdout).trim(),
            "NITRO_STANDALONE_OK"
        );
    }
}

#[test]
fn tanstack_unrecognized_output_still_requires_project_dependencies() {
    let mut cloudflare = nitro_metadata();
    cloudflare["preset"] = "cloudflare-module".into();
    let mut wrong_entry = nitro_metadata();
    wrong_entry["serverEntry"] = "../server/index.mjs".into();
    let mut missing_version = nitro_metadata();
    missing_version["versions"] = serde_json::json!({});
    for (metadata, entry) in [
        (None, "server/index.mjs"),
        (Some("invalid json".into()), "server/index.mjs"),
        (Some("{}".into()), "server/index.mjs"),
        (Some(cloudflare.to_string()), "server/index.mjs"),
        (Some(wrong_entry.to_string()), "server/index.mjs"),
        (Some(missing_version.to_string()), "server/index.mjs"),
        (Some(nitro_metadata().to_string()), "server/custom.mjs"),
    ] {
        let project = tempdir().unwrap();
        let output = tanstack_project(project.path());
        fs::write(output.join(entry), "import 'runtime-pkg'").unwrap();
        if let Some(metadata) = metadata {
            fs::write(output.join("nitro.json"), metadata).unwrap();
        }
        let error = resolve_runtime_artifact(
            project.path(),
            project.path(),
            output,
            tanstack_manifest(entry),
            &make_detection("tanstack-start", None),
            true,
        )
        .unwrap_err();
        expect_code(&error, "MISSING_RUNTIME_DEPENDENCIES");
        let message = format!("{error:#}");
        assert_eq!(message.matches(project.path().to_str().unwrap()).count(), 1);
        assert!(!message.contains("--skip-install"));
    }
}

#[test]
fn tanstack_nitro_marker_preserves_sibling_project_dependencies() {
    let project = tempdir().unwrap();
    let output = tanstack_project(project.path());
    fs::write(output.join("nitro.json"), nitro_metadata().to_string()).unwrap();
    fs::write(output.join("server/index.mjs"), "console.log('NITRO')").unwrap();
    fs::create_dir_all(output.join("worker")).unwrap();
    fs::write(output.join("worker/index.mjs"), "import 'runtime-pkg'").unwrap();
    fs::create_dir_all(project.path().join("node_modules/runtime-pkg")).unwrap();
    fs::write(
        project.path().join("node_modules/runtime-pkg/index.js"),
        "module.exports = 'SIBLING_DEPENDENCY'",
    )
    .unwrap();
    let mut manifest = tanstack_manifest("server/index.mjs");
    let mut worker = manifest.layers[0].clone();
    worker.name = "worker".into();
    worker.directory = "worker".into();
    worker.entry = Some("index.mjs".into());
    worker
        .runtime
        .as_mut()
        .unwrap()
        .network
        .as_mut()
        .unwrap()
        .exposure = "PRIVATE".into();
    manifest.layers.push(worker);
    let mut detection = make_detection("tanstack-start", None);
    detection.metadata.source_build_context = Some(
        serde_json::from_value(serde_json::json!({
            "schemaVersion":1,"buildToolchain":{"family":"NODE"},
            "applicationRuntime":{"family":"NODE","entry":"worker/index.mjs","args":[]}
        }))
        .unwrap(),
    );
    let artifact = resolve_runtime_artifact_with_manifest_source(
        project.path(),
        project.path(),
        output,
        manifest,
        &detection,
        true,
        RuntimeArtifactManifestContext {
            source: crate::artifact::BuildManifestSource::File,
            configured_output_entry: true,
        },
    )
    .unwrap();
    assert_eq!(artifact.root_dir, project.path());
    let source = runtime_source_bundle_with_checked_entrypoints(&artifact);
    let dependency = source
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/runtime-pkg/index.js")
        .expect("the sibling still needs its project dependency");
    assert_dependency_owner(dependency, "worker");
    assert!(
        source
            .logical_manifest
            .entrypoints
            .iter()
            .any(|entry| entry == ".output/worker/index.mjs")
    );
}
