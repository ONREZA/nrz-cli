pub(crate) mod manifest;

#[cfg(test)]
mod manifest_tests;

#[cfg(test)]
mod build_tests;

use std::path::Path;

use anyhow::Context;
use serde::Serialize;

use crate::cli::BuildArgs;
use crate::output;
use nrz::config::ProjectConfig;

#[derive(Serialize)]
struct LayerInfo {
    name: String,
    target: String,
    directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<String>,
}

#[derive(Serialize)]
struct BuildOutput {
    layers: Vec<LayerInfo>,
    routes: usize,
    output_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    framework: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    framework_version: Option<String>,
}

/// Result of the build step — carries the output directory and parsed manifest so deploy avoids re-reading them.
#[derive(Debug)]
pub struct BuildResult {
    pub output_dir: std::path::PathBuf,
    pub manifest: Option<manifest::Manifest>,
}

pub async fn run(
    args: BuildArgs,
    json: bool,
    config: &ProjectConfig,
) -> anyhow::Result<BuildResult> {
    run_with_hint(args, json, config, None, None).await
}

pub async fn run_with_hint(
    args: BuildArgs,
    json: bool,
    config: &ProjectConfig,
    detection: Option<&crate::detect::types::DetectionResult>,
    server_output_dir: Option<&str>,
) -> anyhow::Result<BuildResult> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    let internal_detection;
    let detection = match detection {
        Some(d) => d,
        None => {
            internal_detection = crate::detect::detect_with_framework_override(
                &project_dir,
                config.project.framework.as_deref(),
            );
            &internal_detection
        }
    };
    let fw_dirs = compute_aware_output_dirs(detection);
    let (output_dir, has_manifest) = detect_output_dir(
        &project_dir,
        &config.output_dirs(),
        &fw_dirs,
        server_output_dir,
    )?;
    tracing::info!(?output_dir, has_manifest, "found output directory");

    let loaded_manifest = if has_manifest {
        let manifest_path = output_dir.join(".onreza/manifest.json");
        let manifest = manifest::load_and_validate(&manifest_path)
            .map_err(|e| output::with_default_code(e, "INVALID_MANIFEST"))?;

        if !args.skip_validation {
            manifest::verify_files(&output_dir, &manifest)
                .map_err(|e| output::with_default_code(e, "INVALID_MANIFEST"))?;
        }

        let framework = manifest
            .meta
            .as_ref()
            .and_then(|m| m.get("framework"))
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from);
        let framework_version = manifest
            .meta
            .as_ref()
            .and_then(|m| m.get("framework"))
            .and_then(|f| f.get("version"))
            .and_then(|v| v.as_str())
            .map(String::from);

        if json {
            let data = BuildOutput {
                layers: manifest
                    .layers
                    .iter()
                    .map(|l| LayerInfo {
                        name: l.name.clone(),
                        target: l.target.to_string(),
                        directory: l.directory.clone(),
                        entry: l.entry.clone(),
                    })
                    .collect(),
                routes: manifest.routes.len(),
                output_dir: output_dir.to_string_lossy().into_owned(),
                framework,
                framework_version,
            };
            if let Ok(s) = serde_json::to_string(&data) {
                output::log_line("debug", "info", "build", &s);
            }
        } else {
            let layers_display: Vec<String> = manifest
                .layers
                .iter()
                .map(|l| match &l.entry {
                    Some(e) => format!("{}({}:{})", l.target, l.directory, e),
                    None => format!("{}({})", l.target, l.directory),
                })
                .collect();
            eprintln!(
                "  {} {} layer(s): {}",
                console::style("✓").green().bold(),
                manifest.layers.len(),
                layers_display.join(", "),
            );
            eprintln!(
                "  {} {} route(s)",
                console::style("✓").green().bold(),
                manifest.routes.len(),
            );
        }
        Some(manifest)
    } else if detection.framework == "nextjs"
        && (detection
            .metadata
            .ssr_analysis
            .as_ref()
            .is_some_and(|ssr| ssr.has_standalone_output())
            || output_dir.join("server.js").is_file())
    {
        if !output_dir.join("server.js").is_file() {
            return Err(output::coded_error(
                "MISSING_BUILD_OUTPUT",
                format!(
                    "server.js not found in standalone output {}. \
                     Ensure `output: 'standalone'` is set in next.config.js \
                     and `next build` completed successfully.",
                    output_dir.display()
                ),
            ));
        }
        prepare_nextjs_standalone(&project_dir, &output_dir, json)?;
        let has_public = output_dir.join("public").is_dir();
        let auto = manifest::generate_nextjs_standalone_manifest(has_public);
        output::status(
            json,
            "~",
            "Auto-generated Next.js standalone manifest (STATIC + COMPUTE)",
            output::Phase::Build,
        );
        if !args.skip_validation {
            manifest::verify_files(&output_dir, &auto)
                .map_err(|e| output::with_default_code(e, "MISSING_BUILD_OUTPUT"))?;
        }
        emit_build_output(json, &auto, &output_dir, Some(detection));
        Some(auto)
    } else if let Some(auto) = try_generate_ssr_manifest(detection, &output_dir) {
        output::status(
            json,
            "~",
            format!(
                "Auto-generated {} SSR manifest (STATIC + COMPUTE)",
                detection.name
            ),
            output::Phase::Build,
        );
        if !args.skip_validation {
            manifest::verify_files(&output_dir, &auto)
                .map_err(|e| output::with_default_code(e, "MISSING_BUILD_OUTPUT"))?;
        }
        emit_build_output(json, &auto, &output_dir, Some(detection));
        Some(auto)
    } else if detection.suggested_compute == crate::detect::types::ComputeType::Static {
        let auto = manifest::generate_static_manifest();
        output::status(
            json,
            "~",
            "Auto-generated STATIC manifest",
            output::Phase::Build,
        );
        emit_build_output(json, &auto, &output_dir, Some(detection));
        Some(auto)
    } else {
        if !json {
            output::status(
                false,
                "~",
                "No .onreza/manifest.json found",
                output::Phase::Build,
            );
        }
        None
    };

    Ok(BuildResult {
        output_dir,
        manifest: loaded_manifest,
    })
}

