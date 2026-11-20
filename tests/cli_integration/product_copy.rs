use super::{nrz, stdout_json, support};
use axum::{Json, Router, extract::Request, http::Method, routing::any};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

const PRODUCT: &str = "00000000-0000-4000-8000-000000000001";
const ENVIRONMENT: &str = "00000000-0000-4000-8000-000000000002";
const SOURCE: &str = "00000000-0000-4000-8000-000000000003";
const SERVICE: &str = "00000000-0000-4000-8000-000000000004";
const FORK: &str = "00000000-0000-4000-8000-000000000005";
const TARGET: &str = "00000000-0000-4000-8000-000000000006";
const COMPOSITION: &str = "00000000-0000-4000-8000-000000000007";
const PREPARATION: &str = "00000000-0000-4000-8000-000000000008";
const RELEASE: &str = "00000000-0000-4000-8000-000000000009";
const KEY: &str = "019b8952-ca22-76f0-b134-88becec7c62a";

struct RequestRecord {
    method: Method,
    idempotency_key: Option<String>,
    body: Value,
}

fn mock_api(path: &str, response: Value) -> (String, Receiver<RequestRecord>) {
    let (sender, received) = channel();
    let app = Router::new().route(
        path,
        any(move |request: Request| {
            let response = response.clone();
            let sender = sender.clone();
            async move {
                assert_eq!(request.headers()["X-API-Key"], "test-token");
                let method = request.method().clone();
                let idempotency_key = request
                    .headers()
                    .get("Idempotency-Key")
                    .map(|value| value.to_str().unwrap().to_owned());
                let bytes = axum::body::to_bytes(request.into_body(), 64 * 1024)
                    .await
                    .unwrap();
                let body = if bytes.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes).unwrap()
                };
                sender
                    .send(RequestRecord {
                        method,
                        idempotency_key,
                        body,
                    })
                    .unwrap();
                Json(response)
            }
        }),
    );
    (support::api_mock::spawn(app), received)
}

fn invoke(api: &str, directory: &Path, args: &[&str]) -> std::process::Output {
    nrz()
        .current_dir(directory)
        .env("NRZ_API_URL", api)
        .args(["--json", "--token", "test-token"])
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn fork_plan_and_usage_preserve_explicit_parent_and_precise_costs() {
    let directory = tempfile::tempdir().unwrap();
    let spec = json!({"parentEnvironmentId": ENVIRONMENT, "expiresAt": "2026-11-20T00:00:00Z",
        "idempotencyKey": "copy-intent-1", "databases": [], "approvedSecretIds": []});
    let spec_path = directory.path().join("plan.json");
    std::fs::write(&spec_path, spec.to_string()).unwrap();
    let plan = json!({"targetAvailable": true, "parent": {"environmentId": ENVIRONMENT, "productId": PRODUCT,
        "workspaceId": PRODUCT, "sourceReleaseId": RELEASE, "permanent": true, "working": true},
        "services": [], "bindings": [], "resources": [], "secrets": [], "blockers": [], "access": "restricted",
        "expiresAt": "2026-11-20T00:00:00Z", "estimate": {"currency": "RUB", "knownCostCents": "123.456789",
            "coverage": "PARTIAL", "components": [], "assumptions": ["Usage is not final"]}});
    let (api, requests) = mock_api(&format!("/v1/product-forks/{PRODUCT}/plan"), plan.clone());
    let output = invoke(
        &api,
        directory.path(),
        &[
            "fork",
            "--product-id",
            PRODUCT,
            "plan",
            "--spec",
            spec_path.to_str().unwrap(),
        ],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), plan);
    let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(request.method, Method::POST);
    assert_eq!(request.body, spec);

    let usage = json!({"coverage": "PARTIAL", "currency": "RUB", "knownGrossCostCents": "9007199254740993.25",
        "period": null, "freshness": "PROVISIONAL", "lastReconciledAt": null, "limitations": ["Unrated meter"],
        "items": [{"resourceType": "database", "resourceId": FORK, "moduleKey": "postgres", "meterKey": "storage",
            "quantity": "9007199254740993", "unit": "byte", "grossAmountCents": null}]});
    let (api, requests) = mock_api(
        &format!("/v1/product-forks/{PRODUCT}/{FORK}/usage"),
        usage.clone(),
    );
    let output = invoke(
        &api,
        directory.path(),
        &["fork", "--product-id", PRODUCT, "usage", FORK],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), usage);
    assert_eq!(
        requests
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .method,
        Method::GET
    );
}

