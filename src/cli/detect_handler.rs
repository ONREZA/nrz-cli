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
            .take(detect::fs::MAX_DETECTION_MANIFEST_BYTES as u64 + 1)
            .read_to_string(&mut input)
            .context("failed to read stdin")?;
        if input.len() > detect::fs::MAX_DETECTION_MANIFEST_BYTES {
            return Err(output::coded_error(
                "DETECTION_INPUT_TOO_LARGE",
                format!(
                    "stdin detection manifest exceeds {} bytes",
                    detect::fs::MAX_DETECTION_MANIFEST_BYTES
                ),
            ));
        }

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
        if !nrz::config::save_framework(&project_dir, &result.framework)? {
            anyhow::bail!(
                "cannot save detected framework: onreza.toml not found in {}. Run `nrz init --local` first or create onreza.toml.",
                project_dir.display()
            );
        }
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
            println!("{}", output::terminal_line(&result.framework));
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
    let framework_name = output::terminal_line(&result.name);
    let framework_slug = output::terminal_line(&result.framework);
    eprintln!();
    eprintln!(
        "  {} {} ({})",
        console::style("Framework:").bold(),
        console::style(framework_name).cyan().bold(),
        framework_slug,
    );

    if let Some(ref ver) = result.version {
        eprintln!(
            "  {} {}",
            console::style("Version:").bold(),
            output::terminal_line(ver)
        );
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
            .map(|v| format!(" ({})", output::terminal_line(v)))
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
            eprintln!(
                "  {} {}",
                console::style("Build command:").bold(),
                output::terminal_line(cmd)
            );
        }
        if let Some(ref dir) = build.output_dir {
            eprintln!(
                "  {} {}",
                console::style("Output dir:").bold(),
                output::terminal_line(dir)
            );
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
                    .map(|n| {
                        format!(
                            "{} ({})",
                            output::terminal_line(n),
                            output::terminal_line(&pkg.path)
                        )
                    })
                    .unwrap_or_else(|| output::terminal_line(&pkg.path));
                eprintln!("    {} {label}", console::style("·").dim());
            }
        }
    }

    if !result.metadata.config_files.is_empty() {
        eprintln!(
            "  {} {}",
            console::style("Config files:").bold(),
            output::terminal_line(&result.metadata.config_files.join(", ")),
        );
    }

    if let Some(ref ssr) = result.metadata.ssr_analysis
        && ssr.has_ssr_features()
    {
        eprintln!(
            "  {} {}",
            console::style("SSR features:").bold(),
            output::terminal_line(&ssr.ssr_features.join(", ")),
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

    if !result.metadata.structure.is_empty() {
        eprintln!(
            "  {} {}",
            console::style("Structure:").bold(),
            output::terminal_line(
                &result
                    .metadata
                    .structure
                    .iter()
                    .map(|s| format!("{s}/"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        );
    }

    eprintln!();
    eprintln!(
        "  {} {}",
        console::style("Reason:").dim(),
        output::terminal_line(&result.reason)
    );
    eprintln!();
}
