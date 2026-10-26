use super::*;
use crate::test_support::serve_api;
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
    let (client, server) = serve_api(app).await;
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
async fn create_database_keeps_standalone_ownership_without_implicit_attachment() {
    let key = "create-primary:1";
    let accepted = json!({"id": DB, "workspaceId": PROJECT, "operationId": PROJECT, "status": "CREATING"});
    let response = accepted.clone();
    let app = Router::new().fallback(move |request: Request| {
        let response = response.clone();
        async move {
            assert_eq!(request.method(), "POST");
            assert_eq!(request.uri(), "/v1/kaiki/databases/");
            assert_eq!(request.headers()["Idempotency-Key"], key);
            let body = axum::body::to_bytes(request.into_body(), 1024).await.unwrap();
            assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), json!({"dbName":"primary","cuSize":0.5}));
            (axum::http::StatusCode::ACCEPTED, Json(response))
        }
    });
    let (client, server) = serve_api(app).await;
    cmd_create(&client, true, Some("primary".into()), Some(0.5), Some(key.into()), false)
        .await
        .unwrap();
    let output = serde_json::to_value(wire::created(serde_json::from_value(accepted).unwrap(), key.into())).unwrap();
    assert_eq!(
        output,
        json!({"id":DB,"workspaceId":PROJECT,"operationId":PROJECT,"status":"CREATING","idempotencyKey":key})
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
    let detail = nrz_api::ManagedDatabaseNativeDetail {
        id: PROJECT.parse().unwrap(),
        ..Default::default()
    };
    let attachment = nrz_api::ManagedDatabaseProjectAttachment {
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
            body: Some(json!({"serviceId":PROJECT,"productEnvironmentId":PROJECT,"ownerProductEnvironmentId":PROJECT,"providerBranchId":"exact-provider-branch","autoInjectDbUrl":false})),
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
                service_id: PROJECT.parse().unwrap(),
                product_environment_id: PROJECT.parse().unwrap(),
                owner_product_environment_id: PROJECT.parse().unwrap(),
                provider_branch_id: "exact-provider-branch".into(),
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

#[tokio::test]
async fn attachment_preserves_explicit_targets_and_rejects_another_binding() {
    let body = nrz_api::AttachmentRequestBody {
        service_id: DB.parse().unwrap(),
        product_environment_id: PROJECT.parse().unwrap(),
        owner_product_environment_id: DB.parse().unwrap(),
        provider_branch_id: "provider/exact".into(),
        auto_inject_db_url: Some(true),
        ..Default::default()
    };
    for changed in [false, true] {
        let response = nrz_api::ManagedDatabaseProjectAttachment {
            managed_database_id: DB.parse().unwrap(),
            project_id: PROJECT.parse().unwrap(),
            service_id: body.service_id,
            product_environment_id: body.product_environment_id,
            owner_product_environment_id: body.owner_product_environment_id,
            provider_branch_id: if changed {
                "other-branch".into()
            } else {
                body.provider_branch_id.clone()
            },
            ..Default::default()
        };
        let (client, pending, server) = serve(vec![Exchange {
            method: "PATCH",
            path: format!("/v1/kaiki/databases/{DB}/attachments/{PROJECT}"),
            body: Some(serde_json::to_value(&body).unwrap()),
            response: serde_json::to_value(response).unwrap(),
        }])
        .await;
        let result = wire::attach(&client, DB, PROJECT, body.clone()).await;
        if changed {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("another Service, ProductEnvironment or provider branch")
            );
        } else {
            assert_eq!(result.unwrap().provider_branch_id, body.provider_branch_id);
        }
        assert!(pending.lock().unwrap().is_empty());
        server.abort();
    }
}

#[tokio::test]
async fn exact_database_selection_needs_no_application_attachment() {
    let (client, pending, server) = serve(vec![]).await;
    assert_eq!(
        resolve_db_by_id_or_name(&client, Some(PROJECT), DB)
            .await
            .unwrap(),
        DB
    );
    assert!(pending.lock().unwrap().is_empty());
    server.abort();
}
