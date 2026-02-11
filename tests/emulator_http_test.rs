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
struct DbQueryRequest {
    sql: String,
    #[serde(default)]
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
    let key = req.args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
    let value = req.args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
    state.kv.set(key, value, 0);
    Json(serde_json::json!("OK"))
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
        Err(e) => return Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
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
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
    
    let state = AppState {
        kv: kv.clone(),
        db: Arc::new(Mutex::new(conn)),
    };
    
    let app = Router::new()
        .route("/__nrz/health", get(health))
        .route("/__nrz/kv/get", post(kv_get))
        .route("/__nrz/kv/set", post(kv_set))
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

    let resp = reqwest::get(format!("{}/__nrz/health", base_url)).await.unwrap();
    
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
    assert_eq!(set_body, "OK");

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
