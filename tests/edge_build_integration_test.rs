//! Edge runner handoff contract against the real CLI binary.
mod support;
use axum::{
    Json, Router,
    extract::{OriginalUri, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::json;
use std::{
    fs,
    sync::{Arc, Mutex},
};
use support::cli::{nrz, stdout_json};

#[derive(Clone)]
struct EdgeBuildApiState {
    deployment_id: String,
    requests: Arc<Mutex<Vec<String>>>,
    application_runtime: serde_json::Value,
    build_command: Option<String>,
    install_command: Option<String>,
    build_toolchain: serde_json::Value,
}

impl EdgeBuildApiState {
    fn record(&self, method: Method, uri: &OriginalUri) {
        self.requests
            .lock()
            .expect("request log")
            .push(format!("{method} {}", uri.0.path()));
    }
}

fn edge_build_execution_context() -> serde_json::Value {
    json!({
        "workspaceId": "00000000-0000-0000-0000-000000000001",
        "workspaceSlug": "edge",
        "productId": "00000000-0000-0000-0000-000000000002",
        "productEnvironmentId": "00000000-0000-0000-0000-000000000003",
        "workspaceEnvironmentId": "00000000-0000-0000-0000-000000000004",
        "workspaceBindingGeneration": "1",
        "serviceId": "00000000-0000-0000-0000-000000000005",
        "sourceId": "00000000-0000-0000-0000-000000000006",
        "projectId": "00000000-0000-0000-0000-000000000002",
        "projectName": "Edge project",
        "environmentId": "00000000-0000-0000-0000-000000000003",
        "environmentName": "Preview",
        "environmentType": "PREVIEW",
        "sourceRef": "0123456789abcdef0123456789abcdef01234567",
        "selectionSource": "DEPLOYMENT"
    })
}

async fn edge_build_runner_context(
    State(state): State<EdgeBuildApiState>,
    uri: OriginalUri,
) -> Json<serde_json::Value> {
    state.record(Method::GET, &uri);
    Json(json!({
        "protocolVersion": "runner-context-v6",
        "context": edge_build_execution_context(),
        "deployment": {
            "id": state.deployment_id,
            "attempt": 1,
            "status": "BUILDING",
            "branch": "main",
            "commitSha": "0123456789abcdef0123456789abcdef01234567",
            "url": null
        },
        "settings": {
            "nodeVersion": "NODE_24",
            "sourceBuildContext": {"schemaVersion":1,"buildToolchain":state.build_toolchain,"applicationRuntime": state.application_runtime},
            "frameworkPreset": null,
            "rootDirectory": ".",
            "gitLfsEnabled": false,
            "packageManager": "NPM",
            "installCommand": state.install_command,
            "installCommandSource": if state.build_command.is_some() || !state.application_runtime.is_null() {"USER"} else {"PRESET"},
            "buildCommand": state.build_command,
            "buildCommandSource": if state.build_command.is_some() {"USER"} else {"PRESET"},
            "outputDirectory": state.build_command.as_ref().map(|_| "."),
            "outputDirectorySource": if state.build_command.is_some() {"USER"} else {"PRESET"},
            "ignoredBuildBehavior": "AUTOMATIC",
            "ignoredBuildFolder": null,
            "ignoredBuildCommand": null
        }
    }))
}

async fn edge_build_materialize(
    State(state): State<EdgeBuildApiState>,
    uri: OriginalUri,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    state.record(Method::POST, &uri);
    assert_eq!(body, json!({ "purpose": "DEPLOY" }));
    Json(json!({
        "protocolVersion": "execution-context-v2",
        "context": edge_build_execution_context(),
        "variables": {},
        "secretKeys": [],
        "snapshot": {
            "fingerprint": format!("v1:{}", "0".repeat(64)),
            "resolvedAt": "2026-08-29T00:00:00Z",
            "source": "DEPLOYMENT",
            "deploymentId": state.deployment_id
        }
    }))
}

async fn reject_unexpected_edge_build_request(
    State(state): State<EdgeBuildApiState>,
    method: Method,
    uri: OriginalUri,
) -> impl IntoResponse {
    state.record(method, &uri);
    (
        StatusCode::IM_A_TEAPOT,
        Json(json!({ "error": "unexpected Edge build request" })),
    )
}

fn spawn_edge_build_handoff_mock(deployment_id: &str) -> (String, Arc<Mutex<Vec<String>>>) {
    spawn_edge_build_handoff_mock_with_intent(deployment_id, serde_json::Value::Null)
}

fn spawn_edge_build_handoff_mock_with_intent(
    deployment_id: &str,
    application_runtime: serde_json::Value,
) -> (String, Arc<Mutex<Vec<String>>>) {
    spawn_edge_build_handoff_mock_with_build(deployment_id, application_runtime, None)
}

fn spawn_edge_build_handoff_mock_with_build(
    deployment_id: &str,
    application_runtime: serde_json::Value,
    build_command: Option<String>,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let install_command = if build_command
        .as_ref()
        .is_some_and(|command| command.is_empty())
    {
        Some(String::new())
    } else if build_command.is_some() {
        Some("true".into())
    } else if application_runtime.is_null() {
        None
    } else {
        Some("touch installer-started".into())
    };
    spawn_edge_build_handoff_mock_with_commands(
        deployment_id,
        application_runtime,
        json!({"family":"NODE"}),
        install_command,
        build_command,
    )
}

fn spawn_edge_build_handoff_mock_with_commands(
    deployment_id: &str,
    application_runtime: serde_json::Value,
    build_toolchain: serde_json::Value,
    install_command: Option<String>,
    build_command: Option<String>,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = EdgeBuildApiState {
        deployment_id: deployment_id.to_string(),
        requests: Arc::clone(&requests),
        application_runtime,
        build_command,
        install_command,
        build_toolchain,
    };
    let app = Router::new()
        .route(
            "/v1/deployments/{deployment_id}/runner-context",
            get(edge_build_runner_context),
        )
        .route(
            "/v1/deployments/{deployment_id}/execution-context/materialize",
            post(edge_build_materialize),
        )
        .fallback(reject_unexpected_edge_build_request)
        .with_state(state);
    (support::api_mock::spawn(app), requests)
}

fn edge_build_command(
    project: &std::path::Path,
    output: &std::path::Path,
    api_url: &str,
    deployment_id: &str,
    json_mode: bool,
) -> assert_cmd::Command {
    let mut command = nrz();
    if json_mode {
        command.arg("--json");
    }
    command
        .current_dir(project)
        .env("NRZ_API_URL", api_url)
        .env("NRZ_RUNNER", "PLATFORM")
        .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
        .env("ONREZA_BUILD_RUNTIME_VERSION", "node-24")
        .env_remove("ONREZA_BUILD_NODE_MAJOR")
        .env("ONREZA_RUNTIME_OS", "linux")
        .env("ONREZA_RUNTIME_ARCH", "x86_64")
        .env("ONREZA_RUNTIME_LIBC", "glibc")
        .env("NRZ_EDGE_BUILD_HANDOFF", "V1")
        .env("NRZ_LOG_UPLOAD", "0")
        .env("ONREZA_OUTPUT_DIR", output)
        .args([
            "--token",
            "runner-token",
            "deploy",
            project.to_str().unwrap(),
            "--resume-deployment",
            deployment_id,
        ]);
    command
}

fn successful_handoff(
    output: &std::process::Output,
    scenario: &str,
) -> nrz_source_bundle::EdgeBuildHandoffV1 {
    assert!(
        output.status.success(),
        "{scenario} stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let handoff: nrz_source_bundle::EdgeBuildHandoffV1 =
        serde_json::from_value(stdout_json(output)).unwrap();
    handoff.validate().unwrap();
    handoff
}

fn source_bundle_manifest(
    output: &std::path::Path,
    cli_output: &std::process::Output,
    scenario: &str,
) -> nrz_source_bundle::SourceLogicalManifest {
    let handoff = successful_handoff(cli_output, scenario);
    let decoder =
        zstd::Decoder::new(fs::File::open(output.join(&handoff.source_bundle.path)).unwrap())
            .unwrap();
    let mut archive = tar::Archive::new(decoder);
    archive
        .entries()
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| {
            entry.path().unwrap().as_ref()
                == std::path::Path::new(nrz_source_bundle::SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH)
        })
        .map(|entry| serde_json::from_reader(entry).unwrap())
        .unwrap()
}

#[test]
fn changed_source_runtime_is_rejected_before_the_installer_executes() {
    let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
    let (api_url, _) = spawn_edge_build_handoff_mock_with_intent(
        deployment_id,
        json!({"family":"NODE","entry":"server.ts","args":[]}),
    );
    let project = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("onreza.toml"),
        "[deploy]\nruntime='bun'\nentry='server.ts'\nargs=[]\n",
    )
    .unwrap();
    fs::write(
        project.path().join("server.ts"),
        "Bun.serve({fetch:()=>new Response('ok')})",
    )
    .unwrap();
    let output = edge_build_command(
        project.path(),
        output_dir.path(),
        &api_url,
        deployment_id,
        true,
    )
    .env("ONREZA_RUNTIME_VERSION", "bun-1.4.2")
    .output()
    .unwrap();
    assert!(!output.status.success());
    let result = stdout_json(&output);
    assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("frozen deployment snapshot")
    );
    assert!(!project.path().join("installer-started").exists());
    assert!(!output_dir.path().join("source-bundle-v1.tar.zst").exists());
}

