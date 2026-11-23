use super::*;
use crate::test_support::serve_api;
use axum::{
    Json, Router,
    extract::{Request, State},
};
use clap::Parser;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const DB: &str = "11111111-1111-4111-8111-111111111111";
const PROJECT: &str = "22222222-2222-4222-8222-222222222222";

struct Exchange {
    status: u16,
    key: Option<&'static str>,
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
                if let Some(key) = exchange.key {
                    assert_eq!(request.headers().get("idempotency-key").unwrap(), key);
                } else if exchange.method == "POST"
                    && (exchange.path.ends_with("/databases/")
                        || exchange.path.ends_with("/branches"))
                {
                    let key: uuid::Uuid = request
                        .headers()
                        .get("idempotency-key")
                        .expect("Create and Fork need a key")
                        .to_str()
                        .unwrap()
                        .parse()
                        .unwrap();
                    assert_eq!(key.get_version_num(), 7);
                }
                let body = axum::body::to_bytes(request.into_body(), 1024 * 1024)
                    .await
                    .unwrap();
                match exchange.body {
                    Some(expected) => {
                        assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), expected)
                    }
                    None => assert!(body.is_empty()),
                }
                (
                    axum::http::StatusCode::from_u16(exchange.status).unwrap(),
                    Json(exchange.response),
                )
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
    let created = nrz_api::Database200Response2Object {
        id: DB.parse().unwrap(),
        product_id: Some(PROJECT.parse().unwrap()),
        db_name: "primary".into(),
        password: "private-password".into(),
        ..Default::default()
    };
    let (client, pending, server) = serve(vec![Exchange {
        status: 200,
        key: None,
        method: "POST",
        path: "/v1/kaiki/databases/".into(),
        body: Some(json!({"dbName":"primary","cuSize":0.5})),
        response: serde_json::to_value(created.clone()).unwrap(),
    }])
    .await;
    cmd_create(
        &client,
        true,
        Some("primary".into()),
        Some(0.5),
        false,
        None,
    )
    .await
    .unwrap();
    assert!(pending.lock().unwrap().is_empty());
    let output = serde_json::to_value(CreateResponse::from(created)).unwrap();
    assert_eq!(
        output,
        json!({"id":DB,"productId":PROJECT,"dbName":"primary","status":"CREATING"})
    );
    server.abort();
}

