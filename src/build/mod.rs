mod manifest;

#[cfg(test)]
mod manifest_tests;

use std::path::Path;

use anyhow::Context;

use crate::cli::BuildArgs;

/// Validate build output and manifest.
///
/// 1. Locate output directory (dist/, build/, .output/)
/// 2. Find .onreza/manifest.json
/// 3. Parse and validate against BUILD_OUTPUT_SPEC v1
/// 4. Verify referenced files exist (server entry, assets dir, prerender dir)
/// 5. Report results
pub async fn run(args: BuildArgs) -> anyhow::Result<()> {
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

    // Check if output dir exists but without .onreza
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
