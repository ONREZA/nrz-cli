use super::{
    edge_build_command, edge_build_execution_context,
    spawn_edge_build_handoff_mock_with_retry_count,
};
use nrz_contract::source_bundle_v2::{DigestAlgorithm, FileEntry, FileRole, LayerTarget};
use nrz_source_bundle::{
    EDGE_BUILD_HANDOFF_V2_FILE, EDGE_BUILD_SOURCE_MANIFEST_V2_FILE,
    EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY, EdgeBuildBindingV2, EdgeBuildHandoffV2,
    source_bundle_v2::{SourceBundleV2ManifestBudget, decode_source_bundle_v2_manifest},
};
use std::{fs, path::Path};

const DEPLOYMENT_ID: &str = "01991c1d-08ad-75f0-8f9a-e5925fb3c2a7";
const COMMAND_ID: &str = "77777777-7777-4777-8777-777777777777";
const EXECUTION_ID: &str = "11111111-1111-4111-8111-111111111111";
const COMMAND_DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn ordinary_deploy_uses_the_installed_v2_profile_without_environment_overrides() {
    let (api_url, requests) = spawn_edge_build_handoff_mock_with_retry_count(DEPLOYMENT_ID, 0);
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("index.html"),
        "<h1>Default Source V2</h1>",
    )
    .unwrap();
    let context = edge_build_execution_context();
    let result = super::nrz()
        .current_dir(project.path())
        .env("NRZ_API_URL", api_url)
        .env("NRZ_LOG_UPLOAD", "0")
        .env_remove("NRZ_SOURCE_TRANSFER")
        .env_remove("NRZ_SOURCE_PACKAGE_PROFILE")
        .env_remove("NRZ_EDGE_BUILD_HANDOFF")
        .env_remove("NRZ_RUNNER")
        .args([
            "--json",
            "--token",
            "fixture-token",
            "deploy",
            ".",
            "--project-id",
            context["projectId"].as_str().unwrap(),
            "--service-id",
            context["serviceId"].as_str().unwrap(),
            "--environment",
            "preview",
            "--skip-build",
            "--skip-install",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(super::stdout_json(&result)["status"], "live");
    let requests = requests.lock().unwrap();
    assert!(requests.contains(&format!(
        "POST /v1/deployments/{DEPLOYMENT_ID}/source-v2/metadata"
    )));
    assert!(requests.contains(&format!(
        "POST /v1/deployments/{DEPLOYMENT_ID}/static-source-v2/admit"
    )));
    assert!(
        !requests
            .iter()
            .any(|request| request.ends_with("/source") || request.ends_with("/prepare-upload"))
    );
}

#[test]
fn source_configuration_errors_never_reach_admission() {
    for (name, value, code) in [
        ("NRZ_SOURCE_TRANSFER", "V1", "CLI_UPDATE_REQUIRED"),
        (
            "NRZ_SOURCE_PACKAGE_PROFILE",
            "uninstalled-profile",
            "INVALID_ARGUMENT",
        ),
        ("NRZ_EDGE_BUILD_HANDOFF", "V1", "CLI_UPDATE_REQUIRED"),
    ] {
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let (api_url, requests) = spawn_edge_build_handoff_mock_with_retry_count(DEPLOYMENT_ID, 0);
        let result = command(project.path(), output.path(), &api_url)
            .env(name, value)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert_eq!(super::stdout_json(&result)["code"], code);
        assert!(requests.lock().unwrap().is_empty());
    }
}

#[test]
fn generated_log_session_dto_accepts_disabled_by_workspace_and_rejects_disabled() {
    use nrz_api::{
        BuildLogSession200Response, BuildLogSession200ResponseSessionShippingPolicy,
        BuildLogSession200ResponseSessionStatus, BuildLogSessionRequestBodySource,
    };
    let project = "44444444-4444-4444-8444-444444444444";
    let wire = serde_json::json!({
        "created": true,
        "session": {
            "id": EXECUTION_ID,
            "projectId": project,
            "deploymentId": DEPLOYMENT_ID,
            "attempt": 1,
            "source": "REMOTE_BUILDER",
            "status": "OPEN",
            "shippingPolicy": "DISABLED_BY_WORKSPACE",
            "nextSeq": 0,
            "acceptedBytes": 0,
            "terminalMessage": null,
            "terminalErrorCode": null,
            "terminalErrorDetails": null,
            "failurePhase": null,
            "startedAt": "2026-10-18T00:00:00Z",
            "finishedAt": null,
            "diagnosticPath": "selected-source-v2-fixture"
        }
    });
    // Exercise the same generated response type decoded by the actual API client.
    let response: BuildLogSession200Response = serde_json::from_value(wire.clone()).unwrap();
    assert!(response.created);
    let session = &response.session;
    assert_eq!(session.id.to_string(), EXECUTION_ID);
    assert_eq!(session.project_id.to_string(), project);
    assert_eq!(session.deployment_id.unwrap().to_string(), DEPLOYMENT_ID);
    assert_eq!(session.attempt, Some(1));
    assert_eq!(
        session.source,
        BuildLogSessionRequestBodySource::RemoteBuilder
    );
    assert_eq!(
        session.status,
        BuildLogSession200ResponseSessionStatus::Open
    );
    assert_eq!(
        session.shipping_policy,
        BuildLogSession200ResponseSessionShippingPolicy::DisabledByWorkspace
    );
    assert_eq!(serde_json::to_value(&response).unwrap(), wire);
    let mut invalid = wire;
    invalid["session"]["shippingPolicy"] = "DISABLED".into();
    assert!(serde_json::from_value::<BuildLogSession200Response>(invalid).is_err());
}

fn command(project: &Path, output: &Path, api_url: &str) -> assert_cmd::Command {
    let mut command = edge_build_command(project, output, api_url, DEPLOYMENT_ID, true);
    command
        .env("NRZ_EDGE_BUILD_HANDOFF", "V2")
        .env("NRZ_EDGE_BUILD_COMMAND_ID", COMMAND_ID)
        .env("NRZ_EDGE_BUILD_COMMAND_DIGEST", COMMAND_DIGEST)
        .env("NRZ_EDGE_BUILD_BUILD_ATTEMPT_ID", EXECUTION_ID)
        .env("NRZ_EDGE_BUILD_ATTEMPT", "1")
        .env("NRZ_EDGE_BUILD_SOURCE_WRITER_GENERATION", "9")
        .env(
            "NRZ_EDGE_BUILD_SOURCE_ENVIRONMENT_ID",
            edge_build_execution_context()["productEnvironmentId"]
                .as_str()
                .unwrap(),
        );
    command
}

fn expected_requests(metadata: bool) -> Vec<String> {
    let mut requests = vec![
        format!("GET /v1/deployments/{DEPLOYMENT_ID}/runner-context"),
        format!("POST /v1/deployments/{DEPLOYMENT_ID}/execution-context/materialize"),
    ];
    if metadata {
        requests.push(format!(
            "POST /v1/deployments/{DEPLOYMENT_ID}/source-v2/metadata"
        ));
    }
    requests
}

#[test]
fn actual_cli_exports_initial_bound_v2_manifest_and_complete_source_bytes() {
    assert_actual_cli_exports_bound_v2_manifest_and_complete_source_bytes(0);
}

#[test]
fn actual_cli_exports_retried_bound_v2_manifest_and_complete_source_bytes() {
    assert_actual_cli_exports_bound_v2_manifest_and_complete_source_bytes(1);
}

fn assert_actual_cli_exports_bound_v2_manifest_and_complete_source_bytes(retry_count: u32) {
    let (api_url, requests) =
        spawn_edge_build_handoff_mock_with_retry_count(DEPLOYMENT_ID, retry_count);
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let expected: [(&str, &[u8]); 3] = [
        ("index.html", b"<h1>Actual Source V2 CLI</h1>"),
        ("assets/data.bin", &[0, 1, 127, 128, 255]),
        ("empty.txt", b""),
    ];
    fs::create_dir(project.path().join("assets")).unwrap();
    for (path, bytes) in expected {
        fs::write(project.path().join(path), bytes).unwrap();
    }
    let result = command(project.path(), output.path(), &api_url)
        .env("NRZ_EDGE_BUILD_ATTEMPT", (retry_count + 1).to_string())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let handoff: EdgeBuildHandoffV2 = serde_json::from_slice(&result.stdout).unwrap();
    handoff.validate().unwrap();
    assert_eq!(
        handoff.binding,
        EdgeBuildBindingV2 {
            deployment_id: DEPLOYMENT_ID.into(),
            command_id: COMMAND_ID.into(),
            command_digest: COMMAND_DIGEST.into(),
            build_attempt_id: EXECUTION_ID.into(),
            attempt: u64::from(retry_count) + 1,
            source_writer_generation: 9,
            source_environment_id: edge_build_execution_context()["productEnvironmentId"]
                .as_str()
                .unwrap()
                .into(),
        }
    );
    let persisted: EdgeBuildHandoffV2 =
        serde_json::from_slice(&fs::read(output.path().join(EDGE_BUILD_HANDOFF_V2_FILE)).unwrap())
            .unwrap();
    assert_eq!(persisted, handoff);
    let bytes = fs::read(output.path().join(EDGE_BUILD_SOURCE_MANIFEST_V2_FILE)).unwrap();
    assert_eq!(bytes.len() as u64, handoff.source_manifest.size_bytes);
    assert_eq!(
        blake3::hash(&bytes).to_hex().as_str(),
        handoff.source_manifest.blake3
    );
    let logical_bytes = expected.iter().map(|(_, bytes)| bytes.len() as u64).sum();
    let verified = decode_source_bundle_v2_manifest(
        &bytes,
        SourceBundleV2ManifestBudget {
            max_file_count: expected.len() as u64,
            max_logical_bytes: logical_bytes,
            max_static_file_bytes: logical_bytes,
        },
    )
    .unwrap();
    assert_eq!(verified.summary(), handoff.source_manifest.summary);
    assert_eq!(verified.summary().file_count, expected.len() as u64);
    assert_eq!(verified.summary().logical_static_bytes, logical_bytes);
    assert_eq!(verified.summary().artifact_size_bytes, 0);
    assert!(
        verified
            .wire()
            .layers
            .iter()
            .all(|layer| layer.target == LayerTarget::Static)
    );
    assert_eq!(verified.wire().files.len(), expected.len());
    for entry in &verified.wire().files {
        let FileEntry::File {
            path,
            role,
            blob_digest,
            size,
            executable,
            ..
        } = entry
        else {
            panic!("regular source file expected: {entry:?}");
        };
        let (_, expected_bytes) = expected
            .iter()
            .find(|(expected_path, _)| *expected_path == path.0)
            .unwrap();
        let frozen = fs::read(
            output
                .path()
                .join(EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY)
                .join(&path.0),
        )
        .unwrap();
        assert_eq!(frozen, *expected_bytes, "{}", path.0);
        assert_eq!(*size, expected_bytes.len() as u64);
        assert_eq!(role, &FileRole::Static);
        assert!(!executable);
        assert_eq!(blob_digest.algorithm, DigestAlgorithm::Blake3);
        assert_eq!(
            blob_digest.value,
            blake3::hash(expected_bytes).to_hex().as_str()
        );
    }
    let mut output_names: Vec<_> = fs::read_dir(output.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    output_names.sort();
    assert_eq!(
        output_names,
        [
            EDGE_BUILD_HANDOFF_V2_FILE,
            EDGE_BUILD_SOURCE_MANIFEST_V2_FILE,
            EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY,
        ]
        .map(std::ffi::OsString::from)
    );
    assert_eq!(requests.lock().unwrap().as_slice(), expected_requests(true));
}

#[test]
fn actual_cli_rejects_v2_environment_mismatch_before_handoff() {
    let (api_url, requests) = spawn_edge_build_handoff_mock_with_retry_count(DEPLOYMENT_ID, 0);
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::write(project.path().join("index.html"), b"retained source").unwrap();
    let result = command(project.path(), output.path(), &api_url)
        .env(
            "NRZ_EDGE_BUILD_SOURCE_ENVIRONMENT_ID",
            "99999999-9999-4999-8999-999999999999",
        )
        .output()
        .unwrap();
    assert!(!result.status.success());
    let error = super::stdout_json(&result);
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("source environment disagrees with authenticated execution context"),
        "{error}"
    );
    assert_eq!(fs::read_dir(output.path()).unwrap().count(), 0);
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        expected_requests(false)
    );
}

