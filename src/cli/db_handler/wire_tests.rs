use super::*;
use axum::{
    Json, Router,
    extract::{Request, State},
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const DB: &str = "11111111-1111-4111-8111-111111111111";
const PROJECT: &str = "22222222-2222-4222-8222-222222222222";

struct Exchange {
    method: &'static str,
    path: String,
    body: Option<Value>,
    response: Value,
}

async fn serve(
    exchanges: Vec<Exchange>,
) -> (
    ApiClient,
    Arc<Mutex<VecDeque<Exchange>>>,
    tokio::task::JoinHandle<()>,
) {
    let pending = Arc::new(Mutex::new(VecDeque::from(exchanges)));
    let app = Router::new()
        .fallback(
            |State(pending): State<Arc<Mutex<VecDeque<Exchange>>>>, request: Request| async move {
                let exchange = pending
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected API request");
                assert_eq!(request.method().as_str(), exchange.method);
                assert_eq!(request.uri().to_string(), exchange.path);
                let body = axum::body::to_bytes(request.into_body(), 1024 * 1024)
                    .await
                    .unwrap();
                match exchange.body {
                    Some(expected) => {
                        assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), expected)
                    }
                    None => assert!(body.is_empty()),
                }
                Json(exchange.response)
            },
        )
        .with_state(Arc::clone(&pending));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (client, pending, server)
}

fn branch_json() -> Value {
    json!({"id":"preview/../branch?scope=other","name":"preview","database_id":"provider-db",
        "created_at":"2026-09-12T00:00:00Z","updated_at":"2026-09-12T00:00:00Z",
        "desired_state":"active","external_generation":1,"fork_kind":"branch","integration_annotations":{},"labels":{},
        "lifecycle_phase":"active","protected":false,"resource_version":1,"status":"active"})
}
fn connection() -> Value {
    json!({"connection_uri":"postgres://user:password@db.example.com/app?sslmode=require", "db_name":"app", "role_name":"user",
        "password":"password","endpoint_id":"endpoint", "default_connection_mode":"direct",
        "connection_targets":{"direct":{"host":"db.example.com","port":5432,"mode":"direct","sslmode":"require",
            "connection_uri":"postgres://user:password@db.example.com/app?sslmode=require"},"pooled":null}})
}

#[tokio::test]
async fn create_database_attaches_exact_resource_and_project_without_sending_defaults() {
    let created = nrz_api::Database200Response2 {
        id: DB.parse().unwrap(),
        db_name: "primary".into(),
        password: "private-password".into(),
        ..Default::default()
    };
    let attachment = nrz_api::Database200ResponseDatumProjectAttachment {
        managed_database_id: DB.parse().unwrap(),
        project_id: PROJECT.parse().unwrap(),
        ..Default::default()
    };
    let (client, pending, server) = serve(vec![
        Exchange {
            method: "POST",
            path: "/v1/kaiki/databases/".into(),
            body: Some(json!({"dbName":"primary","cuSize":0.5})),
            response: serde_json::to_value(created.clone()).unwrap(),
        },
        Exchange {
            method: "PATCH",
            path: format!("/v1/kaiki/databases/{DB}/attachments/{PROJECT}"),
            body: Some(json!({})),
            response: serde_json::to_value(attachment).unwrap(),
        },
    ])
    .await;
    cmd_create(
        &client,
        PROJECT,
        true,
        Some("primary".into()),
        Some(0.5),
        false,
    )
    .await
    .unwrap();
    assert!(pending.lock().unwrap().is_empty());
    let output = serde_json::to_value(CreateResponse::from(created)).unwrap();
    assert_eq!(
        output,
        json!({"id":DB,"dbName":"primary","status":"CREATING"})
    );
    server.abort();
}

