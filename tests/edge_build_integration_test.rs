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
}

impl EdgeBuildApiState {
    fn record(&self, method: Method, uri: &OriginalUri) {
        self.requests
            .lock()
            .expect("request log")
            .push(format!("{method} {}", uri.0.path()));
    }
}

async fn edge_build_runner_context(
    State(state): State<EdgeBuildApiState>,
    uri: OriginalUri,
) -> Json<serde_json::Value> {
    state.record(Method::GET, &uri);
    Json(json!({
        "protocolVersion": "runner-context-v6",
        "context": {
            "workspaceId": "00000000-0000-0000-0000-000000000001",
            "workspaceSlug": "edge",
            "projectId": "00000000-0000-0000-0000-000000000002",
            "projectName": "Edge project",
            "environmentId": "00000000-0000-0000-0000-000000000003",
            "environmentName": "Preview",
            "environmentType": "PREVIEW",
            "sourceRef": "0123456789abcdef0123456789abcdef01234567",
            "selectionSource": "DEPLOYMENT"
        },
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
            "sourceBuildContext": {"schemaVersion":1,"buildToolchain":{"family":"NODE"},"applicationRuntime": state.application_runtime},
            "frameworkPreset": null,
            "rootDirectory": ".",
            "gitLfsEnabled": false,
            "packageManager": "NPM",
            "installCommand": if state.build_command.is_some() {Some("true")} else if state.application_runtime.is_null() {None} else {Some("touch installer-started")},
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
        "context": {
            "workspaceId": "00000000-0000-0000-0000-000000000001",
            "workspaceSlug": "edge",
            "projectId": "00000000-0000-0000-0000-000000000002",
            "projectName": "Edge project",
            "environmentId": "00000000-0000-0000-0000-000000000003",
            "environmentName": "Preview",
            "environmentType": "PREVIEW",
            "sourceRef": "0123456789abcdef0123456789abcdef01234567",
            "selectionSource": "DEPLOYMENT"
        },
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
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = EdgeBuildApiState {
        deployment_id: deployment_id.to_string(),
        requests: Arc::clone(&requests),
        application_runtime,
        build_command,
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
    let output = nrz()
        .current_dir(project.path())
        .env("NRZ_API_URL", api_url)
        .env("NRZ_RUNNER", "PLATFORM")
        .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
        .env("ONREZA_BUILD_RUNTIME_VERSION", "node-24")
        .env_remove("ONREZA_BUILD_NODE_MAJOR")
        .env("ONREZA_RUNTIME_OS", "linux")
        .env("ONREZA_RUNTIME_ARCH", "x86_64")
        .env("ONREZA_RUNTIME_LIBC", "glibc")
        .env("ONREZA_RUNTIME_VERSION", "bun-1.4.2")
        .env("NRZ_EDGE_BUILD_HANDOFF", "V1")
        .env("NRZ_LOG_UPLOAD", "0")
        .env("ONREZA_OUTPUT_DIR", output_dir.path())
        .args([
            "--json",
            "--token",
            "runner-token",
            "deploy",
            project.path().to_str().unwrap(),
            "--resume-deployment",
            deployment_id,
        ])
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

    let output = nrz()
        .current_dir(project.path())
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
        .env("ONREZA_OUTPUT_DIR", output_dir.path())
        .args([
            "--token",
            "runner-token",
            "deploy",
            project.path().to_str().unwrap(),
            "--resume-deployment",
            deployment_id,
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let handoff: nrz_source_bundle::EdgeBuildHandoffV1 =
        serde_json::from_value(stdout_json(&output)).unwrap();
    handoff.validate().unwrap();
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
        let output = nrz()
            .current_dir(project.path())
            .env("NRZ_API_URL", api_url)
            .env("NRZ_RUNNER", "PLATFORM")
            .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
            .env("ONREZA_BUILD_RUNTIME_VERSION", "node-24")
            .env_remove("ONREZA_BUILD_NODE_MAJOR")
            .env("ONREZA_RUNTIME_FAMILY", "javascript")
            .env("ONREZA_RUNTIME_VERSION", "node-24")
            .env("ONREZA_RUNTIME_OS", "linux")
            .env("ONREZA_RUNTIME_ARCH", "x86_64")
            .env("ONREZA_RUNTIME_LIBC", "glibc")
            .env("NRZ_EDGE_BUILD_HANDOFF", "V1")
            .env("NRZ_LOG_UPLOAD", "0")
            .env("ONREZA_OUTPUT_DIR", output_dir.path())
            .args([
                "--json",
                "--token",
                "runner-token",
                "deploy",
                project.path().to_str().unwrap(),
                "--resume-deployment",
                deployment_id,
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{target} stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let handoff: nrz_source_bundle::EdgeBuildHandoffV1 =
            serde_json::from_value(stdout_json(&output)).unwrap();
        handoff.validate().unwrap();
        let decoder = zstd::Decoder::new(
            fs::File::open(output_dir.path().join(&handoff.source_bundle.path)).unwrap(),
        )
        .unwrap();
        let mut archive = tar::Archive::new(decoder);
        let manifest: nrz_source_bundle::SourceLogicalManifest = archive
            .entries()
            .unwrap()
            .map(Result::unwrap)
            .find(|entry| {
                entry.path().unwrap().as_ref()
                    == std::path::Path::new(nrz_source_bundle::SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH)
            })
            .map(|entry| serde_json::from_reader(entry).unwrap())
            .unwrap();
        assert_eq!(manifest.layers.len(), 1);
        let layer = &manifest.layers[0];
        assert_eq!(layer.target, "COMPUTE");
        assert_eq!(layer.entrypoint.as_deref(), Some(entry));
        let runtime = layer.runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], target);
        declaration.as_object_mut().unwrap().remove("entry");
        declaration.as_object_mut().unwrap().remove("pythonVersion");
        assert_eq!(runtime["applicationRuntime"], declaration);
        assert_eq!(
            nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some(target))
                .unwrap()
                .profile
                .to_string(),
            minor.map_or("EXECUTABLE", |minor| minor.profile_name())
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
            "bun-1.4.2"
        };
        let output = nrz()
            .current_dir(project.path())
            .env("NRZ_API_URL", api_url)
            .env("NRZ_RUNNER", "PLATFORM")
            .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
            .env("ONREZA_BUILD_RUNTIME_VERSION", "node-24")
            .env_remove("ONREZA_BUILD_NODE_MAJOR")
            .env("ONREZA_RUNTIME_OS", "linux")
            .env("ONREZA_RUNTIME_ARCH", "x86_64")
            .env("ONREZA_RUNTIME_LIBC", "glibc")
            .env("ONREZA_RUNTIME_VERSION", target)
            .env("NRZ_EDGE_BUILD_HANDOFF", "V1")
            .env("NRZ_LOG_UPLOAD", "0")
            .env("ONREZA_OUTPUT_DIR", output_dir.path())
            .args([
                "--json",
                "--token",
                "runner-token",
                "deploy",
                project.path().to_str().unwrap(),
                "--resume-deployment",
                deployment_id,
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let handoff: nrz_source_bundle::EdgeBuildHandoffV1 =
            serde_json::from_value(stdout_json(&output)).unwrap();
        handoff.validate().unwrap();
        let decoder = zstd::Decoder::new(
            fs::File::open(output_dir.path().join(&handoff.source_bundle.path)).unwrap(),
        )
        .unwrap();
        let mut archive = tar::Archive::new(decoder);
        let manifest: nrz_source_bundle::SourceLogicalManifest = archive
            .entries()
            .unwrap()
            .map(Result::unwrap)
            .find(|entry| {
                entry.path().unwrap().as_ref()
                    == std::path::Path::new(nrz_source_bundle::SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH)
            })
            .map(|entry| serde_json::from_reader(entry).unwrap())
            .unwrap();
        assert_eq!(manifest.layers.len(), 1);
        let layer = &manifest.layers[0];
        assert_eq!(layer.target, "COMPUTE");
        assert_eq!(layer.entrypoint.as_deref(), Some("server.js"));
        let runtime = layer.runtime_config.as_ref().unwrap();
        assert_eq!(runtime["buildRuntimeVersion"], target);
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
        let output = nrz()
            .current_dir(project.path())
            .env("NRZ_API_URL", api_url)
            .env("NRZ_RUNNER", "PLATFORM")
            .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
            .env("ONREZA_BUILD_RUNTIME_VERSION", "node-24")
            .env_remove("ONREZA_BUILD_NODE_MAJOR")
            .env("ONREZA_RUNTIME_OS", "linux")
            .env("ONREZA_RUNTIME_ARCH", "x86_64")
            .env("ONREZA_RUNTIME_LIBC", "glibc")
            .env("ONREZA_RUNTIME_VERSION", "node-24")
            .env("NRZ_EDGE_BUILD_HANDOFF", "V1")
            .env("NRZ_LOG_UPLOAD", "0")
            .env("ONREZA_OUTPUT_DIR", output_dir.path())
            .args([
                "--json",
                "--token",
                "runner-token",
                "deploy",
                project.path().to_str().unwrap(),
                "--resume-deployment",
                deployment_id,
            ])
            .output()
            .unwrap();
        if explicit_static {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            let handoff: nrz_source_bundle::EdgeBuildHandoffV1 =
                serde_json::from_value(stdout_json(&output)).unwrap();
            handoff.validate().unwrap();
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