fn emit_build_output(
    json: bool,
    manifest: &manifest::Manifest,
    output_dir: &Path,
    detection: Option<&crate::detect::types::DetectionResult>,
) {
    let framework = detection.map(|d| d.framework.clone());
    let framework_version = detection.and_then(|d| d.version.clone());

    if json {
        let data = BuildOutput {
            layers: manifest
                .layers
                .iter()
                .map(|l| LayerInfo {
                    name: l.name.clone(),
                    target: l.target.to_string(),
                    directory: l.directory.clone(),
                    entry: l.entry.clone(),
                })
                .collect(),
            routes: manifest.routes.len(),
            output_dir: output_dir.to_string_lossy().into_owned(),
            framework,
            framework_version,
        };
        if let Ok(s) = serde_json::to_string(&data) {
            output::log_line("debug", "info", "build", &s);
        }
    } else {
        let layers_display: Vec<String> = manifest
            .layers
            .iter()
            .map(|l| match &l.entry {
                Some(e) => format!("{}({}:{})", l.target, l.directory, e),
                None => format!("{}({})", l.target, l.directory),
            })
            .collect();
        eprintln!(
            "  {} {} layer(s): {}",
            console::style("✓").green().bold(),
            manifest.layers.len(),
            layers_display.join(", "),
        );
        eprintln!(
            "  {} {} route(s)",
            console::style("✓").green().bold(),
            manifest.routes.len(),
        );
    }
}

/// Use SSR analysis from detection to refine the output directory list.
///
/// For Next.js, the correct output dir depends on the mode:
/// - `output: 'export'` → `out/` (static HTML)
/// - `output: 'standalone'` → `.next/standalone/` (self-contained server)
/// - default SSR → `.next/` (requires `next start`)
fn compute_aware_output_dirs(
    detection: &crate::detect::types::DetectionResult,
) -> Vec<&'static str> {
    match detection.framework.as_str() {
        "nextjs" | "blitzjs" | "payload" => {
            if let Some(ref ssr) = detection.metadata.ssr_analysis {
                if ssr.is_static_compatible {
                    return vec!["out"];
                }
                if ssr.has_standalone_output() {
                    return vec![".next/standalone", ".next"];
                }
            }
            // Be optimistic and try standalone first even when SSR analysis
            // misses `output: 'standalone'` in a complex config file.
            vec![".next/standalone", ".next"]
        }
        "nuxt" => {
            if let Some(ref ssr) = detection.metadata.ssr_analysis
                && ssr.is_static_compatible
            {
                // Static Nuxt: serve from .output/public/ directly
                return vec![".output/public", ".output"];
            }
            vec![".output"]
        }
        "remix" | "react-router" => {
            if let Some(ref ssr) = detection.metadata.ssr_analysis
                && ssr.is_static_compatible
            {
                return vec!["build/client", "build"];
            }
            vec!["build"]
        }
        "hydrogen" => {
            // Oxygen (default) emits dist/*, Express recipe emits build/*.
            // Try dist first — when it exists, the workers-runtime detector fires
            // with a clear error; otherwise we fall through to build/.
            if let Some(ref ssr) = detection.metadata.ssr_analysis
                && ssr.is_static_compatible
            {
                return vec!["dist/client", "build/client", "build"];
            }
            vec!["dist", "build"]
        }
        "tanstack-start" => vec![".output", "dist"],
        slug => crate::detect::presets::framework_output_dirs(slug).to_vec(),
    }
}

