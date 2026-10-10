use crate::rollback::activate_release;
use crate::test_support::serve_api;
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use clap::Parser;
use serde_json::{Value, json};

const ENVIRONMENT: &str = "00000000-0000-4000-8000-000000000001";
const RELEASE: &str = "00000000-0000-4000-8000-000000000002";
const OPERATION: &str = "00000000-0000-4000-8000-000000000003";

fn serving(environment: &str) -> Value {
    json!({
        "environmentId": environment,
        "address": {"url":"https://stable.example.test","subdomain":"stable","enabled":true},
        "desired": {
            "generation":"7","operationId":null,"releaseId":null,
            "phase":"IDLE","result":null,"activeEverywhere":false
        },
        "observed": {"known":false,"releaseId":null,"deploymentId":null,"members":[]}
    })
}

#[tokio::test]
async fn rollback_activates_exact_retained_release_with_generation_and_idempotency() {
    for finish_observation_early in [false, true] {
        let mut arguments = vec!["rollback", "--release-id", RELEASE];
        if finish_observation_early {
            arguments.push("--finish-observation-early");
        }
        let args = crate::cli::RollbackArgs::try_parse_from(arguments).unwrap();
        assert_eq!(args.finish_observation_early, finish_observation_early);
        let app = Router::new()
        .route(
            "/v1/environments/{id}/serving",
            get(|| async { Json(serving(ENVIRONMENT)) }),
        )
        .route(
            "/v1/environments/{id}/actions/activate-release",
            post(move |headers: HeaderMap, Json(body): Json<Value>| async move {
                assert_eq!(
                    body,
                    json!({"releaseId": RELEASE, "expectedGeneration": "7", "finishObservationEarly": finish_observation_early})
                );
                let key = headers.get("idempotency-key").unwrap().to_str().unwrap();
                let key_id = uuid::Uuid::parse_str(key).unwrap();
                assert_eq!(key_id.get_version_num(), 7);
                assert_eq!(key_id.to_string(), key);
                (
                    StatusCode::ACCEPTED,
                    Json(json!({
                        "operationId": OPERATION, "releaseId": RELEASE,
                        "desiredGeneration": "8", "status": "PENDING"
                    })),
                )
            }),
        );
        let (client, server) = serve_api(app).await;
        let response =
            activate_release(&client, ENVIRONMENT, RELEASE, args.finish_observation_early)
                .await
                .unwrap();
        assert_eq!(response.operation_id.to_string(), OPERATION);
        assert_eq!(response.release_id.to_string(), RELEASE);
        assert_eq!(response.desired_generation, "8");
        server.abort();
    }
}

#[tokio::test]
async fn rollback_lists_releases_with_sha256_and_blake3_content_digests() {
    for algorithm in ["SHA256", "BLAKE3"] {
        let digest = json!({"algorithm": algorithm, "value": "a".repeat(64)});
        let response_digest = digest.clone();
        let app = Router::new().route(
            "/v1/environments/{id}/releases",
            get(move || {
                let digest = response_digest.clone();
                async move {
                    Json(json!({
                        "releases": [{
                            "releaseId": RELEASE,
                            "deploymentId": OPERATION,
                            "createdAt": "2026-09-12T00:00:00Z",
                            "contentDigest": digest,
                            "sizeBytes": 123
                        }],
                        "nextCursor": null
                    }))
                }
            }),
        );
        let (client, server) = serve_api(app).await;

        let page = client.environment_releases(ENVIRONMENT).await.unwrap();
        let output = serde_json::to_value(&page).unwrap();

        assert_eq!(page.releases.len(), 1);
        assert_eq!(output["releases"][0]["releaseId"], RELEASE);
        assert_eq!(output["releases"][0]["contentDigest"], digest);
        assert_eq!(output["releases"][0]["sizeBytes"], 123);
        server.abort();
    }
}

#[tokio::test]
async fn rollback_rejects_serving_state_from_another_environment_before_mutation() {
    let app = Router::new()
        .route(
            "/v1/environments/{id}/serving",
            get(|| async { Json(serving("00000000-0000-4000-8000-000000000099")) }),
        )
        .route(
            "/v1/environments/{id}/actions/activate-release",
            post(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );
    let (client, server) = serve_api(app).await;
    assert!(
        activate_release(&client, ENVIRONMENT, RELEASE, false)
            .await
            .unwrap_err()
            .to_string()
            .contains("another environment")
    );
    server.abort();
}
