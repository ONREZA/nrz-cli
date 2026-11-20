use crate::test_support::serve_api;
use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get},
};
use serde_json::{Value, json};
use std::collections::HashMap;

use super::{ServiceConfigDeleteBody, ServiceConfigSaveBody, ServiceConfigVariable};

const PRODUCT: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT: &str = "00000000-0000-0000-0000-000000000002";
const SERVICE: &str = "00000000-0000-0000-0000-000000000003";

fn context(environment: Option<&str>) -> Value {
    json!({
        "workspaceId": PRODUCT, "productId": PRODUCT, "serviceId": SERVICE, "sourceId": SERVICE,
        "workspaceBindingGeneration": "1",
        "environment": environment.map(|id| json!({
            "workspaceId": PRODUCT, "productId": PRODUCT, "productEnvironmentId": id,
            "workspaceEnvironmentId": id, "workspaceBindingGeneration": "1"
        }))
    })
}

#[tokio::test]
async fn service_config_operations_keep_the_exact_service_and_environment_target() {
    let app = Router::new()
        .route(
            "/v1/projects/{product}/services/{service}/config",
            get(
                |Path((product, service)): Path<(String, String)>,
                 Query(query): Query<HashMap<String, String>>| async move {
                    assert_eq!(product, PRODUCT);
                    assert_eq!(service, SERVICE);
                    assert_eq!(query.get("environmentId").map(String::as_str), Some(ENVIRONMENT));
                    assert_eq!(query.get("branch").map(String::as_str), Some("feature/config"));
                    Json(json!({"context":context(Some(ENVIRONMENT)), "variables":[], "effective":[]}))
                },
            )
            .put(
                |Path((product, service)): Path<(String, String)>, Json(body): Json<Value>| async move {
                    assert_eq!((product.as_str(), service.as_str()), (PRODUCT, SERVICE));
                    assert_eq!(body, json!({
                        "environmentId": ENVIRONMENT, "replaceScope": true, "changeCategory": true,
                        "confirmed": true, "variables":[{"key":"TOKEN", "value":"test-secret", "isSecret":true}]
                    }));
                    Json(json!({"context":context(Some(ENVIRONMENT)), "message":"Saved"}))
                },
            ),
        )
        .route(
            "/v1/projects/{product}/services/{service}/config/{key}",
            delete(
                |Path((product, service, key)): Path<(String, String, String)>,
                 Query(query): Query<HashMap<String, String>>,
                 Json(body): Json<Value>| async move {
                    assert_eq!((product.as_str(), service.as_str(), key.as_str()), (PRODUCT, SERVICE, "TOKEN"));
                    assert!(query.is_empty());
                    assert_eq!(body, json!({"environmentId":ENVIRONMENT, "confirmed":true}));
                    Json(json!({"context":context(Some(ENVIRONMENT)), "message":"Deleted"}))
                },
            ),
        );
    let (client, server) = serve_api(app).await;
    let product = PRODUCT.parse().unwrap();
    let service = SERVICE.parse().unwrap();
    let environment = ENVIRONMENT.parse().unwrap();
    let listed = client
        .service_config(product, service, Some(environment), Some("feature/config"))
        .await
        .unwrap();
    assert_eq!(listed.context.service_id, service);
    assert_eq!(
        listed.context.environment.unwrap().product_environment_id,
        environment
    );
    let saved = client
        .save_service_config(
            product,
            service,
            ServiceConfigSaveBody {
                environment_id: Some(environment),
                replace_scope: Some(true),
                change_category: Some(true),
                confirmed: Some(true),
                variables: vec![ServiceConfigVariable {
                    key: "TOKEN".into(),
                    value: Some("test-secret".into()),
                    is_secret: true,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(saved.message, "Saved");
    let removed = client
        .delete_service_config(
            product,
            service,
            "TOKEN",
            ServiceConfigDeleteBody {
                environment_id: Some(environment),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(removed.message, "Deleted");
    server.abort();
}

#[tokio::test]
async fn service_defaults_omit_environment_query_and_keep_explicit_null_in_writes() {
    let app = Router::new().route(
        "/v1/projects/{product}/services/{service}/config",
        get(|Query(query):Query<HashMap<String,String>>| async move {
            assert!(query.is_empty());
            Json(json!({"context":context(None),"variables":[],"effective":[]}))
        }).put(|Json(body):Json<Value>| async move {
            assert_eq!(body,json!({"environmentId":null,"variables":[{"key":"PLAIN","value":"value","isSecret":false}]}));
            Json(json!({"context":context(None),"message":"Saved"}))
        }),
    );
    let (client, server) = serve_api(app).await;
    assert!(
        client
            .service_config(
                PRODUCT.parse().unwrap(),
                SERVICE.parse().unwrap(),
                None,
                None
            )
            .await
            .unwrap()
            .context
            .environment
            .is_none()
    );
    client
        .save_service_config(
            PRODUCT.parse().unwrap(),
            SERVICE.parse().unwrap(),
            ServiceConfigSaveBody {
                variables: vec![ServiceConfigVariable {
                    key: "PLAIN".into(),
                    value: Some("value".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    server.abort();
}

#[tokio::test]
async fn malformed_service_config_response_omits_received_secret_values_from_errors() {
    let app = Router::new().route(
        "/v1/projects/{product}/services/{service}/config",
        get(|| async {
            Json(json!({"context":"sensitive-mismatched-value","variables":[],"effective":[]}))
        }),
    );
    let (client, server) = serve_api(app).await;
    let error = client
        .service_config(
            PRODUCT.parse().unwrap(),
            SERVICE.parse().unwrap(),
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(super::classify_api_retry(&error).is_none());
    assert!(error.to_string().contains("OpenAPI contract"));
    assert!(!format!("{error:#}").contains("sensitive-mismatched-value"));
    assert!(!format!("{error:?}").contains("sensitive-mismatched-value"));
    server.abort();
}

#[tokio::test]
async fn service_config_conflicts_preserve_structured_platform_errors() {
    let app=Router::new().route("/v1/projects/{product}/services/{service}/config",get(||async {(StatusCode::CONFLICT,Json(json!({"code":"ENV_CONFIG_SCOPE_REPLACEMENT_REQUIRED","message":"Confirm scope replacement","details":{"key":"TOKEN"}})))}));
    let (client, server) = serve_api(app).await;
    let error = client
        .service_config(
            PRODUCT.parse().unwrap(),
            SERVICE.parse().unwrap(),
            None,
            None,
        )
        .await
        .unwrap_err();
    let error = error.downcast_ref::<super::StructuredApiError>().unwrap();
    assert_eq!(error.status, reqwest::StatusCode::CONFLICT);
    assert_eq!(error.code, "ENV_CONFIG_SCOPE_REPLACEMENT_REQUIRED");
    assert_eq!(error.details, Some(json!({"key":"TOKEN"})));
    server.abort();
}