/// Try to auto-generate an SSR manifest for known frameworks.
/// Returns `None` if the framework is not SSR, is static-compatible,
/// or the expected build output structure is not found.
/// Expected SSR entry point for a framework, or `None` for non-SSR frameworks.
fn ssr_expected_entry(framework: &str) -> Option<&'static str> {
    match framework {
        "nextjs" | "blitzjs" | "payload" => Some("server.js"),
        "nuxt" => Some("server/index.mjs"),
        "sveltekit" => Some("index.js"),
        "remix" | "react-router" => Some("server/index.js"),
        // Hydrogen: no shared entry — Oxygen bundles to dist/server/index.js as a
        // workers module, Express recipe uses server.mjs at project root.
        "hydrogen" => None,
        "tanstack-start" => Some("server/index.mjs"),
        "astro" => Some("server/entry.mjs"),
        _ => None,
    }
}

fn try_generate_ssr_manifest(
    detection: &crate::detect::types::DetectionResult,
    output_dir: &Path,
) -> Option<manifest::Manifest> {
    let ssr = detection.metadata.ssr_analysis.as_ref()?;
    if ssr.is_static_compatible {
        return None;
    }

    let expected_entry = ssr_expected_entry(&detection.framework)?;

    if !output_dir.join(expected_entry).is_file() {
        tracing::warn!(
            framework = %detection.framework,
            expected = %output_dir.join(expected_entry).display(),
            "SSR entry point not found; cannot auto-generate manifest"
        );
        return None;
    }

    tracing::debug!(
        framework = %detection.framework,
        entry = expected_entry,
        "auto-generating SSR manifest"
    );

    match detection.framework.as_str() {
        "nuxt" => {
            let has_public = output_dir.join("public").is_dir();
            Some(manifest::generate_nuxt_manifest(has_public))
        }
        "sveltekit" => {
            let has_client = output_dir.join("client").is_dir();
            Some(manifest::generate_sveltekit_manifest(has_client))
        }
        "remix" | "react-router" => {
            let has_client = output_dir.join("client").is_dir();
            Some(manifest::generate_remix_manifest(has_client))
        }
        "astro" => {
            let has_client = output_dir.join("client").is_dir();
            Some(manifest::generate_astro_ssr_manifest(has_client))
        }
        _ => None,
    }
}

/// Try framework-specific and configured output directory names.
/// Returns `(path, has_manifest)` — first prefers dirs with `.onreza/`, then any existing dir.
///
/// Priority: `framework_dirs` > `server_output_dir` > `config_dirs`.
fn detect_output_dir(
    project_dir: &Path,
    config_dirs: &[&str],
    framework_dirs: &[&str],
    server_output_dir: Option<&str>,
) -> anyhow::Result<(std::path::PathBuf, bool)> {
    // Log when server-provided directory doesn't exist on disk
    if let Some(sod) = server_output_dir
        && !project_dir.join(sod).is_dir()
    {
        tracing::debug!(
            server_output_dir = sod,
            "server-configured output directory not found on disk, will try other candidates"
        );
    }

    // Merge: framework-specific dirs first, then server output dir, then config defaults (dedup preserving order)
    let mut seen = std::collections::HashSet::new();
    let all_dirs: Vec<&str> = framework_dirs
        .iter()
        .copied()
        .chain(server_output_dir)
        .chain(config_dirs.iter().copied())
        .filter(|d| seen.insert(*d))
        .collect();

    // Phase 1: prefer dir with .onreza/ (pre-existing manifest)
    for name in &all_dirs {
        let candidate = project_dir.join(name);
        if candidate.is_dir() && candidate.join(".onreza").is_dir() {
            return Ok((candidate, true));
        }
    }

    // Phase 2: any existing output dir (static/process deploy)
    for name in &all_dirs {
        let candidate = project_dir.join(name);
        if candidate.is_dir() {
            return Ok((candidate, false));
        }
    }

    let dirs_display: Vec<_> = all_dirs.iter().map(|d| format!("{d}/")).collect();
    Err(output::coded_error(
        "MISSING_BUILD_OUTPUT",
        format!(
            "no output directory found in {}. Expected one of: {}",
            project_dir.display(),
            dirs_display.join(", ")
        ),
    ))
}

