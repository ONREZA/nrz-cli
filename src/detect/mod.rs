//! Framework detection module — the source of truth for detecting
//! frameworks, package managers, SSR features, and adapters.

pub mod adapter;
pub mod package_json;
pub mod package_manager;
pub mod presets;
pub mod ssr;
pub mod static_html;
pub mod types;
pub mod vite_config;

#[cfg(test)]
mod adapter_tests;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod package_json_tests;
#[cfg(test)]
mod package_manager_tests;
#[cfg(test)]
mod presets_tests;
#[cfg(test)]
mod ssr_tests;
#[cfg(test)]
mod static_html_tests;
#[cfg(test)]
mod vite_config_tests;

use std::path::Path;

use package_json::PackageJson;
use types::*;

/// Full framework detection — returns a complete `DetectionResult`.
pub fn detect(project_dir: &Path) -> DetectionResult {
    let pkg = PackageJson::load(project_dir);

    // 1. Detect package manager
    let pm_info = package_manager::detect_package_manager(project_dir, pkg.as_ref());

    // 2. Try to detect framework from package.json dependencies
    if let Some(ref pkg) = pkg
        && let Some(result) = detect_from_package_json(project_dir, pkg, &pm_info)
    {
        return result;
    }

    // 3. Fallback: static HTML site (no package.json + index.html)
    if static_html::is_static_html_site(project_dir) {
        let preset = presets::get_static_html_preset();
        let html_files = static_html::find_html_files(project_dir);

        return DetectionResult {
            framework: preset.slug.to_string(),
            name: preset.name.to_string(),
            version: None,
            suggested_compute: ComputeType::Static,
            metadata: DetectionMetadata {
                uses_typescript: None,
                config_files: Vec::new(),
                runtime: RuntimeInfo {
                    runtime_type: RuntimeType::Static,
                    version: None,
                },
                package_manager: None,
                build_info: Some(BuildInfo {
                    build_command: None,
                    install_command: None,
                    output_dir: Some(".".to_string()),
                }),
                monorepo: None,
                ssr_analysis: None,
                ssr_adapter: None,
                structure: html_files,
            },
            reason: "Static HTML site detected (index.html found, no package.json)".into(),
        };
    }

    // 4. Unknown project
    let preset = presets::get_default_preset();
    DetectionResult {
        framework: preset.slug.to_string(),
        name: preset.name.to_string(),
        version: None,
        suggested_compute: ComputeType::Static,
        metadata: DetectionMetadata {
            uses_typescript: detect_typescript(project_dir),
            config_files: Vec::new(),
            runtime: RuntimeInfo {
                runtime_type: preset.runtime,
                version: None,
            },
            package_manager: pm_info,
            build_info: None,
            monorepo: detect_monorepo(pkg.as_ref()),
            ssr_analysis: None,
            ssr_adapter: None,
            structure: detect_structure(project_dir),
        },
        reason: "No known framework detected".into(),
    }
}

/// Detect framework from package.json dependencies using preset priorities.
fn detect_from_package_json(
    project_dir: &Path,
    pkg: &PackageJson,
    pm_info: &Option<PackageManagerInfo>,
) -> Option<DetectionResult> {
    // Iterate presets in priority order (already sorted)
    for preset in presets::detection_presets() {
        let matched_dep = preset
            .dependencies
            .iter()
            .find(|&&dep| pkg.has_dependency(dep));

        if let Some(&dep) = matched_dep {
            let version = pkg.dependency_version(dep).map(|s| s.to_string());
            let pm_type = pm_info
                .as_ref()
                .map(|pm| pm.pm_type)
                .unwrap_or(PackageManagerType::Npm);

            // Build info
            let build_cmd = preset
                .build_script
                .map(|script| package_manager::build_command(pm_type, script));

            // Output directory — check vite config override for vite-based frameworks
            let output_dir = if preset.slug == "vite" {
                vite_config::parse_vite_out_dir(project_dir)
                    .unwrap_or_else(|| preset.output_directory.to_string())
            } else {
                preset.output_directory.to_string()
            };

            // SSR analysis for capable frameworks
            let ssr_analysis = ssr::analyze_ssr(project_dir, preset.slug);

            // Adapter detection
            let ssr_adapter = adapter::detect_adapter(pkg);

            // Config files
            let config_files = detect_config_files(project_dir, preset.slug);

            // Infer compute type
            let suggested_compute = infer_compute_type(
                preset.runtime,
                preset.slug,
                ssr_analysis.as_ref(),
                ssr_adapter.as_ref(),
            );

            return Some(DetectionResult {
                framework: preset.slug.to_string(),
                name: preset.name.to_string(),
                version,
                suggested_compute,
                metadata: DetectionMetadata {
                    uses_typescript: detect_typescript(project_dir),
                    config_files,
                    runtime: RuntimeInfo {
                        runtime_type: preset.runtime,
                        version: None,
                    },
                    package_manager: pm_info.clone(),
                    build_info: Some(BuildInfo {
                        build_command: build_cmd,
                        install_command: Some(
                            package_manager::install_command(pm_type).to_string(),
                        ),
                        output_dir: Some(output_dir),
                    }),
                    monorepo: detect_monorepo(Some(pkg)),
                    ssr_analysis,
                    ssr_adapter,
                    structure: detect_structure(project_dir),
                },
                reason: format!(
                    "Detected {dep} in dependencies (priority {})",
                    preset.priority
                ),
            });
        }
    }

    None
}

