use super::{nrz, stdout_json, support};
use axum::{
    Json, Router,
    extract::Query,
    routing::{get, post},
};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::mpsc::channel;

const PROJECT: &str = "00000000-0000-4000-8000-000000000001";
const ENVIRONMENT: &str = "00000000-0000-4000-8000-000000000002";
const SERVICE: &str = "00000000-0000-4000-8000-000000000003";
const RELEASE: &str = "00000000-0000-4000-8000-000000000004";
const COMPOSITION: &str = "00000000-0000-4000-8000-000000000005";
const DEPLOYMENT: &str = "00000000-0000-4000-8000-000000000006";
const BEFORE_RELEASE: &str = "00000000-0000-4000-8000-000000000007";
const BEFORE_COMPOSITION: &str = "00000000-0000-4000-8000-000000000008";
const NEXT_RELEASE: &str = "00000000-0000-4000-8000-000000000009";
const NEXT_COMPOSITION: &str = "00000000-0000-4000-8000-00000000000a";

#[test]
fn list_preserves_independent_release_and_composition_pagination() {
    let page = json!({
        "releases": [{"releaseId": RELEASE, "deploymentId": DEPLOYMENT,
            "createdAt": "2026-10-09T00:00:00Z", "contentDigest": {"algorithm": "BLAKE3", "value": "a".repeat(64)}, "sizeBytes": 123}],
        "compositions": [{"compositionId": COMPOSITION, "createdAt": "2026-10-08T00:00:00Z",
            "components": [{"serviceId": SERVICE, "deploymentId": DEPLOYMENT, "runtimeReleaseId": RELEASE,
                "contentDigest": {"algorithm": "BLAKE3", "value": "b".repeat(64)}}]}],
        "nextCursor": {"createdAt": "2026-10-07T00:00:00Z", "releaseId": NEXT_RELEASE},
        "nextCompositionCursor": {"createdAt": "2026-10-06T00:00:00Z", "compositionId": NEXT_COMPOSITION}
    });
    for mode in ["--json", "--human"] {
        let (sender, queries) = channel();
        let response = page.clone();
        let app = Router::new()
            .route(&format!("/v1/projects/{PROJECT}/execution-context/resolve"), post(|| async {
                Json(json!({"protocolVersion": "execution-context-v3", "context": {
                    "workspaceId": PROJECT, "workspaceSlug": "workspace", "productId": PROJECT,
                    "productEnvironmentId": ENVIRONMENT, "workspaceEnvironmentId": ENVIRONMENT,
                    "workspaceBindingGeneration": "1", "serviceId": SERVICE, "sourceId": PROJECT,
                    "projectId": PROJECT, "projectName": "project", "environmentId": ENVIRONMENT,
                    "environmentName": "Production", "environmentType": "PRODUCTION", "sourceRef": null,
                    "selectionSource": "EXPLICIT"
                }}))
            }))
            .route(&format!("/v1/environments/{ENVIRONMENT}/serving"), get(|| async {
                Json(json!({"environmentId": ENVIRONMENT, "address": null,
                    "desired": {"generation": "7", "operationId": null, "releaseId": null,
                        "phase": "IDLE", "result": null, "activeEverywhere": false},
                    "observed": {"known": true, "releaseId": RELEASE, "deploymentId": DEPLOYMENT, "members": []}
                }))
            }))
            .route(&format!("/v1/environments/{ENVIRONMENT}/releases"), get(move |Query(query): Query<BTreeMap<String, String>>| {
                let response = response.clone();
                let sender = sender.clone();
                async move {
                    sender.send(query).unwrap();
                    Json(response)
                }
            }));
        let api = support::api_mock::spawn(app);
        let directory = tempfile::tempdir().unwrap();
        let output = nrz()
            .current_dir(directory.path())
            .env("NRZ_API_URL", api)
            .args([
                mode,
                "--token",
                "test-token",
                "rollback",
                "--list",
                "--project-id",
                PROJECT,
                "--environment",
                "Production",
                "--service-id",
                SERVICE,
                "--before-created-at",
                "2026-10-11T00:00:00Z",
                "--before-release-id",
                BEFORE_RELEASE,
                "--before-composition-created-at",
                "2026-10-10T00:00:00Z",
                "--before-composition-id",
                BEFORE_COMPOSITION,
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let query = queries
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("rollback did not request a release page");
        assert_eq!(
            query,
            BTreeMap::from([
                ("limit".into(), "100".into()),
                ("beforeCreatedAt".into(), "2026-10-11T00:00:00Z".into()),
                ("beforeReleaseId".into(), BEFORE_RELEASE.into()),
                (
                    "beforeCompositionCreatedAt".into(),
                    "2026-10-10T00:00:00Z".into()
                ),
                ("beforeCompositionId".into(), BEFORE_COMPOSITION.into()),
            ])
        );
        if mode == "--json" {
            assert_eq!(
                stdout_json(&output),
                json!({
                    "environmentId": ENVIRONMENT, "observedReleaseId": RELEASE, "desiredGeneration": "7",
                    "releases": page["releases"], "compositions": page["compositions"],
                    "nextCursor": page["nextCursor"], "nextCompositionCursor": page["nextCompositionCursor"]
                })
            );
        } else {
            assert!(output.stdout.is_empty(), "{output:?}");
            let text = String::from_utf8_lossy(&output.stderr);
            for expected in [
                format!("{RELEASE}  2026-10-09T00:00:00+00:00 (observed active)"),
                format!("Composition {COMPOSITION}"),
                format!("Service {SERVICE} → deployment {DEPLOYMENT} / runtime {RELEASE}"),
                format!(
                    "--before-created-at 2026-10-07T00:00:00+00:00 --before-release-id {NEXT_RELEASE}"
                ),
                format!(
                    "--before-composition-created-at 2026-10-06T00:00:00+00:00 --before-composition-id {NEXT_COMPOSITION}"
                ),
            ] {
                assert!(text.contains(&expected), "missing {expected}: {text}");
            }
        }
    }
}
