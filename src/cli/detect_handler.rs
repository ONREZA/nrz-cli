//! Handler for `nrz detect`.

use std::io::Read as _;
use std::path::Path;

use anyhow::Context;

use crate::detect;
use crate::output;

use super::detect::DetectArgs;

pub fn run(args: DetectArgs, json: bool) -> anyhow::Result<()> {
    // --needed-files: output files the server should include in the manifest
    if args.needed_files {
        let files = detect::fs::DETECTION_CONTENT_FILES;
        if json {
            output::json_output(&serde_json::json!({ "files": files }));
        } else {
            for f in files {
                eprintln!("  {f}");
            }
        }
        return Ok(());
    }

    // --stdin: read JSON manifest from stdin, detect via VirtualFs
    if args.stdin {
        if args.save {
            anyhow::bail!("--stdin and --save cannot be used together");
        }

        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .context("failed to read stdin")?;

        let vfs =
            detect::fs::VirtualFs::from_json(&input).context("failed to parse stdin manifest")?;

        let result = detect::detect_with_fs(&vfs);
        return output_result(&result, &args, json);
    }

    // Local detection (default)
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("directory not found: {}", args.dir))?;

    let result = detect::detect(&project_dir);

    // --save: persist framework to onreza.toml
    if args.save && result.framework != "other" {
        nrz::config::save_framework(&project_dir, &result.framework)?;
        if !json {
            output::status(
                false,
                "+",
                format!("Saved framework \"{}\" to onreza.toml", result.framework),
                output::Phase::Detect,
            );
        }
    }

    output_result(&result, &args, json)
}

fn output_result(
    result: &detect::types::DetectionResult,
    args: &DetectArgs,
    json: bool,
) -> anyhow::Result<()> {
    if args.slug_only {
        if json {
            output::json_output(&serde_json::json!({ "framework": result.framework }));
        } else {
            println!("{}", result.framework);
        }
        return Ok(());
    }

    if json {
        output::json_output(result);
    } else {
        print_human(result);
    }

    Ok(())
}

fn print_human(result: &detect::types::DetectionResult) {
    eprintln!();
    eprintln!(
        "  {} {} ({})",
        console::style("Framework:").bold(),
        console::style(&result.name).cyan().bold(),
        &result.framework,
    );

    if let Some(ref ver) = result.version {
        eprintln!("  {} {ver}", console::style("Version:").bold());
    }

    eprintln!(
        "  {} {}",
        console::style("Compute:").bold(),
        result.suggested_compute,
    );

    eprintln!(
        "  {} {}",
        console::style("Runtime:").bold(),
        format!("{:?}", result.metadata.runtime.runtime_type).to_lowercase(),
    );

    if let Some(ref pm) = result.metadata.package_manager {
        let ver = pm
            .version
            .as_deref()
            .map(|v| format!(" ({v})"))
            .unwrap_or_default();
        eprintln!(
            "  {} {}{}",
            console::style("Package manager:").bold(),
            pm.pm_type,
            ver,
        );
    }

    if let Some(ref build) = result.metadata.build_info {
        if let Some(ref cmd) = build.build_command {
            eprintln!("  {} {cmd}", console::style("Build command:").bold());
        }
        if let Some(ref dir) = build.output_dir {
            eprintln!("  {} {dir}", console::style("Output dir:").bold());
        }
    }

    if let Some(ref ts) = result.metadata.uses_typescript
        && *ts
    {
        eprintln!("  {} yes", console::style("TypeScript:").bold());
    }

    if let Some(ref mono) = result.metadata.monorepo {
        eprintln!(
            "  {} yes — {} ({} workspace patterns, {} packages)",
            console::style("Monorepo:").bold(),
            mono.tool,
            mono.workspaces.len(),
            mono.packages.len(),
        );
        if !mono.packages.is_empty() {
            for pkg in &mono.packages {
                let label = pkg
                    .name
                    .as_deref()
                    .map(|n| format!("{n} ({p})", p = pkg.path))
                    .unwrap_or_else(|| pkg.path.clone());
                eprintln!("    {} {label}", console::style("·").dim());
            }
        }
    }

    if !result.metadata.config_files.is_empty() {
        eprintln!(
            "  {} {}",
            console::style("Config files:").bold(),
            result.metadata.config_files.join(", "),
        );
    }

    if let Some(ref ssr) = result.metadata.ssr_analysis
        && ssr.has_ssr_features()
    {
        eprintln!(
            "  {} {}",
            console::style("SSR features:").bold(),
            ssr.ssr_features.join(", "),
        );
        if ssr.is_static_compatible {
            eprintln!(
                "  {} static-compatible",
                console::style("SSR status:").bold(),
            );
        } else {
            eprintln!(
                "  {} requires SSR runtime",
                console::style("SSR status:").bold(),
            );
        }
    }

    if !result.metadata.tools.is_empty() {
        eprintln!(
            "  {} {}",
            console::style("Tools:").bold(),
            result
                .metadata
                .tools
                .iter()
                .map(|t| {
                    match t.version.as_deref() {
                        Some(v) => format!("{} ({v})", t.name),
                        None => t.name.clone(),
                    }
                })
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    if !result.metadata.structure.is_empty() {
        eprintln!(
            "  {} {}",
            console::style("Structure:").bold(),
            result
                .metadata
                .structure
                .iter()
                .map(|s| format!("{s}/"))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    eprintln!();
    eprintln!("  {} {}", console::style("Reason:").dim(), result.reason);
    eprintln!();
}
