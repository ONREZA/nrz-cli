use crate::{api::ApiClient, rollback::find_live_deployment};
use axum::{
    Json, Router,
    extract::Query,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::collections::HashMap;

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";

fn deployment(id: u128, active: bool) -> Value {
    json!({"id":uuid::Uuid::from_u128(id), "status":"LIVE", "isPreview":false,
        "isRollback":false, "isActive":active, "commitSha":"abc", "branch":"main", "url":null,
        "createdAt":"2026-09-12T00:00:00Z", "deployedAt":null, "finishedAt":null})
}

#[tokio::test]
async fn rollback_finds_active_deployment_beyond_first_page_and_posts_json_body() {
    let app = Router::new().route("/v1/deployments/project/{id}", get(|Query(query): Query<HashMap<String,String>>| async move {
        assert_eq!(query["limit"].parse::<f64>().unwrap(), 100.0);
        match query["offset"].parse::<f64>().unwrap() as u32 {
            0 => Json(json!({"deployments":(1..=100).map(|id|deployment(id, false)).collect::<Vec<_>>(), "total":101})),
            100 => Json(json!({"deployments":[deployment(101, true)], "total":101})),
            other => panic!("unexpected offset {other}"),
        }
    })).route("/v1/deployments/{id}/rollback", post(|Json(body):Json<Value>| async move {
        assert_eq!(body, json!({}));
        Json(json!({"id":uuid::Uuid::from_u128(102),"status":"QUEUED", "message":"Queued",
            "rollbackFrom":uuid::Uuid::from_u128(101),"rollbackTo":uuid::Uuid::from_u128(99), "rebuildQueued":true}))
    }));
    let (client, server) = serve(app).await;
    let id = find_live_deployment(&client, PROJECT).await.unwrap();
    assert_eq!(id, uuid::Uuid::from_u128(101).to_string());
    let response = client.rollback_deployment(&id).await.unwrap();
    assert_eq!(response.rollback_from.to_string(), id);
    assert_eq!(response.rebuild_queued, Some(true));
    server.abort();
}

#[tokio::test]
async fn rollback_requires_explicit_selection_for_multiple_active_environments() {
    let app = Router::new().route(
        "/v1/deployments/project/{id}",
        get(|| async {
            Json(json!({"deployments":[deployment(1, true),deployment(2, true)], "total":2}))
        }),
    );
    let (client, server) = serve(app).await;
    assert!(
        find_live_deployment(&client, PROJECT)
            .await
            .unwrap_err()
            .to_string()
            .contains("--deployment-id")
    );
    server.abort();
}

#[tokio::test]
async fn rollback_rejects_empty_incomplete_pagination_instead_of_selecting_partial_results() {
    let app = Router::new().route(
        "/v1/deployments/project/{id}",
        get(|Query(query): Query<HashMap<String, String>>| async move {
            let deployments = if query["offset"].parse::<f64>().unwrap() == 0.0 {
                vec![deployment(1, true)]
            } else {
                vec![]
            };
            Json(json!({"deployments":deployments,"total":2}))
        }),
    );
    let (client, server) = serve(app).await;
    assert!(
        find_live_deployment(&client, PROJECT)
            .await
            .unwrap_err()
            .to_string()
            .contains("listing changed")
    );
    server.abort();
}

async fn serve(app: Router) -> (ApiClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (client, server)
}
