use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::kv::KvStore;

/// Local HTTP server for the emulator.
///
/// Runs alongside the framework dev server. The JS bootstrap
/// (injected into Node.js) proxies ONREZA.kv/db calls to this server.
pub struct EmulatorServer {
    pub kv: KvStore,
    pub db_path: PathBuf,
    pub addr: SocketAddr,
}

#[derive(Clone)]
struct AppState {
    kv: KvStore,
    db: Arc<Mutex<Connection>>,
}

// --- Request/Response types ---

#[derive(Deserialize)]
struct KvRequest {
    args: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DbQueryRequest {
    sql: String,
    #[serde(default, alias = "params")]
    bindings: Vec<serde_json::Value>,
    mode: String,
    #[serde(default)]
    column: Option<String>,
    #[serde(default, rename = "columnNames")]
    column_names: Option<bool>,
}

#[derive(Deserialize)]
struct DbBatchRequest {
    statements: Vec<DbBatchStatement>,
}

#[derive(Deserialize)]
struct DbBatchStatement {
    sql: String,
    #[serde(default, alias = "params")]
    bindings: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DbExecRequest {
    sql: String,
}

#[derive(Serialize)]
struct QueryResponse {
    results: serde_json::Value,
    success: bool,
    meta: QueryResponseMeta,
}

#[derive(Serialize)]
struct QueryResponseMeta {
    changes: i64,
    last_row_id: i64,
    duration: f64,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

type AppError = (StatusCode, String);

impl EmulatorServer {
    pub fn new(kv: KvStore, db_path: PathBuf, port: u16, host: &str) -> anyhow::Result<Self> {
        let ip: std::net::IpAddr = host
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid host '{}': {}", host, e))?;
        Ok(Self {
            kv,
            db_path,
            addr: SocketAddr::new(ip, port),
        })
    }

    /// Start the emulator HTTP server.
    pub async fn start(&self) -> anyhow::Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let state = AppState {
            kv: self.kv.clone(),
            db: Arc::new(Mutex::new(conn)),
        };

        let app = Router::new()
            .route("/__nrz/health", get(health))
            .route("/__nrz/kv/get", post(kv_get))
            .route("/__nrz/kv/set", post(kv_set))
            .route("/__nrz/kv/delete", post(kv_delete))
            .route("/__nrz/kv/has", post(kv_has))
            .route("/__nrz/kv/list", post(kv_list))
            .route("/__nrz/kv/getMany", post(kv_get_many))
            .route("/__nrz/kv/getWithMetadata", post(kv_get_with_metadata))
            .route("/__nrz/db/query", post(db_query))
            .route("/__nrz/db/batch", post(db_batch))
            .route("/__nrz/db/exec", post(db_exec))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        tracing::info!(addr = %self.addr, "emulator server listening");

        axum::serve(listener, app).await?;
        Ok(())
    }
}

// --- Health ---

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

// --- KV handlers ---

async fn kv_get(
    State(state): State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<impl IntoResponse, AppError> {
    let key = req.args.first().and_then(|v| v.as_str()).ok_or((
        StatusCode::BAD_REQUEST,
        "kv.get requires args: [key]".into(),
    ))?;
    Ok(Json(serde_json::to_value(state.kv.get(key)).unwrap()))
}

async fn kv_set(
    State(state): State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<impl IntoResponse, AppError> {
    let key = req
        .args
        .first()
        .and_then(|v| v.as_str())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "kv.set requires args: [key, value]".into(),
        ))?
        .to_string();
    let value = req
        .args
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "kv.set requires args: [key, value]".into(),
        ))?
        .to_string();
    let ttl = match req.args.get(2) {
        Some(v) => v.as_u64().unwrap_or_else(|| {
            tracing::warn!("kv.set: TTL arg is not a valid u64: {v}, defaulting to 0");
            0
        }),
        None => 0,
    };
    let metadata = req
        .args
        .get(3)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    state.kv.set(key, value, ttl, metadata);
    Ok(Json(serde_json::json!(null)))
}

