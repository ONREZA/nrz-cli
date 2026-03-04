//! Integration tests for Emulator HTTP server
//!
//! Tests:
//! - KV: POST set → POST get → проверка значения
//! - DB: POST exec (CREATE TABLE) → POST query (SELECT) → проверка results
//! - Health endpoint → {"status":"ok"}

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use nrz::emulator::kv::KvStore;

// --- Types copied from server.rs for testing ---

#[derive(Deserialize)]
struct KvRequest {
    args: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct DbQueryRequest {
    sql: String,
    #[serde(default, alias = "params")]
    bindings: Vec<serde_json::Value>,
    mode: String,
}

#[derive(Deserialize)]
struct DbExecRequest {
    sql: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Clone)]
struct AppState {
    kv: KvStore,
    db: Arc<Mutex<Connection>>,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn kv_get(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<KvRequest>,
) -> Json<serde_json::Value> {
    let key = req.args.first().and_then(|v| v.as_str()).unwrap_or("");
    Json(serde_json::to_value(state.kv.get(key)).unwrap())
}

async fn kv_set(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<KvRequest>,
) -> Json<serde_json::Value> {
    let key = req
        .args
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let value = req
        .args
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ttl = req.args.get(2).and_then(|v| v.as_u64()).unwrap_or(0);
    let metadata = req
        .args
        .get(3)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    state.kv.set(key, value, ttl, metadata);
    Json(serde_json::json!(null))
}

async fn kv_get_many(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let keys: Vec<String> = req
        .args
        .first()
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .ok_or((
            axum::http::StatusCode::BAD_REQUEST,
            "kv.getMany requires args: [keys[]]".into(),
        ))?;
    let values = state.kv.get_many(&keys);
    Ok(Json(serde_json::json!({ "values": values })))
}

async fn kv_get_with_metadata(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let key = req.args.first().and_then(|v| v.as_str()).ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        "kv.getWithMetadata requires args: [key]".into(),
    ))?;
    let (value, metadata) = state.kv.get_with_metadata(key);
    Ok(Json(
        serde_json::json!({ "value": value, "metadata": metadata }),
    ))
}

async fn db_exec(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<DbExecRequest>,
) -> Json<serde_json::Value> {
    let conn = state.db.lock().unwrap();
    match conn.execute_batch(&req.sql) {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "results": [],
            "meta": {
                "changes": conn.changes() as i64,
                "last_row_id": conn.last_insert_rowid(),
                "duration": 0.0
            }
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

async fn db_query(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<DbQueryRequest>,
) -> Json<serde_json::Value> {
    use std::time::Instant;

    let start = Instant::now();
    let conn = state.db.lock().unwrap();

    let mut stmt = match conn.prepare(&req.sql) {
        Ok(s) => s,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            }));
        }
    };

    // Bind parameters
    for (i, val) in req.bindings.iter().enumerate() {
        let idx = i + 1;
        let _ = match val {
            serde_json::Value::Null => stmt.raw_bind_parameter(idx, rusqlite::types::Null),
            serde_json::Value::Bool(b) => stmt.raw_bind_parameter(idx, *b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    stmt.raw_bind_parameter(idx, i)
                } else if let Some(f) = n.as_f64() {
                    stmt.raw_bind_parameter(idx, f)
                } else {
                    stmt.raw_bind_parameter(idx, n.to_string())
                }
            }
            serde_json::Value::String(s) => stmt.raw_bind_parameter(idx, s.as_str()),
            other => stmt.raw_bind_parameter(idx, other.to_string()),
        };
    }

    // Get column names
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();

    let mut raw_rows = stmt.raw_query();
    while let Some(row) = raw_rows.next().unwrap() {
        let mut obj = serde_json::Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let val: serde_json::Value = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => serde_json::Value::Null,
                Ok(rusqlite::types::ValueRef::Integer(n)) => serde_json::json!(n),
                Ok(rusqlite::types::ValueRef::Real(f)) => serde_json::json!(f),
                Ok(rusqlite::types::ValueRef::Text(s)) => {
                    serde_json::Value::String(String::from_utf8_lossy(s).into_owned())
                }
                Ok(rusqlite::types::ValueRef::Blob(b)) => {
                    serde_json::Value::String(format!("<blob {} bytes>", b.len()))
                }
                Err(_) => serde_json::Value::Null,
            };
            obj.insert(name.clone(), val);
        }
        rows.push(serde_json::Value::Object(obj));
    }

    let duration = start.elapsed().as_secs_f64();

    Json(serde_json::json!({
        "success": true,
        "results": rows,
        "meta": {
            "changes": conn.changes() as i64,
            "last_row_id": conn.last_insert_rowid(),
            "duration": duration
        }
    }))
}

/// Helper to create a temporary database path
fn temp_db_path() -> (PathBuf, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    (temp_dir.path().join("test.db"), temp_dir)
}