#[test]
fn edge_build_publishes_local_handoff_without_legacy_source_mutations() {
    let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
    let (api_url, requests) = spawn_edge_build_handoff_mock(deployment_id);
    let project = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    fs::write(project.path().join("index.html"), "<h1>Edge build</h1>").unwrap();

    let output = edge_build_command(
        project.path(),
        output_dir.path(),
        &api_url,
        deployment_id,
        false,
    )
    .output()
    .unwrap();

    successful_handoff(&output, "static handoff");
    assert_eq!(
        fs::read(output_dir.path().join("edge-build-handoff-v1.json")).unwrap(),
        output.stdout
    );
    assert!(output_dir.path().join("source-bundle-v1.tar.zst").is_file());
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        [
            format!("GET /v1/deployments/{deployment_id}/runner-context"),
            format!("POST /v1/deployments/{deployment_id}/execution-context/materialize"),
        ]
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[test]
fn edge_build_handoff_preserves_python_and_native_serving_under_node_compiler() {
    let targets = nrz_source_bundle::PythonMinor::ALL
        .into_iter()
        .map(|minor| (Some(minor), minor.target()))
        .chain([(
            Some(nrz_source_bundle::PythonMinor::Python312),
            "python-3.14",
        )])
        .chain([(None, nrz_runtime_artifact::NATIVE_EXECUTION_TARGET)]);
    for (minor, target) in targets {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (family, entry, mut declaration) = if let Some(minor) = minor {
            (
                "PYTHON",
                "main.py",
                json!({"family":"PYTHON", "pythonVersion":minor.version(), "entry":"main.py", "args":["literal argument"]}),
            )
        } else {
            (
                "EXECUTABLE",
                "server",
                json!({"family":"EXECUTABLE", "entry":"server", "args":["literal argument"]}),
            )
        };
        let (api_url, requests) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            declaration.clone(),
            Some("true".into()),
        );
        let project = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let selector = minor.map_or_else(String::new, |minor| {
            format!("python_version='{}'\n", minor.version())
        });
        fs::write(project.path().join("onreza.toml"), format!(
            "[build]\ntoolchain='node'\n[deploy]\nruntime='{}'\n{selector}entry='{entry}'\nargs=['literal argument']\n",
            family.to_ascii_lowercase(),
        )).unwrap();
        if minor.is_some() {
            fs::write(
                project.path().join(entry),
                "print('code-only serving fixture')\n",
            )
            .unwrap();
        } else {
            // Package a real host ELF; this test does not claim runtime execution.
            fs::copy("/usr/bin/true", project.path().join(entry)).unwrap();
        }
        let manifest = admitted_source_manifest(
            project.path(),
            output_dir.path(),
            &api_url,
            deployment_id,
            target,
        );
        assert_eq!(manifest.layers.len(), 1);
        let layer = &manifest.layers[0];
        assert_eq!(layer.target, "COMPUTE");
        assert_eq!(layer.entrypoint.as_deref(), Some(entry));
        let runtime = layer.runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], target);
        assert_eq!(
            runtime.get("runtimePolicyLineage"),
            minor
                .map(|minor| json!(format!("PYTHON:{}", minor.version())))
                .as_ref()
        );
        declaration.as_object_mut().unwrap().remove("entry");
        declaration.as_object_mut().unwrap().remove("pythonVersion");
        assert_eq!(runtime["applicationRuntime"], declaration);
        assert_eq!(
            nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some(target))
                .unwrap()
                .profile
                .to_string(),
            nrz_source_bundle::PythonMinor::from_target(target)
                .map_or("EXECUTABLE", |minor| minor.profile_name())
        );
        assert!(
            nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some("node-24"))
                .is_err()
        );
        assert_eq!(
            requests.lock().unwrap().as_slice(),
            [
                format!("GET /v1/deployments/{deployment_id}/runner-context"),
                format!("POST /v1/deployments/{deployment_id}/execution-context/materialize")
            ]
        );
    }
}

