mod manifest;

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
struct BuildOutput {
    adapter: String,
    adapter_version: String,
    framework: String,
    framework_version: String,
    routes: usize,
    output_dir: String,
}

/// Result of build validation — tells deploy whether a manifest was found.
pub struct BuildResult {
    pub output_dir: std::path::PathBuf,
    pub has_manifest: bool,
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

    if has_manifest {
        let manifest_path = output_dir.join(".onreza/manifest.json");
        let manifest = manifest::load_and_validate(&manifest_path)?;

        if !args.skip_validation {
            manifest::verify_files(&output_dir, &manifest)?;
        }

        if json {
            output::json_output(&BuildOutput {
                adapter: manifest.adapter.name.clone(),
                adapter_version: manifest.adapter.version.clone(),
                framework: manifest.framework.name.clone(),
                framework_version: manifest.framework.version.clone(),
                routes: manifest.routes.len(),
                output_dir: output_dir.to_string_lossy().into_owned(),
            });
        } else {
            eprintln!(
                "  {} {} v{} ({} v{})",
                console::style("✓").green().bold(),
                manifest.adapter.name,
                manifest.adapter.version,
                manifest.framework.name,
                manifest.framework.version,
            );
            eprintln!(
                "  {} {} routes, server entry: {}",
                console::style("✓").green().bold(),
                manifest.routes.len(),
                manifest.server.entry,
            );
        }
    } else if json {
        output::json_output(&serde_json::json!({
            "has_manifest": false,
            "output_dir": output_dir.to_string_lossy(),
        }));
    } else {
        output::status(false, "~", "No .onreza/manifest.json found (no adapter)");
    }

    Ok(BuildResult {
        output_dir,
        has_manifest,
    })
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
            vec![".next"]
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