#[test]
fn actual_cli_rejects_missing_v2_binding_before_api_requests() {
    for missing in [
        "NRZ_EDGE_BUILD_COMMAND_ID",
        "NRZ_EDGE_BUILD_COMMAND_DIGEST",
        "NRZ_EDGE_BUILD_BUILD_ATTEMPT_ID",
        "NRZ_EDGE_BUILD_ATTEMPT",
        "NRZ_EDGE_BUILD_SOURCE_WRITER_GENERATION",
        "NRZ_EDGE_BUILD_SOURCE_ENVIRONMENT_ID",
    ] {
        let (api_url, requests) = spawn_edge_build_handoff_mock_with_retry_count(DEPLOYMENT_ID, 0);
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        fs::write(project.path().join("index.html"), b"retained source").unwrap();
        let result = command(project.path(), output.path(), &api_url)
            .env_remove(missing)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{missing}");
        let error = super::stdout_json(&result);
        assert!(
            error["error"]
                .as_str()
                .unwrap()
                .contains(&format!("V2 handoff requires {missing}")),
            "{missing}: {error}"
        );
        assert_eq!(fs::read_dir(output.path()).unwrap().count(), 0);
        assert!(requests.lock().unwrap().is_empty());
    }
}

#[test]
fn actual_cli_rejects_stale_v2_attempt_before_metadata_and_handoff() {
    let (api_url, requests) = spawn_edge_build_handoff_mock_with_retry_count(DEPLOYMENT_ID, 1);
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::write(project.path().join("index.html"), b"retained source").unwrap();
    let result = command(project.path(), output.path(), &api_url)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let error = super::stdout_json(&result);
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("binding disagrees with authenticated deployment attempt"),
        "{error}"
    );
    assert_eq!(fs::read_dir(output.path()).unwrap().count(), 0);
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        expected_requests(false)
    );
}
