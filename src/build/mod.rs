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

/// Result of build validation — carries the parsed manifest so deploy avoids re-reading it.
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
    } else if detection.suggested_compute == crate::detect::types::ComputeType::Static {
        let auto = manifest::generate_static_manifest();
        output::status(
            json,
            "~",
            "Auto-generated STATIC manifest (no adapter found)",
        );
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