/// Prepare Next.js standalone output by copying static assets and public files
/// into the correct directory structure for STATIC + COMPUTE layers.
///
/// Safe to call after a partial run: skips copy steps when the destination directory already exists.
fn prepare_nextjs_standalone(
    project_dir: &Path,
    output_dir: &Path,
    json: bool,
) -> anyhow::Result<()> {
    let next_static_src = project_dir.join(".next/static");

    if !next_static_src.is_dir() {
        output::status(
            json,
            "~",
            "No .next/static/ found — standalone output will have no static assets",
            output::Phase::Build,
        );
    } else {
        // 1. Copy .next/static/ → {output}/.next/static/ (for server.js)
        let server_static_dst = output_dir.join(".next/static");
        if !server_static_dst.is_dir() {
            output::status(
                json,
                "+",
                "Copying .next/static/ for server.js",
                output::Phase::Build,
            );
            copy_dir_recursive(&next_static_src, &server_static_dst)?;
        } else {
            output::status(
                json,
                "~",
                ".next/static/ already present, skipping copy",
                output::Phase::Build,
            );
        }

        // 2. Copy .next/static/ → {output}/_static/_next/static/ (for CDN STATIC layer)
        let cdn_static_dst = output_dir.join("_static/_next/static");
        if !cdn_static_dst.is_dir() {
            output::status(
                json,
                "+",
                "Copying static assets to _static/ for CDN",
                output::Phase::Build,
            );
            copy_dir_recursive(&next_static_src, &cdn_static_dst)?;
        } else {
            output::status(
                json,
                "~",
                "_static/ already present, skipping copy",
                output::Phase::Build,
            );
        }
    }

    // 3. Copy public/ → {output}/public/ (STATIC layer for root-level assets)
    let public_src = project_dir.join("public");
    if public_src.is_dir() {
        let public_dst = output_dir.join("public");
        if !public_dst.is_dir() {
            output::status(json, "+", "Copying public/ assets", output::Phase::Build);
            copy_dir_recursive(&public_src, &public_dst)?;
        } else {
            output::status(
                json,
                "~",
                "public/ already present, skipping copy",
                output::Phase::Build,
            );
        }
    }

    // 4. Copy metadata route .body files → {output}/public/<name>
    //
    // Next.js App Router compiles static metadata files (favicon.ico, robots.txt,
    // opengraph-image.png, etc.) into .next/server/app/**/*.body. In standalone mode,
    // server.js may return 404 for these routes because it expects a reverse proxy to
    // serve them. Copying the .body files into the STATIC public/ layer ensures they
    // are served from CDN without hitting the COMPUTE layer.
    copy_metadata_routes(output_dir, json)?;

    // 5. Copy missing Prisma external packages into standalone node_modules.
    //
    // Prisma 6+ generates a client into `@prisma/client-<hash>` (a content-addressed
    // package). Next.js standalone file tracing may not include these dynamically-named
    // packages, causing runtime "Cannot find module" errors. We detect any `@prisma/client-*`
    // directories in the project's node_modules that are absent from the standalone output
    // and copy them over.
    copy_missing_prisma_packages(project_dir, output_dir, json)?;

    Ok(())
}

/// Copy compiled Next.js metadata route `.body` files into `public/` for CDN delivery.
///
/// Next.js App Router compiles `app/favicon.ico`, `app/robots.txt`, etc. into
/// `.next/server/app/**/*.body` files. These are served as metadata routes by server.js,
/// but standalone server.js may not handle them reliably. Copying them to `public/`
/// (dropping the `.body` extension) lets the STATIC layer serve them directly.
///
/// Only files whose path maps cleanly to a URL-safe public path are copied.
/// Dynamically-generated metadata routes (those without a `.body` counterpart) are unaffected.
fn copy_metadata_routes(output_dir: &Path, json: bool) -> anyhow::Result<()> {
    let app_server_dir = output_dir.join(".next/server/app");
    if !app_server_dir.is_dir() {
        return Ok(());
    }

    let public_dst = output_dir.join("public");
    let mut copied = 0usize;

    collect_body_files(&app_server_dir, &app_server_dir, &public_dst, &mut copied)?;

    if copied > 0 {
        output::status(
            json,
            "+",
            format!("Copied {copied} metadata route(s) to public/ (favicon.ico, etc.)"),
            output::Phase::Build,
        );
    }

    Ok(())
}