#[test]
fn fork_show_preserves_each_service_url_without_output_flags() {
    let directory = tempfile::tempdir().unwrap();
    let expires_at = "2026-11-20T00:00:00Z";
    let response = json!({"operation": {"id": FORK, "workspaceId": PRODUCT, "productId": PRODUCT,
        "workspaceBindingGeneration": "1", "sourceEnvironmentId": ENVIRONMENT,
        "targetEnvironmentId": TARGET, "targetWorkspaceEnvironmentId": TARGET,
        "sourceReleaseId": RELEASE, "sourceServices": [], "sourceBindings": [], "state": "READY",
        "expiresAt": expires_at, "retained": false, "lifecycleGeneration": "1", "serviceGeneration": "1",
        "failureStage": null, "cleanupReason": null, "createdAt": "2026-11-19T00:00:00Z", "deletedAt": null},
        "services": [
            {"operationId": FORK, "serviceId": SERVICE, "sourceDeploymentId": RELEASE,
             "gitCommitSha": null, "targetDeploymentId": RELEASE, "targetDeploymentStatus": "ACTIVE",
             "targetCommitSha": null, "url": "https://web-fork.preview.onreza.ru"},
            {"operationId": FORK, "serviceId": SOURCE, "sourceDeploymentId": RELEASE,
             "gitCommitSha": null, "targetDeploymentId": RELEASE, "targetDeploymentStatus": "ACTIVE",
             "targetCommitSha": null, "url": "https://api-fork.preview.onreza.ru"},
            {"operationId": FORK, "serviceId": PREPARATION, "sourceDeploymentId": RELEASE,
             "gitCommitSha": null, "targetDeploymentId": null, "targetDeploymentStatus": null,
             "targetCommitSha": null, "url": null}],
        "resources": [], "access": "restricted", "externalEffects": "External effects remain possible",
        "residualCost": "Metered until cleanup completes"});

    let (api, requests) = mock_api(
        &format!("/v1/product-forks/{PRODUCT}/{FORK}"),
        response.clone(),
    );
    let output = nrz()
        .current_dir(directory.path())
        .env("NRZ_API_URL", api)
        .env_remove("NRZ_JSON")
        .args([
            "--token",
            "test-token",
            "fork",
            "--product-id",
            PRODUCT,
            "show",
            FORK,
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), response);
    let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(request.method, Method::GET);
    assert_eq!(request.body, Value::Null);
}

#[test]
fn fork_show_preserves_cleaned_resource_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let response = json!({"operation": {"id": FORK, "workspaceId": PRODUCT, "productId": PRODUCT,
        "workspaceBindingGeneration": "1", "sourceEnvironmentId": ENVIRONMENT,
        "targetEnvironmentId": TARGET, "targetWorkspaceEnvironmentId": TARGET,
        "sourceReleaseId": RELEASE, "sourceServices": [], "sourceBindings": [], "state": "CLEANING",
        "expiresAt": "2026-11-20T10:00:00Z", "retained": false, "lifecycleGeneration": "3", "serviceGeneration": "2",
        "failureStage": "CLEANUP", "cleanupReason": "MANUAL", "createdAt": "2026-11-20T08:00:00Z", "deletedAt": null},
        "services": [], "resources": [{"id": TARGET, "forkId": FORK, "sourceDatabaseEnvironmentId": SOURCE,
            "databaseId": PREPARATION, "providerDatabaseId": "db-parent", "sourceProviderBranchId": "br-parent",
            "mode": "fork", "targetDatabaseEnvironmentId": TARGET, "providerBranchId": "br-copy",
            "providerEndpointId": "ep-copy", "ancestorLsn": "0/215CD38", "state": "CLEANED",
            "cleanupEvidence": {"providerDeleted": true, "providerBranchId": "br-copy", "residualCost": "Parent data remains billable"}}],
        "access": "restricted", "externalEffects": "External effects remain possible", "residualCost": "Metering may lag"});
    let (api, requests) = mock_api(
        &format!("/v1/product-forks/{PRODUCT}/{FORK}"),
        response.clone(),
    );
    let output = invoke(
        &api,
        directory.path(),
        &["fork", "--product-id", PRODUCT, "show", FORK],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), response);
    assert_eq!(
        requests
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .method,
        Method::GET
    );
}

