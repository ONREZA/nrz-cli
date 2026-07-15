//! Health check endpoint detection and resolution for PROCESS deployments.
//!
//! Performs static analysis only — never starts the application.

use std::path::Path;

use nrz::config::{HealthCheckPathConfig, ProjectConfig};

use super::{HealthCheckSource, ResolvedHealthCheck};
use crate::output;

/// Skip files larger than 512 KB to avoid loading bundled output into memory.
const MAX_SCAN_FILE_SIZE: u64 = 512 * 1024;

/// Result of health check endpoint detection.
#[derive(Debug, Clone)]
pub struct HealthCheckDetection {
    /// The detected health check path (e.g. `/health`, `/api/health`).
    pub path: String,
    /// Source file where the endpoint was found (relative to project dir).
    pub source_description: String,
}

/// Well-known health endpoint names to search for.
const HEALTH_NAMES: &[&str] = &["health", "healthz", "ping"];

/// Well-known route patterns that indicate a health endpoint registration.
/// Matches things like `.get("/health"`, `.get('/healthz'`, etc.
const ROUTE_PATTERNS: &[&str] = &[
    ".get(\"/health\"",
    ".get('/health'",
    ".get(\"/healthz\"",
    ".get('/healthz'",
    ".get(\"/ping\"",
    ".get('/ping'",
    ".get( \"/health\"",
    ".get( '/health'",
    ".get( \"/healthz\"",
    ".get( '/healthz'",
    ".get( \"/ping\"",
    ".get( '/ping'",
];

/// NestJS decorator patterns.
const NESTJS_PATTERNS: &[&str] = &[
    "@Get(\"/health\"",
    "@Get('/health'",
    "@Get(\"/healthz\"",
    "@Get('/healthz'",
    "@Get(\"/ping\"",
    "@Get('/ping'",
];

/// Detect a health check endpoint from project source code.
///
/// Uses static analysis only: file structure for Next.js, string pattern
/// matching for Express/Fastify/Hono/NestJS, and generic file-based fallback.
pub fn detect_health_path(
    project_dir: &Path,
    framework: &str,
    _output_dir: &Path,
) -> Option<HealthCheckDetection> {
    match framework {
        "nextjs" => detect_nextjs(project_dir),
        _ => detect_generic_grep(project_dir).or_else(|| detect_nestjs(project_dir)),
    }
}

/// Resolve health check path for PROCESS deployments.
///
/// Priority: CLI flag > config > autodetect > TCP default.
pub(super) fn resolve_health_check(
    cli_flag: Option<&str>,
    config: &ProjectConfig,
    project_dir: &Path,
    detection: &crate::detect::types::DetectionResult,
    output_dir: &Path,
    json: bool,
) -> anyhow::Result<ResolvedHealthCheck> {
    if let Some(flag) = cli_flag {
        if flag.eq_ignore_ascii_case("none")
            || flag.eq_ignore_ascii_case("false")
            || flag.eq_ignore_ascii_case("tcp")
        {
            output::success(
                json,
                "Health check: TCP (from --health-check-path)",
                output::Phase::Deploy,
            );
            return Ok(ResolvedHealthCheck {
                path: None,
                source: HealthCheckSource::Flag,
            });
        }
        validate_health_path(flag, "--health-check-path")?;
        output::success(
            json,
            format!("Health check: HTTP {flag} (from --health-check-path)"),
            output::Phase::Deploy,
        );
        return Ok(ResolvedHealthCheck {
            path: Some(flag.to_string()),
            source: HealthCheckSource::Flag,
        });
    }

    if let Some(health_check) = config.health_check_path() {
        match health_check {
            HealthCheckPathConfig::Tcp => {
                output::success(
                    json,
                    "Health check: TCP (configured)",
                    output::Phase::Deploy,
                );
                return Ok(ResolvedHealthCheck {
                    path: None,
                    source: HealthCheckSource::Config,
                });
            }
            HealthCheckPathConfig::Http(path) => {
                output::success(
                    json,
                    format!("Health check: HTTP {path} (from config)"),
                    output::Phase::Deploy,
                );
                return Ok(ResolvedHealthCheck {
                    path: Some(path.clone()),
                    source: HealthCheckSource::Config,
                });
            }
        }
    }

    if let Some(detected) = detect_health_path(project_dir, &detection.framework, output_dir) {
        output::success(
            json,
            format!(
                "Found health endpoint: {} (source: {})",
                detected.path, detected.source_description
            ),
            output::Phase::Deploy,
        );
        return Ok(ResolvedHealthCheck {
            path: Some(detected.path),
            source: HealthCheckSource::Detected,
        });
    }

    output::status(
        json,
        "ℹ",
        "No health check endpoint detected. Using TCP readiness check.\n    \
         To add HTTP health check, create a /health endpoint or set\n    \
         deploy.health_check_path in onreza.toml",
        output::Phase::Deploy,
    );
    Ok(ResolvedHealthCheck {
        path: None,
        source: HealthCheckSource::Default,
    })
}

