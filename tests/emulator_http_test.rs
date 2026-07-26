//! Integration tests for Emulator HTTP server
//!
//! Tests:
//! - KV: POST set → POST get → проверка значения
//! - Health endpoint → {"status":"ok"}

use std::time::Duration;

use nrz::emulator::kv::KvStore;
use nrz::emulator::server::{EMULATOR_TOKEN_HEADER, EmulatorServer};

/// Start test server and return base URL
async fn start_test_server() -> (String, KvStore, u16, reqwest::Client) {
    let kv = KvStore::new();

    // Find a free port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = EmulatorServer::new(kv.clone(), port, "127.0.0.1").unwrap();
    let token = server.token().to_string();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        EMULATOR_TOKEN_HEADER,
        reqwest::header::HeaderValue::from_str(&token).unwrap(),
    );
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    tokio::spawn(async move {
        server.start().await.unwrap();
    });

    let base_url = format!("http://127.0.0.1:{port}");

    // Wait for server to be ready
    for _ in 0..50 {
        match client.get(format!("{base_url}/__nrz/health")).send().await {
            Ok(resp) if resp.status().is_success() => break,
            _ => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }

    (base_url, kv, port, client)
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let (base_url, _kv, _, client) = start_test_server().await;

    let resp = client
        .get(format!("{base_url}/__nrz/health"))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn emulator_rejects_requests_without_session_token() {
    let (base_url, _kv, _, _) = start_test_server().await;

    let response = reqwest::Client::new()
        .post(format!("{base_url}/__nrz/kv/get"))
        .json(&serde_json::json!({ "args": ["key"] }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn kv_set_and_get() {
    let (base_url, _kv, _, client) = start_test_server().await;

    // Set a key
    let set_resp = client
        .post(format!("{base_url}/__nrz/kv/set"))
        .json(&serde_json::json!({
            "args": ["test_key", "test_value"]
        }))
        .send()
        .await
        .unwrap();

    assert!(set_resp.status().is_success());
    let set_body: serde_json::Value = set_resp.json().await.unwrap();
    assert!(set_body.is_null());

    // Get the key
    let get_resp = client
        .post(format!("{base_url}/__nrz/kv/get"))
        .json(&serde_json::json!({
            "args": ["test_key"]
        }))
        .send()
        .await
        .unwrap();

    assert!(get_resp.status().is_success());
    let get_body: serde_json::Value = get_resp.json().await.unwrap();
    assert_eq!(get_body, "test_value");
}

#[tokio::test]
async fn kv_get_nonexistent_key_returns_null() {
    let (base_url, _kv, _, client) = start_test_server().await;

    let resp = client
        .post(format!("{base_url}/__nrz/kv/get"))
        .json(&serde_json::json!({
            "args": ["nonexistent_key"]
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.is_null());
}

#[tokio::test]
async fn kv_get_many_returns_values() {
    let (base_url, _kv, _, client) = start_test_server().await;

    // Set multiple keys
    for (k, v) in [("m1", "val1"), ("m2", "val2"), ("m3", "val3")] {
        client
            .post(format!("{base_url}/__nrz/kv/set"))
            .json(&serde_json::json!({ "args": [k, v] }))
            .send()
            .await
            .unwrap();
    }

    // Get many
    let resp = client
        .post(format!("{base_url}/__nrz/kv/getMany"))
        .json(&serde_json::json!({ "args": [["m1", "m2", "missing", "m3"]] }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    let values = body["values"].as_array().unwrap();
    assert_eq!(values.len(), 4);
    assert_eq!(values[0], "val1");
    assert_eq!(values[1], "val2");
    assert!(values[2].is_null());
    assert_eq!(values[3], "val3");
}

#[tokio::test]
async fn kv_get_with_metadata_returns_value_and_metadata() {
    let (base_url, _kv, _, client) = start_test_server().await;

    // Set key with metadata (args: [key, value, ttl, metadata])
    client
        .post(format!("{base_url}/__nrz/kv/set"))
        .json(&serde_json::json!({ "args": ["mk", "mv", 0, "my_meta"] }))
        .send()
        .await
        .unwrap();

    // Get with metadata
    let resp = client
        .post(format!("{base_url}/__nrz/kv/getWithMetadata"))
        .json(&serde_json::json!({ "args": ["mk"] }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["value"], "mv");
    assert_eq!(body["metadata"], "my_meta");

    // Get with metadata for key without metadata
    client
        .post(format!("{base_url}/__nrz/kv/set"))
        .json(&serde_json::json!({ "args": ["nk", "nv"] }))
        .send()
        .await
        .unwrap();

    let resp2 = client
        .post(format!("{base_url}/__nrz/kv/getWithMetadata"))
        .json(&serde_json::json!({ "args": ["nk"] }))
        .send()
        .await
        .unwrap();

    assert!(resp2.status().is_success());
    let body2: serde_json::Value = resp2.json().await.unwrap();
    assert_eq!(body2["value"], "nv");
    assert!(body2["metadata"].is_null());
}

#[tokio::test]
async fn kv_get_with_metadata_nonexistent_returns_nulls() {
    let (base_url, _kv, _, client) = start_test_server().await;

    let resp = client
        .post(format!("{base_url}/__nrz/kv/getWithMetadata"))
        .json(&serde_json::json!({ "args": ["nonexistent"] }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["value"].is_null());
    assert!(body["metadata"].is_null());
}