/// Detect TypeScript usage (tsconfig.json or tsconfig.app.json).
fn detect_typescript(project_dir: &Path) -> Option<bool> {
    if project_dir.join("tsconfig.json").exists() || project_dir.join("tsconfig.app.json").exists()
    {
        Some(true)
    } else {
        Some(false)
    }
}

/// Detect monorepo (workspaces in package.json).
fn detect_monorepo(pkg: Option<&PackageJson>) -> Option<MonorepoInfo> {
    let pkg = pkg?;
    if pkg.is_monorepo() {
        Some(MonorepoInfo {
            workspaces: pkg.workspaces.to_vec(),
        })
    } else {
        None
    }
}

/// Detect framework-specific config files.
fn detect_config_files(project_dir: &Path, framework: &str) -> Vec<String> {
    let candidates: &[&str] = match framework {
        "nextjs" => &[
            "next.config.js",
            "next.config.mjs",
            "next.config.ts",
            "next.config.mts",
        ],
        "nuxt" => &["nuxt.config.ts", "nuxt.config.js"],
        "sveltekit" => &["svelte.config.js"],
        "astro" => &["astro.config.mjs", "astro.config.ts", "astro.config.js"],
        "vite" => &[
            "vite.config.ts",
            "vite.config.mts",
            "vite.config.js",
            "vite.config.mjs",
        ],
        "gatsby" => &["gatsby-config.js", "gatsby-config.ts"],
        "angular" => &["angular.json"],
        "docusaurus" => &["docusaurus.config.js", "docusaurus.config.ts"],
        "vitepress" => &[".vitepress/config.ts", ".vitepress/config.js"],
        _ => &[],
    };

    candidates
        .iter()
        .filter(|&&f| project_dir.join(f).exists())
        .map(|f| f.to_string())
        .collect()
}

/// Detect key project structure directories.
fn detect_structure(project_dir: &Path) -> Vec<String> {
    let dirs = &[
        "src",
        "pages",
        "app",
        "public",
        "static",
        "components",
        "lib",
        "server",
    ];
    dirs.iter()
        .filter(|&&d| project_dir.join(d).is_dir())
        .map(|d| d.to_string())
        .collect()
}

/// Infer suggested compute type from detection metadata.
///
/// Rules:
/// - STATIC: static runtime, or SSR framework with static export and no adapter
/// - ISOLATE: SSR framework with @onreza adapter installed
/// - PROCESS: SSR framework without adapter (standalone Next.js, full-runtime frameworks)
fn infer_compute_type(
    runtime: RuntimeType,
    framework: &str,
    ssr: Option<&SsrAnalysis>,
    adapter: Option<&AdapterInfo>,
) -> ComputeType {
    // Static runtime → always STATIC
    if runtime == RuntimeType::Static {
        return ComputeType::Static;
    }

    // Adapter installed → ISOLATE regardless of framework category
    if adapter.is_some() {
        return ComputeType::Isolate;
    }

    // Non-SSR frameworks (CRA, Vite, Gatsby, Astro default, etc.) → STATIC
    if !presets::is_ssr_framework(framework) {
        return ComputeType::Static;
    }

    // SSR frameworks: check SSR analysis
    match ssr {
        Some(analysis) if analysis.is_static_compatible && analysis.has_ssr_features() => {
            // Explicitly configured for static (e.g. output: 'export') → STATIC
            ComputeType::Static
        }
        _ => {
            // SSR framework default (clean project or has SSR features) → PROCESS
            ComputeType::Process
        }
    }
}

// ── Public convenience wrappers (for init/deploy) ────────────

/// Detect framework slug only (backward-compatible wrapper for init).
#[allow(dead_code)]
pub fn detect_framework_slug(project_dir: &Path) -> Option<String> {
    let result = detect(project_dir);
    if result.framework == "other" {
        None
    } else {
        Some(result.framework)
    }
}

/// Detect package manager name (backward-compatible wrapper for init/deploy).
pub fn detect_package_manager_name(project_dir: &Path) -> String {
    let pkg = PackageJson::load(project_dir);
    let pm = package_manager::detect_package_manager(project_dir, pkg.as_ref());
    pm.map(|p| p.pm_type.as_str().to_string())
        .unwrap_or_else(|| "npm".to_string())
}