fn collect_body_files(
    base: &Path,
    current: &Path,
    public_dst: &Path,
    copied: &mut usize,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry?;
        let ft = entry.file_type()?;
        let path = entry.path();

        if ft.is_symlink() {
            continue;
        }

        if ft.is_dir() {
            collect_body_files(base, &path, public_dst, copied)?;
        } else if ft.is_file() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Only process .body files
            if !name_str.ends_with(".body") {
                continue;
            }

            // Strip .body to get the public filename (e.g. "favicon.ico.body" → "favicon.ico")
            let public_name = &name_str[..name_str.len() - ".body".len()];

            // Compute relative path from app server dir for nested metadata routes
            // (e.g. .next/server/app/og/opengraph-image.body → og/opengraph-image.png)
            let rel = path
                .strip_prefix(base)
                .context("failed to compute relative path")?
                .with_file_name(public_name);

            let dst = public_dst.join(&rel);

            // Skip if already present (idempotent)
            if dst.exists() {
                continue;
            }

            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }

            std::fs::copy(&path, &dst).with_context(|| {
                format!("failed to copy {} → {}", path.display(), dst.display())
            })?;

            *copied += 1;
        }
    }
    Ok(())
}

/// Copy Prisma generated client packages missing from standalone output.
///
/// Prisma 6+ generates a client into `node_modules/@prisma/client-<hash>`. Next.js standalone
/// file tracing may not include these dynamically-named packages. This function scans the
/// project's `node_modules/@prisma/` for `client-*` directories and copies any that are
/// absent from the standalone output's `node_modules/@prisma/`.
fn copy_missing_prisma_packages(
    project_dir: &Path,
    output_dir: &Path,
    json: bool,
) -> anyhow::Result<()> {
    let src_prisma_dir = project_dir.join("node_modules/@prisma");
    if !src_prisma_dir.is_dir() {
        return Ok(());
    }

    let dst_prisma_dir = output_dir.join("node_modules/@prisma");
    let mut copied = 0usize;

    let entries = std::fs::read_dir(&src_prisma_dir)
        .with_context(|| format!("failed to read {}", src_prisma_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match @prisma/client-<hash> directories (e.g. "client-2c3a283f134fdcb6")
        if !name_str.starts_with("client-") {
            continue;
        }

        let dst_pkg = dst_prisma_dir.join(&name);
        if dst_pkg.exists() {
            continue;
        }

        let src_pkg = entry.path();
        // Resolve symlinks: in pnpm setups the package may be a symlink to the store
        let src_resolved = if src_pkg.is_symlink() {
            match std::fs::canonicalize(&src_pkg) {
                Ok(resolved) => resolved,
                Err(e) => {
                    tracing::warn!(
                        path = %src_pkg.display(),
                        error = %e,
                        "could not resolve Prisma package symlink, skipping"
                    );
                    continue;
                }
            }
        } else {
            src_pkg
        };

        if !src_resolved.is_dir() {
            continue;
        }

        copy_dir_recursive(&src_resolved, &dst_pkg)?;
        copied += 1;
    }

    if copied > 0 {
        output::status(
            json,
            "+",
            format!("Copied {copied} Prisma external package(s) to standalone output"),
            output::Phase::Build,
        );
    }

    Ok(())
}

/// Recursively copy a directory tree. Skips symlinks (consistent with deploy/bundle.rs).
fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)
        .with_context(|| format!("failed to create directory: {}", dst.display()))?;

    for entry in std::fs::read_dir(src)
        .with_context(|| format!("failed to read directory: {}", src.display()))?
    {
        let entry = entry?;
        let ft = entry.file_type()?;

        // Skip symlinks — consistent with deploy/bundle.rs
        if ft.is_symlink() {
            tracing::debug!(path = %entry.path().display(), "skipping symlink in copy");
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ft.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if ft.is_file() {
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "failed to copy {} → {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        } else {
            tracing::warn!(path = %src_path.display(), "skipping non-regular file");
        }
    }

    Ok(())
}
