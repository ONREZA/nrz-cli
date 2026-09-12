use super::*;

fn access() -> ServerPreviewAccess {
    ServerPreviewAccess {
        project_id: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
        secret_id: "00000000-0000-0000-0000-000000000002".parse().unwrap(),
        note: "test".to_string(),
        expires_at: "2026-06-24T17:00:00Z".parse().unwrap(),
        ttl_seconds: 3600,
        header: nrz_api::AccessResponseItemHeader {
            name: BYPASS_HEADER_NAME.to_string(),
            value: "token-value".to_string(),
        },
        query: nrz_api::AccessResponseItemQuery {
            name: "_bypass".to_string(),
            value: "token-value".to_string(),
        },
    }
}

#[test]
fn preview_access_output_builds_agent_and_revoke_snippets() {
    let output = build_preview_access_output(
        access(),
        Some("https://preview.onreza.app/docs?x=1".to_string()),
    )
    .unwrap();

    assert_eq!(output.header_name, BYPASS_HEADER_NAME);
    assert_eq!(output.header_value, "token-value");
    assert_eq!(output.query_name, "_bypass");
    assert_eq!(output.query_value, "token-value");
    assert_eq!(output.expires_at, "2026-06-24T17:00:00Z");
    assert_eq!(output.ttl_seconds, 3600);
    assert!(output.ttl_enforced);
    assert_eq!(
        output.browser_url.as_deref(),
        Some("https://preview.onreza.app/docs?x=1&_bypass=token-value")
    );
    assert_eq!(
        output.curl_command,
        "curl -H 'X-ONREZA-Protection-Bypass: token-value' 'https://preview.onreza.app/docs?x=1'"
    );
    assert_eq!(
        output.revoke_command,
        "nrz preview revoke --project-id 00000000-0000-0000-0000-000000000001 --secret-id 00000000-0000-0000-0000-000000000002"
    );
}

#[test]
fn preview_access_hint_includes_url_only_when_available() {
    assert_eq!(
        preview_access_hint("project-1", Some("https://preview.onreza.app")),
        "nrz preview access --project-id project-1 --url https://preview.onreza.app"
    );
    assert_eq!(
        preview_access_hint("project-1", None),
        "nrz preview access --project-id project-1"
    );
}

#[test]
fn parse_ttl_accepts_common_units() {
    assert_eq!(parse_ttl_seconds("60").unwrap(), 60);
    assert_eq!(parse_ttl_seconds("15m").unwrap(), 900);
    assert_eq!(parse_ttl_seconds("1h").unwrap(), 3600);
    assert_eq!(parse_ttl_seconds("1d").unwrap(), 86_400);
}

#[test]
fn preview_output_rejects_invalid_lifetimes_and_transport() {
    for ttl in [-1, 0, 86_401] {
        let mut invalid = access();
        invalid.ttl_seconds = ttl;
        assert!(build_preview_access_output(invalid, None).is_err());
    }
    let mut invalid = access();
    invalid.header.name = "X-Unknown".into();
    assert!(build_preview_access_output(invalid, None).is_err());
}

#[tokio::test]
async fn preview_access_uses_generated_requests_and_checks_project_binding() {
    use axum::{
        Json, Router,
        routing::{delete, post},
    };
    use serde_json::{Value, json};
    let project_id = access().project_id.to_string();
    for wrong_project in [false, true] {
        let app = Router::new()
            .route(
                "/v1/preview-access/{id}",
                post(move |Json(body): Json<Value>| async move {
                    assert_eq!(body, json!({"note":"test", "ttlSeconds":3600}));
                    let mut result = access();
                    if wrong_project {
                        result.project_id = uuid::Uuid::nil();
                    }
                    Json(json!({"access":result}))
                }),
            )
            .route(
                "/v1/preview-access/{id}/{secret_id}",
                delete(|| async { Json(json!({"success":true})) }),
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
        let result = create_preview_access(&client, &project_id, "test".into(), None, 3600).await;
        if wrong_project {
            assert!(result.unwrap_err().to_string().contains("another project"));
        } else {
            let access = result.unwrap();
            assert_eq!(access.project_id, project_id);
            assert_eq!(access.header_value, "token-value");
            revoke_preview_access(&client, &project_id, &access.secret_id)
                .await
                .unwrap();
        }
        server.abort();
    }
}