async fn kv_delete(
    State(state): State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<impl IntoResponse, AppError> {
    let key = req.args.first().and_then(|v| v.as_str()).ok_or((
        StatusCode::BAD_REQUEST,
        "kv.delete requires args: [key]".into(),
    ))?;
    Ok(Json(serde_json::json!(state.kv.delete(key))))
}

async fn kv_has(
    State(state): State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<impl IntoResponse, AppError> {
    let key = req.args.first().and_then(|v| v.as_str()).ok_or((
        StatusCode::BAD_REQUEST,
        "kv.has requires args: [key]".into(),
    ))?;
    Ok(Json(serde_json::json!(state.kv.has(key))))
}

async fn kv_list(
    State(state): State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<impl IntoResponse, AppError> {
    let prefix = req.args.first().and_then(|v| v.as_str());
    let limit = req.args.get(1).and_then(|v| v.as_u64()).unwrap_or(1000) as usize;
    Ok(Json(serde_json::json!(state.kv.list(prefix, limit))))
}

async fn kv_get_many(
    State(state): State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<impl IntoResponse, AppError> {
    let keys: Vec<String> = match req.args.first() {
        Some(v) => serde_json::from_value::<Vec<String>>(v.clone()).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("kv.getMany: invalid keys array: {e}"),
            )
        })?,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                "kv.getMany requires args: [keys[]]".into(),
            ));
        }
    };
    let values = state.kv.get_many(&keys);
    Ok(Json(serde_json::json!({ "values": values })))
}

async fn kv_get_with_metadata(
    State(state): State<AppState>,
    Json(req): Json<KvRequest>,
) -> Result<impl IntoResponse, AppError> {
    let key = req.args.first().and_then(|v| v.as_str()).ok_or((
        StatusCode::BAD_REQUEST,
        "kv.getWithMetadata requires args: [key]".into(),
    ))?;
    let (value, metadata) = state.kv.get_with_metadata(key);
    Ok(Json(
        serde_json::json!({ "value": value, "metadata": metadata }),
    ))
}

// --- DB helpers ---

fn bind_params(
    stmt: &mut rusqlite::Statement,
    bindings: &[serde_json::Value],
) -> Result<(), AppError> {
    for (i, val) in bindings.iter().enumerate() {
        let idx = i + 1;
        let result = match val {
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
        result.map_err(|e| (StatusCode::BAD_REQUEST, format!("bind error at {idx}: {e}")))?;
    }
    Ok(())
}

fn rows_to_json(
    stmt: &mut rusqlite::Statement,
) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, String> {
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();

    let mut raw_rows = stmt.raw_query();
    while let Some(row) = raw_rows.next().map_err(|e| e.to_string())? {
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
                    serde_json::Value::String(base64_encode(b))
                }
                Err(e) => serde_json::Value::String(format!("<error: {e}>")),
            };
            obj.insert(name.clone(), val);
        }
        rows.push(obj);
    }
    Ok(rows)
}