/// Start test server and return base URL
async fn start_test_server() -> (String, KvStore, tempfile::TempDir) {
    let kv = KvStore::new();
    let (db_path, temp_dir) = temp_db_path();

    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .unwrap();

    let state = AppState {
        kv: kv.clone(),
        db: Arc::new(Mutex::new(conn)),
    };

    let app = Router::new()
        .route("/__nrz/health", get(health))
        .route("/__nrz/kv/get", post(kv_get))
        .route("/__nrz/kv/set", post(kv_set))
        .route("/__nrz/kv/getMany", post(kv_get_many))
        .route("/__nrz/kv/getWithMetadata", post(kv_get_with_metadata))
        .route("/__nrz/db/query", post(db_query))
        .route("/__nrz/db/exec", post(db_exec))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let base_url = format!("http://127.0.0.1:{}", port);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for server to be ready
    for _ in 0..50 {
        match reqwest::get(format!("{}/__nrz/health", base_url)).await {
            Ok(resp) if resp.status().is_success() => break,
            _ => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }

    (base_url, kv, temp_dir)
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let (base_url, _kv, _temp) = start_test_server().await;

    let resp = reqwest::get(format!("{}/__nrz/health", base_url))
        .await
        .unwrap();

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn kv_set_and_get() {
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    // Set a key
    let set_resp = client
        .post(format!("{}/__nrz/kv/set", base_url))
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
        .post(format!("{}/__nrz/kv/get", base_url))
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
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/__nrz/kv/get", base_url))
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
async fn db_exec_and_query() {
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    // Create table using exec
    let exec_resp = client
        .post(format!("{}/__nrz/db/exec", base_url))
        .json(&serde_json::json!({
            "sql": "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)"
        }))
        .send()
        .await
        .unwrap();

    assert!(exec_resp.status().is_success());
    let exec_body: serde_json::Value = exec_resp.json().await.unwrap();
    assert_eq!(exec_body["success"], true);

    // Insert data using query (run mode)
    let insert_resp = client
        .post(format!("{}/__nrz/db/query", base_url))
        .json(&serde_json::json!({
            "sql": "INSERT INTO users (name) VALUES ('Alice'), ('Bob')",
            "bindings": [],
            "mode": "run"
        }))
        .send()
        .await
        .unwrap();

    assert!(insert_resp.status().is_success());
    let insert_body: serde_json::Value = insert_resp.json().await.unwrap();
    assert_eq!(insert_body["success"], true);
    assert_eq!(insert_body["meta"]["changes"], 2);

    // Query data
    let query_resp = client
        .post(format!("{}/__nrz/db/query", base_url))
        .json(&serde_json::json!({
            "sql": "SELECT * FROM users ORDER BY id",
            "bindings": [],
            "mode": "all"
        }))
        .send()
        .await
        .unwrap();

    assert!(query_resp.status().is_success());
    let query_body: serde_json::Value = query_resp.json().await.unwrap();
    assert_eq!(query_body["success"], true);

    let results = query_body["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);

    // Check first row
    assert_eq!(results[0]["id"], 1);
    assert_eq!(results[0]["name"], "Alice");

    // Check second row
    assert_eq!(results[1]["id"], 2);
    assert_eq!(results[1]["name"], "Bob");
}

#[tokio::test]
async fn db_query_with_bindings() {
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    // Create table
    client
        .post(format!("{}/__nrz/db/exec", base_url))
        .json(&serde_json::json!({
            "sql": "CREATE TABLE items (id INTEGER PRIMARY KEY, value INTEGER)"
        }))
        .send()
        .await
        .unwrap();

    // Insert with binding
    client
        .post(format!("{}/__nrz/db/query", base_url))
        .json(&serde_json::json!({
            "sql": "INSERT INTO items (value) VALUES (?)",
            "bindings": [42],
            "mode": "run"
        }))
        .send()
        .await
        .unwrap();

    // Query with binding
    let resp = client
        .post(format!("{}/__nrz/db/query", base_url))
        .json(&serde_json::json!({
            "sql": "SELECT * FROM items WHERE value = ?",
            "bindings": [42],
            "mode": "all"
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["value"], 42);
}

#[tokio::test]
async fn kv_get_many_returns_values() {
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    // Set multiple keys
    for (k, v) in [("m1", "val1"), ("m2", "val2"), ("m3", "val3")] {
        client
            .post(format!("{}/__nrz/kv/set", base_url))
            .json(&serde_json::json!({ "args": [k, v] }))
            .send()
            .await
            .unwrap();
    }

    // Get many
    let resp = client
        .post(format!("{}/__nrz/kv/getMany", base_url))
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
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    // Set key with metadata (args: [key, value, ttl, metadata])
    client
        .post(format!("{}/__nrz/kv/set", base_url))
        .json(&serde_json::json!({ "args": ["mk", "mv", 0, "my_meta"] }))
        .send()
        .await
        .unwrap();

    // Get with metadata
    let resp = client
        .post(format!("{}/__nrz/kv/getWithMetadata", base_url))
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
        .post(format!("{}/__nrz/kv/set", base_url))
        .json(&serde_json::json!({ "args": ["nk", "nv"] }))
        .send()
        .await
        .unwrap();

    let resp2 = client
        .post(format!("{}/__nrz/kv/getWithMetadata", base_url))
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
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/__nrz/kv/getWithMetadata", base_url))
        .json(&serde_json::json!({ "args": ["nonexistent"] }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["value"].is_null());
    assert!(body["metadata"].is_null());
}

#[tokio::test]
async fn db_query_with_params_alias() {
    let (base_url, _kv, _temp) = start_test_server().await;
    let client = reqwest::Client::new();

    // Create table
    client
        .post(format!("{}/__nrz/db/exec", base_url))
        .json(&serde_json::json!({
            "sql": "CREATE TABLE params_test (id INTEGER PRIMARY KEY, val TEXT)"
        }))
        .send()
        .await
        .unwrap();

    // Insert using "params" alias instead of "bindings"
    client
        .post(format!("{}/__nrz/db/query", base_url))
        .json(&serde_json::json!({
            "sql": "INSERT INTO params_test (val) VALUES (?)",
            "params": ["hello"],
            "mode": "run"
        }))
        .send()
        .await
        .unwrap();

    // Query to verify
    let resp = client
        .post(format!("{}/__nrz/db/query", base_url))
        .json(&serde_json::json!({
            "sql": "SELECT val FROM params_test",
            "bindings": [],
            "mode": "all"
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["val"], "hello");
}
