use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Response, StatusCode};
use axum::routing::get;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use url::Url;

use super::process::RuntimeProcess;
use super::{CachedRuntime, RuntimeResolver};

#[derive(Clone)]
struct ReleaseFixture {
    manifest: Arc<Vec<u8>>,
    signature: Arc<Vec<u8>>,
    artifact_name: String,
    artifact: Arc<Vec<u8>>,
}

#[tokio::test]
async fn installs_and_reuses_a_verified_runtime_artifact() {
    let target = current_target();
    let artifact_name = runtime_file_name(target);
    let artifact = b"verified-functions-runtime".to_vec();
    let artifact_sha256 = sha256_hex(&artifact);
    let source_revision = "1111111111111111111111111111111111111111";
    let runtime_release_id = format!("runtime-{source_revision}");
    let manifest = serde_json::to_vec(&json!({
        "schemaVersion": 1,
        "runtimeReleaseId": runtime_release_id,
        "protocolVersion": "onreza-functions-poc/v1",
        "source": { "revision": source_revision },
        "artifacts": [{
            "target": target,
            "fileName": artifact_name,
            "sha256": artifact_sha256,
            "sizeBytes": artifact.len(),
        }],
    }))
    .unwrap();
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    let signature = signing_key.sign(&manifest).to_bytes().to_vec();
    let fixture = ReleaseFixture {
        manifest: Arc::new(manifest.clone()),
        signature: Arc::new(signature),
        artifact_name: artifact_name.clone(),
        artifact: Arc::new(artifact.clone()),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(
        axum::serve(
            listener,
            Router::new()
                .route("/{path}", get(release_file))
                .with_state(fixture),
        )
        .into_future(),
    );
    let cache = TempDir::new_in("/var/tmp").unwrap();
    let base = Url::parse(&format!("http://{address}/")).unwrap();
    let resolver = RuntimeResolver::for_test(
        &runtime_release_id,
        base.join("manifest.json").unwrap(),
        sha256_hex(&manifest),
        base.join("manifest.sig").unwrap(),
        signing_key.verifying_key(),
        cache.path().to_path_buf(),
    )
    .unwrap();
    let concurrent_resolver = RuntimeResolver::for_test(
        &runtime_release_id,
        base.join("manifest.json").unwrap(),
        sha256_hex(&manifest),
        base.join("manifest.sig").unwrap(),
        signing_key.verifying_key(),
        cache.path().to_path_buf(),
    )
    .unwrap();

    let (installed, concurrent) = tokio::join!(resolver.resolve(), concurrent_resolver.resolve());
    let installed = installed.unwrap();
    let concurrent = concurrent.unwrap();
    assert_eq!(tokio::fs::read(&installed.path).await.unwrap(), artifact);
    assert_eq!(concurrent.path, installed.path);
    assert_eq!(installed.runtime_release_id, runtime_release_id);
    assert_eq!(installed.target, target);

    server.abort();
    let cached = resolver.resolve().await.unwrap();
    assert_eq!(cached.path, installed.path);
    assert!(resolver.status().await.unwrap().installed);
}

#[tokio::test]
async fn drives_the_runtime_json_lines_process_contract() {
    let directory = TempDir::new_in("/var/tmp").unwrap();
    let source = directory.path().join("runtime-helper.rs");
    let executable = directory.path().join(if cfg!(windows) {
        "runtime-helper.exe"
    } else {
        "runtime-helper"
    });
    std::fs::write(
        &source,
        r#"
use std::io::{self, BufRead};

fn main() {
    let release = std::env::var("ONREZA_FUNCTIONS_RUNTIME_RELEASE_ID").unwrap();
    println!("{{\"type\":\"ready\",\"protocolVersion\":\"onreza-functions-poc/v1\",\"runtimeReleaseId\":\"{}\"}}", release);
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        if line.contains("\"type\":\"shutdown\"") { break; }
    }
}
"#,
    )
    .unwrap();
    let status = std::process::Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .status()
        .unwrap();
    assert!(status.success());
    let cached = CachedRuntime {
        runtime_release_id: "runtime-1111111111111111111111111111111111111111".to_string(),
        target: current_target().to_string(),
        path: executable,
    };

    let process = RuntimeProcess::start(&cached, directory.path(), "function.ts")
        .await
        .unwrap();
    process.shutdown().await.unwrap();
}

async fn release_file(
    State(fixture): State<ReleaseFixture>,
    Path(path): Path<String>,
) -> Response<Body> {
    let bytes = match path.as_str() {
        "manifest.json" => fixture.manifest.as_ref().clone(),
        "manifest.sig" => fixture.signature.as_ref().clone(),
        value if value == fixture.artifact_name => fixture.artifact.as_ref().clone(),
        _ => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap();
        }
    };
    Response::new(Body::from(bytes))
}

fn current_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "x86_64") => "darwin-x64",
        ("macos", "aarch64") => "darwin-arm64",
        ("windows", "x86_64") => "windows-x64",
        pair => panic!("unsupported test target: {pair:?}"),
    }
}

fn runtime_file_name(target: &str) -> String {
    let suffix = if target == "windows-x64" { ".exe" } else { "" };
    format!("onreza-functions-runtime-{target}{suffix}")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