#[tokio::test]
async fn branch_requests_encode_identifiers_and_preserve_list_metadata() {
    let mut listed = branch_json();
    listed.as_object_mut().unwrap().extend(json!({"localId":null,"parentKaikiBranchId":null,"ancestorLsn":null,"isPreviewBranch":true}).as_object().unwrap().clone());
    let (client, pending, server) = serve(vec![
        Exchange {
            method: "GET",
            path: format!("/v1/kaiki/databases/{DB}/branches"),
            body: None,
            response: json!([listed]),
        },
        Exchange {
            method: "GET",
            path: format!(
                "/v1/kaiki/databases/{DB}/branches/preview%2F..%2Fbranch%3Fscope=other/connection"
            ),
            body: None,
            response: connection(),
        },
        Exchange {
            method: "POST",
            path: format!("/v1/kaiki/databases/{DB}/branches"),
            body: Some(json!({"name":"preview"})),
            response: branch_json(),
        },
        Exchange {
            method: "DELETE",
            path: format!("/v1/kaiki/databases/{DB}/branches/preview%2F..%2Fbranch%3Fscope=other"),
            body: None,
            response: json!({"status":"deleted"}),
        },
    ])
    .await;
    let branches = client.database_branches(DB).await.unwrap();
    let branch: Branch = branches.into_iter().next().unwrap().into();
    assert_eq!(branch.is_preview_branch, Some(true));
    assert_eq!(branch.status.as_deref(), Some("active"));
    let uri = client
        .database_branch_connection(DB, &branch.id)
        .await
        .unwrap()
        .connection_uri;
    assert!(uri.ends_with("sslmode=require"));
    let created = client
        .create_database_branch(
            DB,
            nrz_api::BranchRequestBody {
                name: "preview".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(created.id, branch.id);
    client.delete_database_branch(DB, &branch.id).await.unwrap();
    assert!(pending.lock().unwrap().is_empty());
    server.abort();
}

#[tokio::test]
async fn database_controls_send_no_body_and_keep_operation_acknowledgements() {
    let ack = json!({"operation_id":DB,"operation_type":"start","resource":{"resource_type":"branch","resource_id":"main"},"status":"pending"});
    let (client, pending, server) = serve(vec![
        Exchange {
            method: "POST",
            path: format!("/v1/kaiki/databases/{DB}/start"),
            body: None,
            response: ack.clone(),
        },
        Exchange {
            method: "POST",
            path: format!("/v1/kaiki/databases/{DB}/stop"),
            body: None,
            response: ack,
        },
        Exchange {
            method: "GET",
            path: format!("/v1/kaiki/databases/{DB}/connection"),
            body: None,
            response: connection(),
        },
        Exchange {
            method: "DELETE",
            path: format!("/v1/kaiki/databases/{DB}"),
            body: None,
            response: json!({"status":"deleted"}),
        },
    ])
    .await;
    assert_eq!(
        client.start_database(DB).await.unwrap().status,
        nrz_api::Start200ResponseStatus::Pending
    );
    assert_eq!(
        client.stop_database(DB).await.unwrap().operation_id,
        DB.parse::<uuid::Uuid>().unwrap()
    );
    assert!(
        fetch_connection_uri(&client, DB, None)
            .await
            .unwrap()
            .ends_with("sslmode=require")
    );
    cmd_delete(&client, DB, true, true).await.unwrap();
    assert!(pending.lock().unwrap().is_empty());
    server.abort();
}

#[tokio::test]
async fn database_detail_and_attachment_reject_another_resource() {
    let detail = nrz_api::Database200Response3 {
        id: PROJECT.parse().unwrap(),
        ..Default::default()
    };
    let attachment = nrz_api::Database200ResponseDatumProjectAttachment {
        managed_database_id: PROJECT.parse().unwrap(),
        project_id: PROJECT.parse().unwrap(),
        ..Default::default()
    };
    let (client, pending, server) = serve(vec![
        Exchange {
            method: "GET",
            path: format!("/v1/kaiki/databases/{DB}"),
            body: None,
            response: serde_json::to_value(detail).unwrap(),
        },
        Exchange {
            method: "PATCH",
            path: format!("/v1/kaiki/databases/{DB}/attachments/{PROJECT}"),
            body: Some(json!({"autoInjectDbUrl":false})),
            response: serde_json::to_value(attachment).unwrap(),
        },
    ])
    .await;
    assert!(
        wire::database(&client, DB)
            .await
            .unwrap_err()
            .to_string()
            .contains("another database")
    );
    assert!(
        wire::attach(
            &client,
            DB,
            PROJECT,
            nrz_api::AttachmentRequestBody {
                auto_inject_db_url: Some(false),
                ..Default::default()
            }
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("another database or project")
    );
    assert!(pending.lock().unwrap().is_empty());
    server.abort();
}