#[test]
fn fork_policy_uses_exact_source_and_expected_generation_for_changes() {
    let directory = tempfile::tempdir().unwrap();
    let spec = json!({"expectedGeneration": "17", "parentEnvironmentId": ENVIRONMENT,
        "serviceIds": [SERVICE], "lifetimeSeconds": 3600, "databases": [], "approvedSecretIds": [],
        "executePullRequestCode": true});
    let spec_path = directory.path().join("policy.json");
    std::fs::write(&spec_path, spec.to_string()).unwrap();
    let response = json!({"policy": null, "runs": []});
    let (api, requests) = mock_api(
        &format!("/v1/product-fork-policies/{PRODUCT}/{SOURCE}"),
        response.clone(),
    );
    for (arguments, method, body) in [
        (vec!["show"], Method::GET, Value::Null),
        (
            vec!["set", "--spec", spec_path.to_str().unwrap()],
            Method::PUT,
            spec,
        ),
        (
            vec!["disable", "--expected-generation", "17"],
            Method::DELETE,
            json!({"expectedGeneration": "17"}),
        ),
    ] {
        let mut args = vec!["fork", "--product-id", PRODUCT, "policy", SOURCE];
        args.extend(arguments);
        let output = invoke(&api, directory.path(), &args);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(stdout_json(&output), response);
        let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(request.method, method);
        assert_eq!(request.body, body);
    }
}

#[test]
fn publish_previews_selected_versions_and_sends_exact_replacement_confirmation() {
    let directory = tempfile::tempdir().unwrap();
    let selection = json!({"sourceEnvironmentId": ENVIRONMENT, "sourceCompositionId": COMPOSITION, "serviceIds": [SERVICE]});
    let preview = json!({"requiresTargetPreparation": true, "environmentId": TARGET, "generation": "23", "sourceEnvironmentId": ENVIRONMENT,
        "sourceCompositionId": COMPOSITION, "services": [{"serviceId": SERVICE, "sourceRuntimeReleaseId": RELEASE,
            "candidatePreparationId": PREPARATION, "baseRuntimeReleaseId": null, "replacedRuntimeReleaseId": RELEASE,
            "confirmationRequired": true}], "accepted": [], "pending": [], "observed": null});
    let base = [
        "publish",
        "--target-environment",
        TARGET,
        "--source-environment",
        ENVIRONMENT,
        "--source-composition",
        COMPOSITION,
        "--service",
        SERVICE,
    ];
    let (api, requests) = mock_api(
        &format!("/v1/environments/{TARGET}/actions/preview-publication"),
        preview.clone(),
    );
    let mut args = base.to_vec();
    args.push("--preview");
    let output = invoke(&api, directory.path(), &args);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), preview);
    let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(request.method, Method::POST);
    assert_eq!(request.body, selection);
    assert_eq!(request.idempotency_key, None);

    let confirmations = json!([{"serviceId": SERVICE, "candidatePreparationId": PREPARATION, "replacedRuntimeReleaseId": RELEASE}]);
    let confirmation_path = directory.path().join("confirmations.json");
    std::fs::write(&confirmation_path, confirmations.to_string()).unwrap();
    let mut response = preview.clone();
    response["status"] = json!("ACCEPTED");
    response["requiresTargetPreparation"] = json!(false);
    response["activation"] = Value::Null;
    response["requestId"] = json!(KEY);
    response["preparations"] = json!([]);
    let (api, requests) = mock_api(
        &format!("/v1/environments/{TARGET}/actions/publish-services"),
        response.clone(),
    );
    let mut args = base.to_vec();
    args.extend([
        "--idempotency-key",
        KEY,
        "--confirmations",
        confirmation_path.to_str().unwrap(),
    ]);
    let output = invoke(&api, directory.path(), &args);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_json(&output), response);
    let request = requests.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(request.method, Method::POST);
    assert_eq!(request.idempotency_key.as_deref(), Some(KEY));
    let mut expected = selection;
    expected["confirmations"] = confirmations;
    assert_eq!(request.body, expected);

    let output = invoke(&api, directory.path(), &base);
    assert!(
        !output.status.success(),
        "publication without a key must be refused"
    );
    let mut args = base.to_vec();
    args.extend(["--preview", "--idempotency-key", KEY]);
    assert!(
        !invoke(&api, directory.path(), &args).status.success(),
        "preview cannot publish"
    );
    assert!(requests.try_recv().is_err());
}