fn rows_to_arrays(
    stmt: &mut rusqlite::Statement,
    include_columns: bool,
) -> Result<(Vec<String>, Vec<Vec<serde_json::Value>>), String> {
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();

    if include_columns {
        rows.push(col_names.iter().map(|n| serde_json::json!(n)).collect());
    }

    let mut raw_rows = stmt.raw_query();
    while let Some(row) = raw_rows.next().map_err(|e| e.to_string())? {
        let mut arr = Vec::new();
        for i in 0..col_names.len() {
            let val: serde_json::Value = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => serde_json::Value::Null,
                Ok(rusqlite::types::ValueRef::Integer(n)) => serde_json::json!(n),
                Ok(rusqlite::types::ValueRef::Real(f)) => serde_json::json!(f),
                Ok(rusqlite::types::ValueRef::Text(s)) => {
                    serde_json::Value::String(String::from_utf8_lossy(s).into_owned())
                }
                Ok(rusqlite::types::ValueRef::Blob(b)) => {
                    serde_json::Value::String(base64_encode(b))
                }
                Err(e) => serde_json::Value::String(format!("<error: {e}>")),
            };
            arr.push(val);
        }
        rows.push(arr);
    }
    Ok((col_names, rows))
}

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        let _ = write!(result, "{}", CHARS[(n >> 18 & 63) as usize] as char);
        let _ = write!(result, "{}", CHARS[(n >> 12 & 63) as usize] as char);
        if chunk.len() > 1 {
            let _ = write!(result, "{}", CHARS[(n >> 6 & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            let _ = write!(result, "{}", CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn execute_query(
    conn: &Connection,
    sql: &str,
    bindings: &[serde_json::Value],
    mode: &str,
    column: Option<&str>,
    column_names: Option<bool>,
) -> Result<QueryResponse, AppError> {
    let start = Instant::now();
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("SQL error: {e}")))?;

    bind_params(&mut stmt, bindings)?;

    let results = match mode {
        "all" => {
            let rows =
                rows_to_json(&mut stmt).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            serde_json::json!(rows)
        }
        "first" => {
            let rows =
                rows_to_json(&mut stmt).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            match rows.into_iter().next() {
                Some(row) => {
                    if let Some(col) = column {
                        row.get(col).cloned().unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::json!(row)
                    }
                }
                None => serde_json::Value::Null,
            }
        }
        "run" => {
            // For run mode, execute and return meta only
            let _count = stmt
                .raw_execute()
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("SQL error: {e}")))?;
            serde_json::json!([])
        }
        "raw" => {
            let include_cols = column_names.unwrap_or(false);
            let (_cols, rows) = rows_to_arrays(&mut stmt, include_cols)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            serde_json::json!(rows)
        }
        _ => {
            return Err((StatusCode::BAD_REQUEST, format!("unknown mode: {mode}")));
        }
    };

    let duration = start.elapsed().as_secs_f64();
    let changes = conn.changes() as i64;
    let last_row_id = conn.last_insert_rowid();

    Ok(QueryResponse {
        results,
        success: true,
        meta: QueryResponseMeta {
            changes,
            last_row_id,
            duration,
        },
    })
}

// --- DB handlers ---

async fn db_query(
    State(state): State<AppState>,
    Json(req): Json<DbQueryRequest>,
) -> Result<impl IntoResponse, AppError> {
    let conn = state.db.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db lock error: {e}"),
        )
    })?;
    let resp = execute_query(
        &conn,
        &req.sql,
        &req.bindings,
        &req.mode,
        req.column.as_deref(),
        req.column_names,
    )?;
    Ok(Json(resp))
}

async fn db_batch(
    State(state): State<AppState>,
    Json(req): Json<DbBatchRequest>,
) -> Result<impl IntoResponse, AppError> {
    let conn = state.db.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db lock error: {e}"),
        )
    })?;
    let mut results = Vec::new();
    for stmt in &req.statements {
        let resp = execute_query(&conn, &stmt.sql, &stmt.bindings, "all", None, None)?;
        results.push(resp);
    }
    Ok(Json(results))
}

async fn db_exec(
    State(state): State<AppState>,
    Json(req): Json<DbExecRequest>,
) -> Result<impl IntoResponse, AppError> {
    let start = Instant::now();
    let conn = state.db.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db lock error: {e}"),
        )
    })?;
    conn.execute_batch(&req.sql)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("SQL error: {e}")))?;
    let duration = start.elapsed().as_secs_f64();

    Ok(Json(QueryResponse {
        results: serde_json::json!([]),
        success: true,
        meta: QueryResponseMeta {
            changes: conn.changes() as i64,
            last_row_id: conn.last_insert_rowid(),
            duration,
        },
    }))
}
