use super::logs::{LogsResponse, format_log_entry};

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
        format_log_entry(&response.entries[0]),
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