#[cfg(unix)]
#[test]
fn build_generated_package_infers_process_without_changing_frozen_runtime() {
    for (frozen_runtime, keep_start, root_html, configured_entry) in [
        (serde_json::Value::Null, true, false, true),
        (serde_json::Value::Null, true, false, false),
        (
            json!({"family":"BUN","entry":"server.js","args":["literal argument"]}),
            true,
            false,
            true,
        ),
        (
            json!({"family":"BUN","entry":"server.js","args":["literal argument"]}),
            false,
            false,
            true,
        ),
        (
            json!({"family":"BUN","entry":"server.js","args":["literal argument"]}),
            false,
            true,
            true,
        ),
    ] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (api_url, _) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            frozen_runtime.clone(),
            Some("mv package_temp.json package.json && npm install --ignore-scripts --no-audit --no-fund".into()),
        );
        let project = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        let config = if !configured_entry {
            ""
        } else if frozen_runtime.is_null() {
            "[deploy]\nentry='server.js'\n"
        } else {
            "[deploy]\nruntime='bun'\nentry='server.js'\nargs=['literal argument']\n"
        };
        fs::write(project.path().join("onreza.toml"), config).unwrap();
        let package = json!({"name":"generated-process", "version":"1.0.0", "private":true,
            "scripts": if keep_start {json!({"start":"node server.js"})} else {json!({})}});
        fs::write(
            project.path().join("package_temp.json"),
            package.to_string(),
        )
        .unwrap();
        if root_html {
            fs::write(
                project.path().join("index.html"),
                "<h1>server-owned content</h1>",
            )
            .unwrap();
        }
        fs::write(project.path().join("server.js"),
            "require('node:http').createServer((_,r)=>r.end('ok')).listen(process.env.PORT || 3000)").unwrap();
        if !frozen_runtime.is_null() {
            fs::write(
                project.path().join("package.json"),
                r#"{"scripts":{"start":"bun server.js"}}"#,
            )
            .unwrap();
        }
        let target = if frozen_runtime.is_null() {
            "node-24"
        } else {
            "bun-1.4.3"
        };
        let manifest = admitted_source_manifest(
            project.path(),
            output_dir.path(),
            &api_url,
            deployment_id,
            target,
        );
        assert_eq!(manifest.layers.len(), 1);
        let layer = &manifest.layers[0];
        assert_eq!(layer.target, "COMPUTE");
        assert_eq!(layer.entrypoint.as_deref(), Some("server.js"));
        let runtime = layer.runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], target);
        assert_eq!(
            runtime.get("runtimePolicyLineage"),
            (!frozen_runtime.is_null())
                .then(|| json!("BUN:CURRENT"))
                .as_ref()
        );
        let expected_intent = if frozen_runtime.is_null() {
            serde_json::Value::Null
        } else {
            json!({"family":"BUN","args":["literal argument"]})
        };
        assert_eq!(runtime["applicationRuntime"], expected_intent);
        let launch =
            nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some(target))
                .unwrap();
        assert_eq!(
            launch.profile.to_string(),
            if frozen_runtime.is_null() {
                "NODE_24"
            } else {
                "BUN"
            }
        );
        assert!(
            manifest
                .files
                .iter()
                .any(|file| file.path == "server.js" && file.role == "compute")
        );
        assert!(manifest.files.iter().all(|file| file.role != "static"));
        assert!(!project.path().join("package_temp.json").exists());
        assert!(project.path().join("package-lock.json").exists());
    }
}

