use super::logs::format_log_entry;
use crate::cli::LogsArgs;
use nrz_api::RuntimeLog200Response as LogsResponse;

#[test]
fn logs_response_accepts_new_entries_contract() {
    let response: LogsResponse = serde_json::from_str(
        r#"{
          "entries": [
            {
              "timestamp": "2026-06-07T20:06:36.000Z",
              "method": "GET",
              "path": "/api/hello",
              "status": 200,
              "durationMs": 12.5,
              "bytesSent": 0, "bytesReceived": 0, "clientIp": "127.0.0.1",
              "userAgent": "test", "isBot": false,
              "functionName": "stage-smoke-hello"
            }
          ],
          "pagination": { "limit": 50, "hasMore": false, "nextCursor": null },
          "filters": {
            "stream": "access",
            "startTime": "2026-06-07T19:06:36.000Z",
            "endTime": "2026-06-07T20:06:36.000Z"
          }
        }"#,
    )
    .unwrap();

    assert_eq!(response.entries.len(), 1);
    assert_eq!(
        serde_json::to_value(&response).unwrap()["pagination"],
        serde_json::json!({ "limit": 50, "hasMore": false, "nextCursor": null })
    );
    assert_eq!(
        format_log_entry(&serde_json::to_value(&response.entries[0]).unwrap()),
        "[2026-06-07T20:06:36.000Z] [200] GET /api/hello 12.5ms function=stage-smoke-hello"
    );
}

#[test]
fn logs_human_formatter_uses_message_entries() {
    let entry = serde_json::json!({
        "timestamp": "2026-06-07T20:06:36.000Z",
        "functionLogLevel": "warn",
        "message": "slow path"
    });

    assert_eq!(
        format_log_entry(&entry),
        "[2026-06-07T20:06:36.000Z] [warn] slow path"
    );
}

#[test]
fn logs_human_formatter_emits_one_safe_terminal_line() {
    let entry = serde_json::json!({
        "timestamp": "2026-06-07T20:06:36.000Z",
        "functionLogLevel": "warn",
        "message": "\u{1b}]0;forged title\u{7}first\rsecond\nthird"
    });

    assert_eq!(
        format_log_entry(&entry),
        "[2026-06-07T20:06:36.000Z] [warn] first second third"
    );
}

#[tokio::test]
async fn runtime_logs_encode_search_and_deployment_filters_with_generated_client() {
    use axum::{Json, Router, extract::Query, routing::get};
    use std::collections::HashMap;
    let id = "00000000-0000-0000-0000-000000000001";
    let app = Router::new().route("/v1/projects/{id}/runtime-logs", get(move |Query(query):Query<HashMap<String,String>>| async move {
        assert_eq!(query["search"], "a&b + /שלום");
        assert_eq!(query["deploymentId"], id);
        assert_eq!(query["limit"], "73");
        Json(serde_json::json!({"entries":[{"timestamp":"2026-09-12T00:00:00Z", "level":"WARN", "message":"slow path", "deploymentId":id}],
            "pagination":{"limit":73,"hasMore":false,"nextCursor":null},
            "filters":{"stream":"access","startTime":"2026-09-11T00:00:00Z","endTime":"2026-09-12T00:00:00Z"}}))
    }));
    let (client, server) = crate::test_support::serve_api(app).await;
    let response = client
        .runtime_logs(
            id,
            nrz_api::GetV1projectsByIdRuntimeLogsRequestQuery {
                limit: Some(serde_json::json!(73)),
                deployment_id: Some(id.parse().unwrap()),
                search: Some("a&b + /שלום".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        format_log_entry(&serde_json::to_value(&response.entries[0]).unwrap()),
        "[2026-09-12T00:00:00Z] [WARN] slow path"
    );
    server.abort();
}

#[tokio::test]
async fn logs_run_requires_valid_authentication_in_both_output_modes() {
    for json in [false, true] {
        let error = super::logs::run(
            LogsArgs {
                project_id: None,
                limit: 50,
                deployment_id: None,
                search: None,
            },
            json,
            Some("invalid\ntoken"),
            None,
            &nrz::config::ProjectConfig::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "invalid token format", "json={json}");
    }
}

#[tokio::test]
async fn logs_handler_rejects_invalid_deployment_before_transport() {
    let error = super::logs::run(
        LogsArgs {
            project_id: Some("00000000-0000-0000-0000-000000000001".into()),
            limit: 50,
            deployment_id: Some("deployment/../other?force=true".into()),
            search: None,
        },
        true,
        Some("test-token"),
        None,
        &nrz::config::ProjectConfig::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "invalid deployment ID");
}
