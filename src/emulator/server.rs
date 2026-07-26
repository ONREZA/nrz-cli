use std::net::SocketAddr;

use axum::extract::State;
use axum::http::header::{HOST, ORIGIN};
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use super::kv::KvStore;

/// Local HTTP server for the emulator.
///
/// Runs alongside the framework dev server. The JS bootstrap
/// (injected into Node.js) proxies ONREZA.kv calls to this server.
pub struct EmulatorServer {
    pub kv: KvStore,
    pub addr: SocketAddr,
    token: String,
}

#[derive(Clone)]
struct AppState {
    kv: KvStore,
    token: String,
    expected_host: String,
}

pub const EMULATOR_TOKEN_HEADER: &str = "x-nrz-emulator-token";

// --- Request types ---

#[derive(Deserialize)]
struct KvRequest {
    args: Vec<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
}

type AppError = (StatusCode, String);

impl EmulatorServer {
    pub fn new(kv: KvStore, port: u16, host: &str) -> anyhow::Result<Self> {
        let ip: std::net::IpAddr = host
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid host '{}': {}", host, e))?;
        Ok(Self {
            kv,
            addr: SocketAddr::new(ip, port),
            token: uuid::Uuid::now_v7().to_string(),
        })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn client_url(&self) -> String {
        let ip = match self.addr.ip() {
            std::net::IpAddr::V4(ip) if ip.is_unspecified() => std::net::Ipv4Addr::LOCALHOST.into(),
            std::net::IpAddr::V6(ip) if ip.is_unspecified() => std::net::Ipv6Addr::LOCALHOST.into(),
            ip => ip,
        };
        format!("http://{}", SocketAddr::new(ip, self.addr.port()))
    }

    /// Start the emulator HTTP server.
    pub async fn start(&self) -> anyhow::Result<()> {
        let expected_host = self
            .client_url()
            .strip_prefix("http://")
            .expect("emulator client URL is HTTP")
            .to_string();
        let state = AppState {
            kv: self.kv.clone(),
            token: self.token.clone(),
            expected_host,
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
            .layer(middleware::from_fn_with_state(state.clone(), require_token))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        tracing::info!(addr = %self.addr, "emulator server listening");

        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn require_token(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();
    let authorized = headers
        .get(EMULATOR_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == state.token);
    let expected_host = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == state.expected_host);
    if !authorized || !expected_host || headers.contains_key(ORIGIN) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(request).await)
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
