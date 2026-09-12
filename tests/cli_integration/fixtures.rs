use super::*;
use axum::{
    Json, Router,
    body::Bytes,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};

pub(super) fn spawn_project_settings_mock() -> String {
    let app = Router::new().route(
        "/v1/projects/{project_id}",
        get(
            |axum::extract::Path(project_id): axum::extract::Path<String>| async {
                let mut body = serde_json::to_value(project(project_id)).unwrap();
                let settings = json!({
                    "frameworkPreset": "vite",
                    "rootDirectory": ".",
                    "packageManager": "NPM",
                    "buildCommand": "npm run server-build",
                    "buildCommandSource": "USER",
                    "outputDirectory": "server-dist",
                    "outputDirectorySource": "USER"
                });
                body.as_object_mut()
                    .unwrap()
                    .extend(settings.as_object().unwrap().clone());
                Json(body)
            },
        ),
    );
    support::api_mock::spawn(app)
}

pub(super) fn spawn_project_settings_failure_mock(status: StatusCode) -> String {
    let app = Router::new().route(
        "/v1/projects/{project_id}",
        get(move || async move {
            (
                status,
                Json(json!({
                    "error": "project settings unavailable"
                })),
            )
                .into_response()
        }),
    );
    support::api_mock::spawn(app)
}

pub(super) fn spawn_preview_access_mock() -> String {
    let app = Router::new()
        .route(
            "/v1/preview-access/{project_id}",
            post(
                |axum::extract::Path(project_id): axum::extract::Path<String>,
                 Json(body): Json<serde_json::Value>| async move {
                    assert!(
                        body.get("url").is_none(),
                        "preview access request body must stay compatible with strict server schema"
                    );
                    Json(json!({
                        "access": {
                            "projectId": project_id,
                            "secretId": "00000000-0000-0000-0000-000000000002",
                            "note": body["note"].as_str().unwrap_or(""),
                            "expiresAt": "2026-06-24T17:00:00.000Z",
                            "ttlSeconds": body["ttlSeconds"].as_u64().unwrap_or(0),
                            "header": {
                                "name": "X-ONREZA-Protection-Bypass",
                                "value": "token-value"
                            },
                            "query": {
                                "name": "_bypass",
                                "value": "token-value"
                            }
                        }
                    }))
                },
            ),
        )
        .route(
            "/v1/preview-access/{project_id}/{secret_id}",
            delete(
                |axum::extract::Path(_params): axum::extract::Path<
                    std::collections::HashMap<String, String>,
                >| async move { Json(json!({ "success": true })) },
            ),
        );
    support::api_mock::spawn(app)
}

pub(super) fn spawn_device_flow_mock() -> String {
    let app = Router::new()
        .route(
            "/v1/device/",
            post(|body: Bytes| async move {
                assert!(body.is_empty(), "device code request body must be empty");
                Json(json!({
                    "device_code": "0123456789abcdef0123456789abcdef01234567",
                    "user_code": "ABCD-EFGH",
                    "verification_uri": "https://example.test/device",
                    "verification_uri_complete": "https://example.test/device?code=ABCD-EFGH",
                    "expires_in": 60,
                    "interval": 1
                }))
            }),
        )
        .route(
            "/v1/device/token",
            post(|Json(body): Json<serde_json::Value>| async move {
                assert_eq!(
                    body["device_code"],
                    "0123456789abcdef0123456789abcdef01234567"
                );
                assert_eq!(
                    body["grant_type"],
                    "urn:ietf:params:oauth:grant-type:device_code"
                );
                Json(json!({
                    "access_token": "access-token",
                    "expires_in": 3600,
                    "token_type": "Bearer",
                    "workspace_slug": "test-workspace",
                    "workspace_name": "Test Workspace"
                }))
            }),
        );
    support::api_mock::spawn(app)
}

pub(super) fn spawn_nullable_project_mock() -> String {
    let app =
        Router::new()
            .route(
                "/v1/projects/",
                get(|| async {
                    Json(nrz_api::Project200Response {
                        projects: vec![nrz_api::Project200ResponseProject {
                            id: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
                            name: "internal-name".to_string(),
                            display_name: None,
                            ..Default::default()
                        }],
                        total: 1,
                    })
                }),
            )
            .route(
                "/v1/projects/{project_id}",
                get(
                    |axum::extract::Path(id): axum::extract::Path<String>| async move {
                        Json(project(id))
                    },
                ),
            );
    support::api_mock::spawn(app)
}

fn project(id: String) -> nrz_api::Project200Response3 {
    nrz_api::Project200Response3 {
        id: id.parse().unwrap(),
        name: "internal-name".to_string(),
        display_name: None,
        ..Default::default()
    }
}
