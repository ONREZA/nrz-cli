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
          "pagination": { "limit": 50, "offset": 0, "hasMore": false },
          "filters": { "stream": "access" }
        }"#,
    )
    .unwrap();

    assert_eq!(response.entries.len(), 1);
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