#[cfg(unix)]
#[test]
fn build_generated_elysia_rejects_node_and_keeps_explicit_static_before_source_handoff() {
    for (frozen_runtime, explicit_static) in [
        (
            json!({"family":"NODE","entry":"server.js","args":[]}),
            false,
        ),
        (serde_json::Value::Null, false),
        (serde_json::Value::Null, true),
    ] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (api_url, _) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            frozen_runtime.clone(),
            Some("mv package_temp.json package.json && touch build-completed".into()),
        );
        let project = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("onreza.toml"),
            if explicit_static {
                "[deploy]\ncompute='static'\n"
            } else if frozen_runtime.is_null() {
                "[deploy]\nentry='server.js'\n"
            } else {
                "[deploy]\nruntime='node'\nentry='server.js'\nargs=[]\n"
            },
        )
        .unwrap();
        fs::write(
            project.path().join("package.json"),
            if frozen_runtime.is_null() {
                r#"{}"#
            } else {
                r#"{"scripts":{"start":"node server.js"}}"#
            },
        )
        .unwrap();
        fs::write(
            project.path().join("package_temp.json"),
            r#"{"dependencies":{"elysia":"^1.4.0"},"scripts":{"start":"node server.js"}}"#,
        )
        .unwrap();
        fs::write(project.path().join("server.js"), "console.log('server')").unwrap();
        let output = edge_build_command(
            project.path(),
            output_dir.path(),
            &api_url,
            deployment_id,
            true,
        )
        .env("ONREZA_RUNTIME_VERSION", "node-24")
        .output()
        .unwrap();
        if explicit_static {
            successful_handoff(&output, "explicit static elysia");
            assert!(output_dir.path().join("source-bundle-v1.tar.zst").exists());
            continue;
        }
        assert!(!output.status.success());
        let result = stdout_json(&output);
        assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
        assert!(
            result["error"]
                .as_str()
                .unwrap()
                .contains("conflicts with framework elysia"),
            "{result}"
        );
        assert!(project.path().join("build-completed").exists());
        assert!(!project.path().join("package_temp.json").exists());
        assert!(!output_dir.path().join("source-bundle-v1.tar.zst").exists());
        assert!(
            !output_dir
                .path()
                .join("edge-build-handoff-v1.json")
                .exists()
        );
    }
}

