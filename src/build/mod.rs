mod manifest;

#[cfg(test)]
mod manifest_tests;

use std::path::Path;

use anyhow::Context;
use serde::Serialize;

use crate::cli::BuildArgs;
use crate::output;

#[derive(Serialize)]
struct BuildOutput {
    adapter: String,
    adapter_version: String,
    framework: String,
    framework_version: String,
    routes: usize,
    output_dir: String,
}

pub async fn run(args: BuildArgs, json: bool) -> anyhow::Result<()> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    let output_dir = detect_output_dir(&project_dir)?;
    tracing::info!(?output_dir, "found output directory");

    let manifest_path = output_dir.join(".onreza/manifest.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "manifest not found at {}. Did the adapter run during build?",
            manifest_path.display()
        );
    }

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

    Ok(())
}

/// Try common output directory names.
fn detect_output_dir(project_dir: &Path) -> anyhow::Result<std::path::PathBuf> {
    for name in ["dist", ".output", "build"] {
        let candidate = project_dir.join(name);
        if candidate.is_dir() && candidate.join(".onreza").is_dir() {
            return Ok(candidate);
        }
    }

    for name in ["dist", ".output", "build"] {
        let candidate = project_dir.join(name);
        if candidate.is_dir() {
            anyhow::bail!(
                "found '{}' but it doesn't contain .onreza/. \
                 Make sure you're using an @onreza/* adapter in your framework config.",
                candidate.display()
            );
        }
    }

    anyhow::bail!(
        "no output directory found in {}. Expected dist/, .output/, or build/",
        project_dir.display()
    );
}
