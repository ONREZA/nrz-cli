use super::{nrz, stdout_json, support};
use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;

#[test]
fn admitted_sets_report_one_json_error_with_recovery_details() {
    for local in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let project = Uuid::now_v7();
        let environment = Uuid::now_v7();
        let service = Uuid::now_v7();
        let deployment = Uuid::now_v7();
        let operation = Uuid::now_v7();
        let key = Uuid::now_v7();
        let checkout = root.path().canonicalize().unwrap();
        let context = json!({"workspaceId": project, "productId": project, "productEnvironmentId": environment,
            "workspaceEnvironmentId": environment, "workspaceBindingGeneration": "1", "serviceId": service, "sourceId": project});
        let request = if local {
            json!({"protocolVersion": "execution-context-v3", "environmentId": environment, "expectedGeneration": "7",
                "idempotencyKey": key, "selection": {"kind": "SERVICES"}, "services": [{"mode": "LOCAL", "serviceId": service,
                "branch": "main", "commitSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "expectedContext": context}]})
        } else {
            json!({"projectId": project, "environmentId": environment, "expectedGeneration": "7", "idempotencyKey": key,
                "selection": {"kind": "SERVICES", "services": [{"serviceId": service}]}, "envVars": {}})
        };
        let receipt = root
            .path()
            .join(format!(".onreza/deployment-sets/{key}.json"));
        std::fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let mappings = if local {
            json!({service.to_string(): checkout})
        } else {
            json!({})
        };
        std::fs::write(&receipt, json!({"projectId": project, "environmentId": environment, "environmentSelector": "production",
            "local": mappings, "all": false, "selectedServices": [service], "request": request}).to_string()).unwrap();
        let observed = Arc::new(AtomicUsize::new(0));
        let calls = observed.clone();
        let admission = json!({"operationId": operation, "generation": "8", "status": "PENDING", "replayed": true,
            "deployments": [{"id": deployment, "serviceId": service, "status": "BUILDING"}]});
        let app = Router::new()
            .route(if local { "/v1/projects/{id}/deployments/admit-set" } else { "/v1/deployments/set" }, post(move || {
                let admission = admission.clone();
                async move { Json(admission) }
            }))
            .route("/v1/deployments/{id}/runner-context", get(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { (StatusCode::FORBIDDEN, Json(json!({"code": "FORBIDDEN", "message": "runner snapshot unavailable", "details": {"permission": "deployments.read"}}))) }
            }))
            .route("/v1/deployments/{id}/status", get(move || async move {
                Json(json!({"id": deployment, "status": "building", "url": null, "production": false, "error": null,
                    "errorCode": null, "errorDetails": null, "runtimeArtifactGraphDigest": null, "runtimeArtifactGraph": null,
                    "createdAt": "2026-10-14T00:00:00Z", "readyAt": null}))
            }));
        let api = support::api_mock::spawn(app);
        let mut command = nrz();
        command
            .current_dir(root.path())
            .env("NRZ_API_URL", api)
            .env_remove("NRZ_RUNNER")
            .env_remove("NRZ_EDGE_BUILD_HANDOFF")
            .env_remove("ONREZA_OUTPUT_DIR")
            .args([
                "--json",
                "--token",
                "test-token",
                "deploy-set",
                "--project-id",
                &project.to_string(),
                "--environment",
                "production",
                "--service-id",
                &service.to_string(),
                "--idempotency-key",
                &key.to_string(),
                "--wait-timeout",
                "1",
            ]);
        if local {
            command.args(["--local", &format!("{service}={}", checkout.display())]);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let terminal = stdout_json(&output);
        assert_eq!(
            terminal["code"],
            if local {
                "FORBIDDEN"
            } else {
                "DEPLOY_WAIT_TIMEOUT"
            }
        );
        assert_eq!(terminal["details"]["operationId"], operation.to_string());
        assert_eq!(terminal["details"]["idempotencyKey"], key.to_string());
        assert_eq!(
            terminal["details"]["receipt"],
            receipt.to_string_lossy().as_ref()
        );
        if local {
            assert_eq!(observed.load(Ordering::SeqCst), 1);
            assert_eq!(terminal["details"]["permission"], "deployments.read");
        } else {
            assert_eq!(terminal["details"]["deploymentId"], deployment.to_string());
        }
        assert!(receipt.exists());
    }
}