#[test]
fn retained_platform_handoff_preserves_authored_targets_without_new_build_facts() {
    for (family, old_target, admitted_target, minor, declared_target, typed) in [
        ("BUN", "bun-1.4.2", "bun-1.4.3", None, true, true),
        ("NODE", "node-24", "node-26", None, true, true),
        ("NODE", "node-24", "node-26", None, false, true),
        ("NODE", "node-24", "node-26", None, true, false),
        ("NODE", "node-24", "node-26", None, false, false),
        (
            "PYTHON",
            "python-3.12",
            "python-3.14",
            Some("3.12"),
            true,
            true,
        ),
        (
            "PYTHON",
            "python-3.12",
            "python-3.14",
            Some("3.12"),
            false,
            true,
        ),
    ] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let mut declaration = json!({"family":family,"entry":"server.js","args":[]});
        if let Some(minor) = minor {
            declaration["pythonVersion"] = json!(minor);
        }
        if !typed {
            declaration = serde_json::Value::Null;
        }
        let (api_url, _) = spawn_edge_build_handoff_mock_with_build(
            deployment_id,
            declaration,
            Some(String::new()),
        );
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let minor_config = minor
            .map(|minor| format!("python_version='{minor}'\n"))
            .unwrap_or_default();
        let config = if typed {
            format!(
                "[build]\ntoolchain='node'\n[deploy]\nruntime='{}'\n{minor_config}entry='server.js'\n",
                family.to_ascii_lowercase()
            )
        } else {
            "[build]\ntoolchain='node'\n".into()
        };
        fs::write(project.path().join("onreza.toml"), config).unwrap();
        fs::write(project.path().join("server.js"), "// retained source bytes").unwrap();
        if family != "PYTHON" {
            fs::write(
                project.path().join("package.json"),
                r#"{"name":"retained","dependencies":{"retained-dep":"1.0.0"}}"#,
            )
            .unwrap();
            fs::create_dir_all(project.path().join("node_modules/retained-dep")).unwrap();
            fs::write(
                project
                    .path()
                    .join("node_modules/retained-dep/package.json"),
                r#"{"name":"retained-dep","version":"1.0.0","main":"index.js"}"#,
            )
            .unwrap();
            fs::write(
                project.path().join("node_modules/retained-dep/index.js"),
                "module.exports = 'retained dependency'",
            )
            .unwrap();
        }
        fs::create_dir(project.path().join(".onreza")).unwrap();
        let mut authored_manifest = json!({
            "version":1,"routes":[{"pattern":"^/.*$","layer":"primary"}],"layers":[
                {"name":"primary","target":"COMPUTE","directory":".","entry":"server.js",
                 "runtime":{"applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":old_target}}
            ]
        });
        if !typed {
            authored_manifest["layers"][0]["runtime"]
                .as_object_mut()
                .unwrap()
                .remove("applicationRuntime");
        }
        if !declared_target {
            authored_manifest["layers"][0]["runtime"]
                .as_object_mut()
                .unwrap()
                .remove("buildRuntimeVersion");
        }
        fs::write(
            project.path().join(".onreza/manifest.json"),
            serde_json::to_vec(&authored_manifest).unwrap(),
        )
        .unwrap();
        let result =
            edge_build_command(project.path(), output.path(), &api_url, deployment_id, true)
                .env("ONREZA_RUNTIME_VERSION", admitted_target)
                .output()
                .unwrap();
        if !typed && !declared_target {
            assert!(!result.status.success());
            let result = stdout_json(&result);
            assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
            assert!(
                result["error"].as_str().unwrap().contains(
                    "retained COMPUTE output requires an application runtime and frozen target"
                ),
                "{result}"
            );
            assert!(!output.path().join("source-bundle-v1.tar.zst").exists());
            continue;
        }
        let manifest = source_bundle_manifest(output.path(), &result, old_target);
        let runtime = manifest.layers[0].runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], old_target);
        assert!(runtime.get("runtimePolicyLineage").is_none());
        if family != "PYTHON" {
            let dependency = manifest
                .files
                .iter()
                .find(|file| file.path == "node_modules/retained-dep/index.js")
                .expect("retained dependency bytes must be packaged");
            assert_eq!(dependency.role, "compute");
        }
        assert!(!project.path().join("installer-started").exists());
        assert!(!project.path().join(".onreza/python/launch.py").exists());
        assert_eq!(
            fs::read_to_string(project.path().join("server.js")).unwrap(),
            "// retained source bytes"
        );
    }
}

