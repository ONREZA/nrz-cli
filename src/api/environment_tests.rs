use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::{delete, get, post},
};
use serde_json::{Value, json};
use std::collections::HashMap;

use super::ApiClient;

const PROJECT_ID: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT_ID: &str = "00000000-0000-0000-0000-000000000002";

#[tokio::test]
async fn environment_mutations_follow_typed_scope_and_confirmation_contracts() {
    let app = Router::new()
        .route(
            "/v1/projects/{id}/env",
            post(
                |Path(id): Path<String>, Json(body): Json<Value>| async move {
                    assert_eq!(id, PROJECT_ID);
                    assert_eq!(
                        body,
                        json!({"key":"TOKEN", "value":"test-value", "isSecret":true,
                "scopeType":"SELECTED", "environmentIds":[ENVIRONMENT_ID], "confirmed":true})
                    );
                    Json(
                        json!({"id":ENVIRONMENT_ID, "key":"TOKEN", "scopeType":"SELECTED",
                "isSecret":true, "created":true, "message":"Created"}),
                    )
                },
            ),
        )
        .route(
            "/v1/projects/{id}/env/{key}",
            delete(
                |Path((id, key)): Path<(String, String)>,
                 Query(query): Query<HashMap<String, String>>| async move {
                    assert_eq!(id, PROJECT_ID);
                    assert_eq!(key, "TOKEN");
                    assert_eq!(query, HashMap::from([("confirmed".into(), "true".into())]));
                    Json(json!({"deleted":true, "key":"TOKEN", "message":"Deleted"}))
                },
            ),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let created = client
        .set_environment_variable(
            PROJECT_ID,
            nrz_api::EnvRequestBody {
                key: "TOKEN".into(),
                value: "test-value".into(),
                is_secret: Some(nrz_api::ProjectRequestBody2IncludeFilesOutsideRoot::Boolean(true)),
                scope_type: Some(nrz_api::Project200Response3EnvVarScopeType::Selected),
                environment_ids: Some(vec![ENVIRONMENT_ID.parse().unwrap()]),
                confirmed: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(created.created && created.is_secret);
    assert!(
        client
            .delete_environment_variable(PROJECT_ID, "TOKEN", true)
            .await
            .unwrap()
            .deleted
    );
    server.abort();
}

#[tokio::test]
async fn malformed_env_response_is_an_error_instead_of_an_empty_listing() {
    let app = Router::new().route(
        "/v1/projects/{id}/env",
        get(|| async { Json(json!({"total":"sensitive-mismatched-value", "envVars":[]})) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let error = client.environment_variables(PROJECT_ID).await.unwrap_err();
    assert!(super::classify_api_retry(&error).is_none());
    assert!(error.to_string().contains("OpenAPI contract"));
    assert!(!format!("{error:#}").contains("sensitive-mismatched-value"));
    assert!(!format!("{error:?}").contains("sensitive-mismatched-value"));
    server.abort();
}