pub(super) fn validate_health_path(path: &str, source: &str) -> anyhow::Result<()> {
    if !path.starts_with('/') {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must start with '/', got: \"{path}\""),
        ));
    }
    if path.contains("..") {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must not contain '..', got: \"{path}\""),
        ));
    }
    if path.contains('?') || path.contains('#') {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must not contain query or fragment, got: \"{path}\""),
        ));
    }
    Ok(())
}

// ── Next.js ──────────────────────────────────────────────────

fn detect_nextjs(project_dir: &Path) -> Option<HealthCheckDetection> {
    let exts = ["ts", "js", "tsx", "jsx"];

    // App Router: app/api/<name>/route.{ext}
    for prefix in &["app", "src/app"] {
        for name in HEALTH_NAMES {
            for ext in &exts {
                let rel = format!("{prefix}/api/{name}/route.{ext}");
                if project_dir.join(&rel).is_file() {
                    return Some(HealthCheckDetection {
                        path: format!("/api/{name}"),
                        source_description: rel,
                    });
                }
            }
        }
    }

    // Pages Router: pages/api/<name>.{ext}
    for prefix in &["pages", "src/pages"] {
        for name in HEALTH_NAMES {
            for ext in &exts {
                let rel = format!("{prefix}/api/{name}.{ext}");
                if project_dir.join(&rel).is_file() {
                    return Some(HealthCheckDetection {
                        path: format!("/api/{name}"),
                        source_description: rel,
                    });
                }
            }
        }
    }

    None
}

// ── Generic pattern matching (Express, Fastify, Hono, Koa) ──

fn detect_generic_grep(project_dir: &Path) -> Option<HealthCheckDetection> {
    let candidates = [
        "server.ts",
        "server.js",
        "src/server.ts",
        "src/server.js",
        "src/index.ts",
        "src/index.js",
        "src/app.ts",
        "src/app.js",
        "index.ts",
        "index.js",
        "app.ts",
        "app.js",
        "src/main.ts",
        "src/main.js",
    ];

    for candidate in &candidates {
        let path = project_dir.join(candidate);
        if let Some(content) = read_if_small(&path)
            && let Some(health_path) = find_route_pattern(&content)
        {
            return Some(HealthCheckDetection {
                path: health_path,
                source_description: candidate.to_string(),
            });
        }
    }

    // Check routes/ and api/ directories for health files
    for dir in &["routes", "src/routes", "api", "src/api"] {
        let dir_path = project_dir.join(dir);
        if !dir_path.is_dir() {
            continue;
        }
        for name in HEALTH_NAMES {
            for ext in &["ts", "js"] {
                let file = format!("{dir}/{name}.{ext}");
                if project_dir.join(&file).is_file() {
                    return Some(HealthCheckDetection {
                        path: format!("/{name}"),
                        source_description: file,
                    });
                }
            }
        }
    }

    None
}

/// Search file content for route registration patterns like `.get("/health"`.
fn find_route_pattern(content: &str) -> Option<String> {
    for pattern in ROUTE_PATTERNS {
        if content.contains(pattern) {
            // Extract the path from the pattern
            return extract_path_from_pattern(pattern);
        }
    }
    None
}

/// Extract the health path from a matched pattern string.
pub(crate) fn extract_path_from_pattern(pattern: &str) -> Option<String> {
    // Patterns are like `.get("/health"` — extract between quotes
    let start = pattern.find(['"', '\''])?;
    let quote = pattern.as_bytes()[start] as char;
    let rest = &pattern[start + 1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

// ── NestJS ───────────────────────────────────────────────────

fn detect_nestjs(project_dir: &Path) -> Option<HealthCheckDetection> {
    let pkg_path = project_dir.join("package.json");
    let content = std::fs::read_to_string(pkg_path).ok()?;

    if !content.contains("@nestjs/core") {
        return None;
    }

    // @nestjs/terminus provides a standard health module
    if content.contains("@nestjs/terminus") {
        return Some(HealthCheckDetection {
            path: "/health".to_string(),
            source_description: "package.json (@nestjs/terminus)".to_string(),
        });
    }

    // Grep src/ for @Get('/health') decorator
    let src_dir = project_dir.join("src");
    if src_dir.is_dir() {
        return grep_nestjs_decorators(&src_dir, project_dir);
    }

    None
}

fn grep_nestjs_decorators(dir: &Path, project_dir: &Path) -> Option<HealthCheckDetection> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let path = entry.path();

        if ft.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "node_modules" || name_str.starts_with('.') {
                continue;
            }
            if let Some(det) = grep_nestjs_decorators(&path, project_dir) {
                return Some(det);
            }
        } else if ft.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "ts" | "js") {
                continue;
            }
            if let Some(content) = read_if_small(&path) {
                for pattern in NESTJS_PATTERNS {
                    if content.contains(pattern) {
                        let health_path = extract_path_from_pattern(pattern)?;
                        let rel = path
                            .strip_prefix(project_dir)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .replace('\\', "/");
                        return Some(HealthCheckDetection {
                            path: health_path,
                            source_description: rel,
                        });
                    }
                }
            }
        }
    }
    None
}

/// Read a file only if it exists and is smaller than `MAX_SCAN_FILE_SIZE`.
fn read_if_small(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_SCAN_FILE_SIZE {
        return None;
    }
    std::fs::read_to_string(path).ok()
}