#[test]
fn fresh_python_dependencies_use_resolved_minors_without_rejecting_requested_lines() {
    for (fresh_install, fresh_build) in [(true, false), (false, true), (false, false)] {
        let deployment_id = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
        let (api_url, _) = spawn_edge_build_handoff_mock_with_commands(
            deployment_id,
            json!({"family":"PYTHON","pythonVersion":"3.13","entry":"main.py","args":[]}),
            json!({"family":"PYTHON","pythonVersion":"3.12"}),
            Some(if fresh_install { "true" } else { "" }.into()),
            Some(if fresh_build { "true" } else { "" }.into()),
        );
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        fs::write(project.path().join("onreza.toml"), "[build]\ntoolchain='python'\npython_version='3.12'\n[deploy]\nruntime='python'\npython_version='3.13'\nentry='main.py'\n").unwrap();
        fs::write(project.path().join("main.py"), "print('retained code')").unwrap();
        fs::write(
            project.path().join("requirements.txt"),
            "retained-dep==1.0\n",
        )
        .unwrap();
        let dependency_root = project.path().join(".onreza/python/3.14/site-packages");
        fs::create_dir_all(&dependency_root).unwrap();
        fs::write(dependency_root.join("retained_dep.py"), "VALUE = 1").unwrap();
        let old_dependency_root = project.path().join(".onreza/python/3.13/site-packages");
        fs::create_dir_all(&old_dependency_root).unwrap();
        fs::write(old_dependency_root.join("old_dep.py"), "OLD_ABI_BYTES = 1").unwrap();
        let result =
            edge_build_command(project.path(), output.path(), &api_url, deployment_id, true)
                .env("ONREZA_BUILD_RUNTIME_FAMILY", "python")
                .env("ONREZA_BUILD_RUNTIME_VERSION", "python-3.14")
                .env("ONREZA_RUNTIME_VERSION", "python-3.14")
                .output()
                .unwrap();
        if !fresh_install && !fresh_build {
            assert!(!result.status.success());
            let result = stdout_json(&result);
            assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
            assert!(
                result["error"]
                    .as_str()
                    .unwrap()
                    .contains("matching build and serving Python minors"),
                "{result}"
            );
            assert!(!output.path().join("source-bundle-v1.tar.zst").exists());
            continue;
        }
        if fresh_build && !result.status.success() {
            // A development host can lack the qualified Builder interpreter.
            // The actual compiler guard must remain authoritative after the
            // obsolete source minors stop rejecting an admitted fresh build.
            let result = stdout_json(&result);
            let error = result["error"].as_str().unwrap();
            assert!(
                error.contains("selected Python compiler is unavailable")
                    || error.contains("Python build compiler does not match selected CPython"),
                "{result}"
            );
            assert!(!output.path().join("source-bundle-v1.tar.zst").exists());
            continue;
        }
        let manifest = source_bundle_manifest(output.path(), &result, "python-3.14");
        assert!(
            manifest
                .files
                .iter()
                .all(|file| !file.path.ends_with("old_dep.py"))
        );
        let runtime = manifest.layers[0].runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], "python-3.14");
        assert_eq!(runtime["runtimePolicyLineage"], "PYTHON:3.13");
        assert!(
            manifest
                .files
                .iter()
                .any(|file| file.path.ends_with("retained_dep.py") && file.role == "dependency")
        );
    }
}

fn admitted_source_manifest(
    project: &std::path::Path,
    output: &std::path::Path,
    api_url: &str,
    deployment_id: &str,
    target: &str,
) -> nrz_source_bundle::SourceLogicalManifest {
    let result = edge_build_command(project, output, api_url, deployment_id, true)
        .env("ONREZA_RUNTIME_VERSION", target)
        .output()
        .unwrap();
    source_bundle_manifest(output, &result, target)
}
