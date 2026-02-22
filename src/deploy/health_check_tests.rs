use std::fs;

use tempfile::tempdir;

use super::health_check::{detect_health_path, extract_path_from_pattern};

#[test]
fn nextjs_app_router_health() {
    let dir = tempdir().unwrap();
    let route_dir = dir.path().join("app/api/health");
    fs::create_dir_all(&route_dir).unwrap();
    fs::write(route_dir.join("route.ts"), "export async function GET() {}").unwrap();

    let result = detect_health_path(dir.path(), "nextjs", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/api/health");
    assert_eq!(det.source_description, "app/api/health/route.ts");
}

#[test]
fn nextjs_app_router_healthz() {
    let dir = tempdir().unwrap();
    let route_dir = dir.path().join("src/app/api/healthz");
    fs::create_dir_all(&route_dir).unwrap();
    fs::write(route_dir.join("route.js"), "export function GET() {}").unwrap();

    let result = detect_health_path(dir.path(), "nextjs", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/api/healthz");
}

#[test]
fn nextjs_pages_router_health() {
    let dir = tempdir().unwrap();
    let pages_dir = dir.path().join("pages/api");
    fs::create_dir_all(&pages_dir).unwrap();
    fs::write(
        pages_dir.join("health.ts"),
        "export default function handler() {}",
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "nextjs", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/api/health");
    assert_eq!(det.source_description, "pages/api/health.ts");
}

#[test]
fn nextjs_no_health_endpoint() {
    let dir = tempdir().unwrap();
    let pages_dir = dir.path().join("pages/api");
    fs::create_dir_all(&pages_dir).unwrap();
    fs::write(
        pages_dir.join("users.ts"),
        "export default function handler() {}",
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "nextjs", dir.path());
    assert!(result.is_none());
}

#[test]
fn express_get_health() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("server.ts"),
        r#"
import express from 'express';
const app = express();
app.get("/health", (req, res) => res.json({ ok: true }));
app.listen(3000);
"#,
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "other", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/health");
    assert_eq!(det.source_description, "server.ts");
}

#[test]
fn hono_get_healthz_single_quotes() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/index.ts"),
        r#"
import { Hono } from 'hono';
const app = new Hono();
app.get('/healthz', (c) => c.json({ ok: true }));
export default app;
"#,
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "other", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/healthz");
    assert_eq!(det.source_description, "src/index.ts");
}

#[test]
fn health_file_in_routes_dir() {
    let dir = tempdir().unwrap();
    let routes_dir = dir.path().join("src/routes");
    fs::create_dir_all(&routes_dir).unwrap();
    fs::write(routes_dir.join("health.ts"), "export default {}").unwrap();

    let result = detect_health_path(dir.path(), "other", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/health");
    assert_eq!(det.source_description, "src/routes/health.ts");
}

#[test]
fn nestjs_terminus_dep() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"@nestjs/core": "^10.0.0", "@nestjs/terminus": "^10.0.0"}}"#,
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "other", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/health");
    assert!(det.source_description.contains("@nestjs/terminus"));
}

#[test]
fn nestjs_get_decorator() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies": {"@nestjs/core": "^10.0.0"}}"#,
    )
    .unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("health.controller.ts"),
        r#"
import { Controller, Get } from '@nestjs/common';

@Controller()
export class HealthController {
    @Get("/health")
    check() { return { status: 'ok' }; }
}
"#,
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "other", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/health");
}

#[test]
fn no_health_endpoint_at_all() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("server.ts"),
        r#"
import express from 'express';
const app = express();
app.get("/users", (req, res) => res.json([]));
"#,
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "other", dir.path());
    assert!(result.is_none());
}

#[test]
fn extract_path_from_double_quote_pattern() {
    let path = extract_path_from_pattern(r#".get("/health""#);
    assert_eq!(path, Some("/health".to_string()));
}

#[test]
fn extract_path_from_single_quote_pattern() {
    let path = extract_path_from_pattern(".get('/healthz'");
    assert_eq!(path, Some("/healthz".to_string()));
}

#[test]
fn extract_path_from_spaced_pattern() {
    let path = extract_path_from_pattern(r#".get( "/health""#);
    assert_eq!(path, Some("/health".to_string()));

    let path = extract_path_from_pattern(".get( '/ping'");
    assert_eq!(path, Some("/ping".to_string()));
}

#[test]
fn nextjs_app_router_ping() {
    let dir = tempdir().unwrap();
    let route_dir = dir.path().join("app/api/ping");
    fs::create_dir_all(&route_dir).unwrap();
    fs::write(route_dir.join("route.ts"), "export async function GET() {}").unwrap();

    let result = detect_health_path(dir.path(), "nextjs", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/api/ping");
    assert_eq!(det.source_description, "app/api/ping/route.ts");
}

#[test]
fn express_get_ping() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("server.js"),
        r#"
const app = require('express')();
app.get('/ping', (req, res) => res.send('pong'));
app.listen(3000);
"#,
    )
    .unwrap();

    let result = detect_health_path(dir.path(), "other", dir.path());
    assert!(result.is_some());
    let det = result.unwrap();
    assert_eq!(det.path, "/ping");
    assert_eq!(det.source_description, "server.js");
}
