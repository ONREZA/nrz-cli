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
    run_with_hint(args, json, config, None).await
}

pub async fn run_with_hint(
    args: BuildArgs,
    json: bool,
    config: &ProjectConfig,
    detection: Option<&crate::detect::types::DetectionResult>,
) -> anyhow::Result<BuildResult> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    let internal_detection;
    let detection = match detection {
        Some(d) => d,
        None => {
            internal_detection = crate::detect::detect(&project_dir);
            &internal_detection
        }
    };
    let fw_dirs = compute_aware_output_dirs(detection);
    let (output_dir, has_manifest) =
        detect_output_dir(&project_dir, &config.output_dirs(), &fw_dirs)?;
    tracing::info!(?output_dir, has_manifest, "found output directory");

    let loaded_manifest = if has_manifest {
        let manifest_path = output_dir.join(".onreza/manifest.json");
        let manifest = manifest::load_and_validate(&manifest_path)?;

        if !args.skip_validation {
            manifest::verify_files(&output_dir, &manifest)?;
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
            output::json_output(&BuildOutput {
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
            });
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
        && detection
            .metadata
            .ssr_analysis
            .as_ref()
            .is_some_and(|ssr| ssr.has_standalone_output())
    {
        if !output_dir.join("server.js").is_file() {
            anyhow::bail!(
                "server.js not found in standalone output {}. \
                 Ensure `output: 'standalone'` is set in next.config.js \
                 and `next build` completed successfully.",
                output_dir.display()
            );
        }
        prepare_nextjs_standalone(&project_dir, &output_dir, json)?;
        let has_public = output_dir.join("public").is_dir();
        let auto = manifest::generate_nextjs_standalone_manifest(has_public);
        output::status(
            json,
            "~",
            "Auto-generated Next.js standalone manifest (STATIC + COMPUTE)",
        );
        if !args.skip_validation {
            manifest::verify_files(&output_dir, &auto)?;
        }
        emit_build_output(json, &auto, &output_dir, Some(detection));
        Some(auto)
    } else if detection.suggested_compute == crate::detect::types::ComputeType::Static {
        let auto = manifest::generate_static_manifest();
        output::status(
            json,
            "~",
            "Auto-generated STATIC manifest (no adapter found)",
        );
        emit_build_output(json, &auto, &output_dir, Some(detection));
        Some(auto)
    } else {
        if !json {
            output::status(false, "~", "No .onreza/manifest.json found (no adapter)");
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
        output::json_output(&BuildOutput {
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
        });
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
        "nextjs" => {
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
        slug => crate::detect::presets::framework_output_dirs(slug).to_vec(),
    }
}

/// Try framework-specific and configured output directory names.
/// Returns `(path, has_manifest)` — first prefers dirs with `.onreza/`, then any existing dir.
///
/// `framework_dirs` are checked first (more specific), then `config_dirs` (defaults).
fn detect_output_dir(
    project_dir: &Path,
    config_dirs: &[&str],
    framework_dirs: &[&str],
) -> anyhow::Result<(std::path::PathBuf, bool)> {
    // Merge: framework-specific dirs first, then config defaults (dedup preserving order)
    let mut seen = std::collections::HashSet::new();
    let all_dirs: Vec<&str> = framework_dirs
        .iter()
        .chain(config_dirs.iter())
        .copied()
        .filter(|d| seen.insert(*d))
        .collect();

    // Phase 1: prefer dir with .onreza/ (adapter-generated manifest)
    for name in &all_dirs {
        let candidate = project_dir.join(name);
        if candidate.is_dir() && candidate.join(".onreza").is_dir() {
            return Ok((candidate, true));
        }
    }

    // Phase 2: any existing output dir (no adapter — static/process deploy)
    for name in &all_dirs {
        let candidate = project_dir.join(name);
        if candidate.is_dir() {
            return Ok((candidate, false));
        }
    }

    let dirs_display: Vec<_> = all_dirs.iter().map(|d| format!("{d}/")).collect();
    anyhow::bail!(
        "no output directory found in {}. Expected one of: {}",
        project_dir.display(),
        dirs_display.join(", ")
    );
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
        );
    } else {
        // 1. Copy .next/static/ → {output}/.next/static/ (for server.js)
        let server_static_dst = output_dir.join(".next/static");
        if !server_static_dst.is_dir() {
            output::status(json, "+", "Copying .next/static/ for server.js");
            copy_dir_recursive(&next_static_src, &server_static_dst)?;
        } else {
            output::status(json, "~", ".next/static/ already present, skipping copy");
        }

        // 2. Copy .next/static/ → {output}/_static/_next/static/ (for CDN STATIC layer)
        let cdn_static_dst = output_dir.join("_static/_next/static");
        if !cdn_static_dst.is_dir() {
            output::status(json, "+", "Copying static assets to _static/ for CDN");
            copy_dir_recursive(&next_static_src, &cdn_static_dst)?;
        } else {
            output::status(json, "~", "_static/ already present, skipping copy");
        }
    }

    // 3. Copy public/ → {output}/public/ (STATIC layer for root-level assets)
    let public_src = project_dir.join("public");
    if public_src.is_dir() {
        let public_dst = output_dir.join("public");
        if !public_dst.is_dir() {
            output::status(json, "+", "Copying public/ assets");
            copy_dir_recursive(&public_src, &public_dst)?;
        } else {
            output::status(json, "~", "public/ already present, skipping copy");
        }
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