#[tokio::test]
async fn branch_requests_encode_identifiers_and_preserve_list_metadata() {
    let mut listed = branch_json();
    listed.as_object_mut().unwrap().extend(json!({"localId":null,"parentKaikiBranchId":null,"ancestorLsn":null,"isPreviewBranch":true}).as_object().unwrap().clone());
    let (client, pending, server) = serve(vec![
        Exchange {
            status: 200,
            key: None,
            method: "GET",
            path: format!("/v1/kaiki/databases/{DB}/branches"),
            body: None,
            response: json!([listed]),
        },
        Exchange {
            status: 200,
            key: None,
            method: "GET",
            path: format!(
                "/v1/kaiki/databases/{DB}/branches/preview%2F..%2Fbranch%3Fscope=other/connection"
            ),
            body: None,
            response: connection(),
        },
        Exchange {
            status: 200,
            key: None,
            method: "POST",
            path: format!("/v1/kaiki/databases/{DB}/branches"),
            body: Some(json!({"name":"preview"})),
            response: branch_json(),
        },
        Exchange {
            status: 200,
            key: None,
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
            nrz_api::BranchRequestBody2 {
                name: "preview".into(),
                ..Default::default()
            },
            uuid::Uuid::now_v7(),
        )
        .await
        .unwrap();
    let nrz_api::Branch200Response3::Object(created) = created else {
        panic!("expected legacy branch")
    };
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
            status: 200,
            key: None,
            method: "POST",
            path: format!("/v1/kaiki/databases/{DB}/start"),
            body: None,
            response: ack.clone(),
        },
        Exchange {
            status: 200,
            key: None,
            method: "POST",
            path: format!("/v1/kaiki/databases/{DB}/stop"),
            body: None,
            response: ack,
        },
        Exchange {
            status: 200,
            key: None,
            method: "GET",
            path: format!("/v1/kaiki/databases/{DB}/connection"),
            body: None,
            response: connection(),
        },
        Exchange {
            status: 200,
            key: None,
            method: "DELETE",
            path: format!("/v1/kaiki/databases/{DB}"),
            body: None,
            response: json!({"status":"deleted"}),
        },
    ])
    .await;
    let nrz_api::Start200Response::Object(started) = client.start_database(DB).await.unwrap()
    else {
        panic!("expected legacy acknowledgement")
    };
    assert_eq!(
        started.status,
        nrz_api::Database200ResponseDatumRuntimeOperationStatus::Pending
    );
    let nrz_api::Start200Response::Object(stopped) = client.stop_database(DB).await.unwrap() else {
        panic!("expected legacy acknowledgement")
    };
    assert_eq!(stopped.operation_id, DB.parse::<uuid::Uuid>().unwrap());
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
            status: 200,
        key: None,
        method: "GET",
            path: format!("/v1/kaiki/databases/{DB}"),
            body: None,
            response: serde_json::to_value(detail).unwrap(),
        },
        Exchange {
            status: 200,
        key: None,
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
        let response = nrz_api::Database200ResponseDatumProjectAttachment {
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
            status: 200,
            key: None,
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

#[tokio::test]
async fn native_branch_deletion_resolves_names_before_admission_without_connection_reads() {
    let databases = nrz_api::Database200Response {
        data: vec![nrz_api::Database200ResponseDatum {
            id: DB.parse().unwrap(),
            db_name: "primary".to_owned(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut branch = branch_json();
    branch["id"] = json!(CHILD);
    branch.as_object_mut().unwrap().extend(
        json!({"localId":null,"parentKaikiBranchId":null,"ancestorLsn":null,"isPreviewBranch":true})
            .as_object()
            .unwrap()
            .clone(),
    );
    let (client, pending, server) = serve(vec![
        Exchange {
            status: 200,
            key: None,
            method: "GET",
            path: "/v1/kaiki/databases/".into(),
            body: None,
            response: serde_json::to_value(databases).unwrap(),
        },
        Exchange {
            status: 200,
            key: None,
            method: "GET",
            path: format!("/v1/kaiki/databases/{DB}/branches"),
            body: None,
            response: json!([branch]),
        },
        Exchange {
            status: 202,
            key: None,
            method: "DELETE",
            path: format!("/v1/kaiki/databases/{DB}/branches/{CHILD}"),
            body: None,
            response: operation("branch.delete", "pending"),
        },
    ])
    .await;
    execute(
        arguments(&["branches", "delete", "preview", "--database", "primary"]),
        true,
        &client,
        &ProjectConfig::default(),
    )
    .await
    .unwrap();
    assert!(pending.lock().unwrap().is_empty());
    server.abort();
}

const OPERATION: &str = "019f38da-5555-7000-8000-000000000003";
const ATTEMPT: &str = "019f38da-5555-7000-8000-000000000004";
const CHILD: &str = "33333333-3333-4333-8333-333333333333";

fn operation(kind: &str, status: &str) -> Value {
    json!({"operationId":OPERATION,"operationType":kind,"status":status,
        "result":{"databaseId":DB,"branchId":CHILD,"endpointId":"endpoint"}})
}

fn arguments(args: &[&str]) -> DbArgs {
    DbArgs::try_parse_from(std::iter::once("db").chain(args.iter().copied())).unwrap()
}

#[tokio::test]
async fn create_and_wait_use_the_receipt_without_reading_connection_or_live_database() {
    let mut accepted = operation("database.create", "pending");
    accepted["password"] = json!("private-post-password");
    for wait in [false, true] {
        let mut exchanges = vec![Exchange {
            status: 202,
            key: Some(ATTEMPT),
            method: "POST",
            path: "/v1/kaiki/databases/".into(),
            body: Some(json!({"dbName":"primary"})),
            response: accepted.clone(),
        }];
        if wait {
            exchanges.push(Exchange {
                status: 200,
                key: None,
                method: "GET",
                path: format!("/v1/kaiki/databases/{DB}/operations/{OPERATION}"),
                body: None,
                response: operation("database.create", "succeeded"),
            });
        }
        let (client, pending, server) = serve(exchanges).await;
        let mut args = vec!["create", "--name", "primary", "--idempotency-key", ATTEMPT];
        if wait {
            args.push("--wait");
        }
        execute(arguments(&args), true, &client, &ProjectConfig::default())
            .await
            .unwrap();
        assert!(pending.lock().unwrap().is_empty());
        server.abort();
    }
    let projected = DatabaseOperation::created(
        serde_json::from_value(accepted).unwrap(),
        ATTEMPT.parse().unwrap(),
    );
    let serialized = serde_json::to_string(&projected).unwrap();
    assert!(!serialized.contains("password"));
    assert!(!serialized.contains("private-post-password"));
    assert_eq!(
        serde_json::from_str::<Value>(&serialized).unwrap()["idempotencyKey"],
        ATTEMPT
    );
}

#[tokio::test]
async fn native_mutation_commands_handle_accepted_and_completed_receipts_without_resource_list_reads()
 {
    for status in ["pending", "succeeded"] {
        for (method, path, args, kind, body) in [
            (
                "POST",
                format!("/v1/kaiki/databases/{DB}/start"),
                vec!["start", DB],
                "endpoint.start",
                None,
            ),
            (
                "POST",
                format!("/v1/kaiki/databases/{DB}/stop"),
                vec!["stop", DB],
                "endpoint.stop",
                None,
            ),
            (
                "DELETE",
                format!("/v1/kaiki/databases/{DB}"),
                vec!["delete", DB, "--force"],
                "database.delete",
                None,
            ),
            (
                "POST",
                format!("/v1/kaiki/databases/{DB}/branches"),
                vec![
                    "branches",
                    "--database",
                    DB,
                    "create",
                    "dev",
                    "--idempotency-key",
                    ATTEMPT,
                ],
                "branch.create",
                Some(json!({"name":"dev"})),
            ),
            (
                "DELETE",
                format!("/v1/kaiki/databases/{DB}/branches/{CHILD}"),
                vec!["branches", "--database", DB, "delete", CHILD],
                "branch.delete",
                None,
            ),
        ] {
            let (client, pending, server) = serve(vec![Exchange {
                status: if status == "pending" { 202 } else { 200 },
                key: body.as_ref().map(|_| ATTEMPT),
                method,
                path,
                body,
                response: operation(kind, status),
            }])
            .await;
            execute(arguments(&args), true, &client, &ProjectConfig::default())
                .await
                .unwrap();
            assert!(pending.lock().unwrap().is_empty());
            server.abort();
        }
    }
}

#[tokio::test]
async fn completed_native_create_and_explicit_retry_preserve_one_key_and_never_output_the_password()
{
    let mut response = operation("database.create", "succeeded");
    response["password"] = json!("private-replay-password");
    let exchanges = (0..2)
        .map(|_| Exchange {
            status: 200,
            key: Some(ATTEMPT),
            method: "POST",
            path: "/v1/kaiki/databases/".into(),
            body: Some(json!({})),
            response: response.clone(),
        })
        .collect();
    let (client, pending, server) = serve(exchanges).await;
    for _ in 0..2 {
        execute(
            arguments(&["create", "--idempotency-key", ATTEMPT]),
            false,
            &client,
            &ProjectConfig::default(),
        )
        .await
        .unwrap();
    }
    assert!(pending.lock().unwrap().is_empty());
    server.abort();
}

#[tokio::test]
async fn operation_status_works_with_a_configured_database_id_after_deletion() {
    let (client, pending, server) = serve(vec![Exchange {
        status: 200,
        key: None,
        method: "GET",
        path: format!("/v1/kaiki/databases/{DB}/operations/{OPERATION}"),
        body: None,
        response: operation("database.delete", "succeeded"),
    }])
    .await;
    let config: ProjectConfig = toml::from_str(&format!("[db]\ndatabase = '{DB}'\n")).unwrap();
    execute(
        arguments(&["operation", OPERATION]),
        false,
        &client,
        &config,
    )
    .await
    .unwrap();
    assert!(pending.lock().unwrap().is_empty());
    server.abort();
}

#[tokio::test]
async fn operation_wait_rejects_foreign_receipts_and_reports_durable_failure() {
    for changed in ["database", "operation", "type", "failure"] {
        let mut receipt = operation("database.create", "succeeded");
        match changed {
            "database" => receipt["result"]["databaseId"] = json!(PROJECT),
            "operation" => receipt["operationId"] = json!(ATTEMPT),
            "type" => receipt["operationType"] = json!("branch.delete"),
            _ => {
                receipt["status"] = json!("failed");
                receipt["errorCode"] = json!("history_expired");
            }
        }
        let (client, pending, server) = serve(vec![Exchange {
            status: 200,
            key: None,
            method: "GET",
            path: format!("/v1/kaiki/databases/{DB}/operations/{OPERATION}"),
            body: None,
            response: receipt,
        }])
        .await;
        let initial = DatabaseOperation::from(
            serde_json::from_value::<nrz_api::Database200Response2ObjectOperation>(operation(
                "database.create",
                "pending",
            ))
            .unwrap(),
        );
        let error = wait_operation(&client, initial).await.unwrap_err();
        assert!(
            format!("{error:#}").contains(if changed == "failure" {
                "history_expired"
            } else if changed == "database" {
                "another database"
            } else {
                "identity"
            }) || changed == "type" && error.to_string().contains("type changed")
        );
        assert!(pending.lock().unwrap().is_empty());
        server.abort();
    }
}

#[tokio::test]
async fn native_commands_preserve_http_denials_without_falling_back_or_retrying() {
    for status in [400, 401, 403, 404, 409, 429] {
        let (client, pending, server) = serve(vec![Exchange {
            status,
            key: None,
            method: "POST",
            path: format!("/v1/kaiki/databases/{DB}/start"),
            body: None,
            response: json!({"code":"FORBIDDEN","message":"command denied"}),
        }])
        .await;
        let error = execute(
            arguments(&["start", DB]),
            true,
            &client,
            &ProjectConfig::default(),
        )
        .await
        .unwrap_err();
        let api_error = error
            .downcast_ref::<crate::api::StructuredApiError>()
            .unwrap();
        assert_eq!(api_error.status.as_u16(), status);
        assert!(pending.lock().unwrap().is_empty());
        server.abort();
    }
    assert!(DbArgs::try_parse_from(["db", "create", "--idempotency-key", DB]).is_err());
}
