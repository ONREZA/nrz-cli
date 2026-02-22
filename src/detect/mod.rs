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

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use anyhow::Context;
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
                    entry_point: None,
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
    let suggested_compute = infer_unknown_compute_type(project_dir, pkg.as_ref());
    let reason = if suggested_compute == ComputeType::Process {
        "No known framework detected, but runtime entry signals found (scripts/main/module)"
            .to_string()
    } else {
        "No known framework detected".to_string()
    };

    DetectionResult {
        framework: preset.slug.to_string(),
        name: preset.name.to_string(),
        version: None,
        suggested_compute,
        metadata: DetectionMetadata {
            uses_typescript: detect_typescript(project_dir),
            config_files: Vec::new(),
            runtime: RuntimeInfo {
                runtime_type: infer_runtime(preset.runtime, &pm_info),
                version: None,
            },
            package_manager: pm_info,
            build_info: None,
            monorepo: detect_monorepo(pkg.as_ref()),
            ssr_analysis: None,
            ssr_adapter: None,
            structure: detect_structure(project_dir),
        },
        reason,
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

            // SSR analysis for capable frameworks (needed before output_dir resolution)
            let ssr_analysis = ssr::analyze_ssr(project_dir, preset.slug);

            // Output directory — context-dependent based on framework + SSR analysis
            let output_dir =
                resolve_framework_output_dir(preset, ssr_analysis.as_ref(), project_dir);

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

            let entry_point = framework_entry_point(preset.slug);

            return Some(DetectionResult {
                framework: preset.slug.to_string(),
                name: preset.name.to_string(),
                version,
                suggested_compute,
                metadata: DetectionMetadata {
                    uses_typescript: detect_typescript(project_dir),
                    config_files,
                    runtime: RuntimeInfo {
                        runtime_type: infer_runtime(preset.runtime, pm_info),
                        version: None,
                    },
                    package_manager: pm_info.clone(),
                    build_info: Some(BuildInfo {
                        build_command: build_cmd,
                        install_command: Some(
                            package_manager::install_command(pm_type).to_string(),
                        ),
                        output_dir: Some(output_dir),
                        entry_point,
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

/// Resolve the output directory dynamically based on framework + SSR analysis.
///
/// Overrides the preset default when the SSR analysis reveals a more specific path:
/// - Next.js: `out` (export), `.next/standalone` (standalone), `.next` (default)
/// - Nuxt: `.output/public` (static), `.output` (SSR)
/// - Other frameworks: check vite config for custom outDir, then preset default
fn resolve_framework_output_dir(
    preset: &types::FrameworkPreset,
    ssr: Option<&SsrAnalysis>,
    project_dir: &Path,
) -> String {
    match preset.slug {
        "nextjs" => {
            if let Some(ssr) = ssr {
                if ssr.is_static_compatible {
                    return "out".to_string();
                }
                if ssr.ssr_features.iter().any(|f| f.contains("standalone")) {
                    return ".next/standalone".to_string();
                }
            }
            ".next".to_string()
        }
        "nuxt" => {
            if let Some(ssr) = ssr
                && ssr.is_static_compatible
            {
                return ".output/public".to_string();
            }
            ".output".to_string()
        }
        _ => {
            // For non-SSR frameworks with a vite config, check outDir override
            if !presets::is_ssr_framework(preset.slug)
                && vite_config::has_vite_config(project_dir)
                && let Some(out_dir) = vite_config::parse_vite_out_dir(project_dir)
            {
                return out_dir;
            }
            preset.output_directory.to_string()
        }
    }
}

/// Infer runtime type from package manager.
/// If the project uses Bun as PM, override the runtime to Bun.
/// Static runtime is never overridden.
fn infer_runtime(preset_runtime: RuntimeType, pm_info: &Option<PackageManagerInfo>) -> RuntimeType {
    if preset_runtime == RuntimeType::Static {
        return RuntimeType::Static;
    }
    if pm_info
        .as_ref()
        .is_some_and(|pm| pm.pm_type == PackageManagerType::Bun)
    {
        return RuntimeType::Bun;
    }
    preset_runtime
}

/// Infer suggested compute type from detection metadata.
///
/// Rules (in priority order):
/// - STATIC: static runtime, non-SSR framework (CRA, Vite, Gatsby, etc.),
///   or SSR framework with `is_static_compatible = true`
/// - ISOLATE: @onreza adapter installed (any framework, takes priority)
/// - PROCESS: SSR framework with `is_static_compatible = false` or no SSR analysis
///
/// Each framework analyzer sets `is_static_compatible` based on its own defaults:
/// - Next.js/Nuxt/SvelteKit default to `false` (SSR by default, needs explicit static config)
/// - Astro defaults to `true` (static by default, needs explicit SSR config)
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

    // Non-SSR frameworks (CRA, Vite, Gatsby, etc.) → STATIC
    if !presets::is_ssr_framework(framework) {
        return ComputeType::Static;
    }

    // SSR frameworks: trust the analyzer's is_static_compatible flag
    match ssr {
        Some(analysis) if analysis.is_static_compatible => ComputeType::Static,
        _ => ComputeType::Process,
    }
}

/// Infer compute type for unknown (`other`) frameworks.
///
/// We avoid framework hardcoding and use generic runtime signals:
/// - runtime-like scripts (`start`, `serve`, `prod`, ...)
/// - resolvable `main`/`module` path in package.json
fn infer_unknown_compute_type(project_dir: &Path, pkg: Option<&PackageJson>) -> ComputeType {
    let Some(pkg) = pkg else {
        return ComputeType::Static;
    };

    let has_runtime_script = pkg
        .scripts
        .iter()
        .any(|(name, _)| is_runtime_script_name(name));
    if has_runtime_script {
        return ComputeType::Process;
    }

    let has_main_entry = pkg
        .main
        .as_deref()
        .and_then(|raw| resolve_candidate_path(raw, project_dir, project_dir))
        .is_some();
    if has_main_entry {
        return ComputeType::Process;
    }

    let has_module_entry = pkg
        .module
        .as_deref()
        .and_then(|raw| resolve_candidate_path(raw, project_dir, project_dir))
        .is_some();
    if has_module_entry {
        return ComputeType::Process;
    }

    ComputeType::Static
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

/// Known entry point for a framework's build output (relative to output dir).
fn framework_entry_point(slug: &str) -> Option<String> {
    match slug {
        "nextjs" => Some("server.js".into()),
        "nuxt" => Some("server/index.mjs".into()),
        "sveltekit" => Some("index.js".into()),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPointSource {
    FrameworkHint,
    PackageMainOutput,
    PackageModuleOutput,
    PackageMainProject,
    PackageModuleProject,
    ScriptHintOutput,
    ScriptHintProject,
    BunIndexDefault,
    RootPattern,
    HeuristicScan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEntryPoint {
    pub path: String,
    pub source: EntryPointSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryPointResolution {
    Found(ResolvedEntryPoint),
    Ambiguous(Vec<String>),
    NotFound,
    Error(String),
}

const RUNNABLE_EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "ts", "mts", "cts", "jsx", "tsx"];
const ROOT_ENTRY_BASENAMES: &[&str] = &["server", "main", "app", "start", "entry", "index"];
const SCRIPT_HINT_PRIORITY: &[&str] = &["start", "serve", "preview", "prod", "production"];
const SCRIPT_RUNTIME_NAME_HINTS: &[&str] = &[
    "start",
    "serve",
    "preview",
    "prod",
    "production",
    "runtime",
    "server",
    "launch",
];
const SCRIPT_NON_RUNTIME_NAME_HINTS: &[&str] = &[
    "test",
    "lint",
    "format",
    "fmt",
    "build",
    "typecheck",
    "check",
    "verify",
    "ci",
    "prepare",
    "prepublish",
    "postinstall",
    "install",
    "coverage",
    "bench",
    "docs",
    "storybook",
    "e2e",
    "unit",
    "integration",
];
const SCRIPT_EXECUTORS: &[&str] = &[
    "node",
    "bun",
    "tsx",
    "ts-node",
    "deno",
    "npx",
    "npm",
    "pnpm",
    "yarn",
    "cross-env",
    "env",
    "dotenv",
    "dotenvx",
    "concurrently",
    "nodemon",
    "pm2",
    "forever",
];
const SCRIPT_EXECUTOR_MODULE_TOKENS: &[&str] = &[
    "dotenv/config",
    "dotenvx/config",
    "ts-node/register",
    "tsx/register",
];
const ENTRY_SCAN_SKIP_DIRS: &[&str] = &["node_modules", ".git", ".onreza"];
const ENTRY_SCAN_LIMIT: usize = 4096;

fn stringify_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_windows_drive_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn sanitize_relative_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.replace('\\', "/");
    let lowered = normalized.to_ascii_lowercase();
    let path = Path::new(&normalized);
    if path.is_absolute() || lowered.starts_with("file:") || is_windows_drive_absolute(&normalized)
    {
        return None;
    }

    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Normal(seg) => out.push(seg),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn dedup_push(vec: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, path: PathBuf) {
    if seen.insert(path.clone()) {
        vec.push(path);
    }
}

fn map_candidate_to_output(
    candidate: &Path,
    output_dir: &Path,
    package_dir: &Path,
) -> Option<PathBuf> {
    if output_dir.join(candidate).is_file() {
        return Some(candidate.to_path_buf());
    }

    if package_dir != output_dir {
        if let Ok(output_rel_to_package) = output_dir.strip_prefix(package_dir)
            && !output_rel_to_package.as_os_str().is_empty()
            && candidate.starts_with(output_rel_to_package)
            && let Ok(stripped) = candidate.strip_prefix(output_rel_to_package)
            && !stripped.as_os_str().is_empty()
            && output_dir.join(stripped).is_file()
        {
            return Some(stripped.to_path_buf());
        }

        let package_candidate = package_dir.join(candidate);
        if package_candidate.is_file()
            && let Ok(stripped) = package_candidate.strip_prefix(output_dir)
            && !stripped.as_os_str().is_empty()
        {
            return Some(stripped.to_path_buf());
        }
    }

    None
}

fn resolve_candidate_path(raw: &str, output_dir: &Path, package_dir: &Path) -> Option<String> {
    let rel = sanitize_relative_path(raw)?;

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    dedup_push(&mut candidates, &mut seen, rel.clone());

    if rel.extension().is_none() {
        for ext in RUNNABLE_EXTENSIONS {
            dedup_push(
                &mut candidates,
                &mut seen,
                rel.with_extension(ext.trim_start_matches('.')),
            );
        }
    } else if let Some(stem) = rel.file_stem().and_then(|s| s.to_str()) {
        let parent = rel.parent().unwrap_or_else(|| Path::new(""));
        for ext in RUNNABLE_EXTENSIONS {
            let mut p = parent.to_path_buf();
            p.push(format!("{stem}.{ext}"));
            dedup_push(&mut candidates, &mut seen, p);
        }
    }

    for ext in RUNNABLE_EXTENSIONS {
        let mut p = rel.clone();
        p.push(format!("index.{ext}"));
        dedup_push(&mut candidates, &mut seen, p);
    }

    candidates
        .into_iter()
        .find_map(|candidate| map_candidate_to_output(&candidate, output_dir, package_dir))
        .map(|p| stringify_path(&p))
}

fn resolve_package_field(
    package_dir: &Path,
    output_dir: &Path,
    source_main: EntryPointSource,
    source_module: EntryPointSource,
) -> Option<ResolvedEntryPoint> {
    let pkg = package_json::PackageJson::load(package_dir)?;

    if let Some(ref main) = pkg.main
        && let Some(path) = resolve_candidate_path(main, output_dir, package_dir)
    {
        return Some(ResolvedEntryPoint {
            path,
            source: source_main,
        });
    }
    if let Some(ref module) = pkg.module
        && let Some(path) = resolve_candidate_path(module, output_dir, package_dir)
    {
        return Some(ResolvedEntryPoint {
            path,
            source: source_module,
        });
    }
    None
}

fn looks_like_script_path_token(token: &str) -> bool {
    if token.contains('/') || token.contains('\\') {
        return true;
    }
    if let Some(ext) = Path::new(token).extension().and_then(|e| e.to_str()) {
        return RUNNABLE_EXTENSIONS.contains(&ext);
    }
    ROOT_ENTRY_BASENAMES.contains(&token)
}

fn script_name_has_token(name: &str, wanted: &str) -> bool {
    name.split([':', '-', '_', '.'])
        .any(|part| part.eq_ignore_ascii_case(wanted))
}

fn is_runtime_script_name(name: &str) -> bool {
    if SCRIPT_HINT_PRIORITY.contains(&name) {
        return true;
    }

    if SCRIPT_NON_RUNTIME_NAME_HINTS
        .iter()
        .any(|token| script_name_has_token(name, token))
    {
        return false;
    }

    SCRIPT_RUNTIME_NAME_HINTS
        .iter()
        .any(|token| script_name_has_token(name, token))
}

fn is_known_executor_token(part: &str) -> bool {
    let lower = part.to_ascii_lowercase();
    if SCRIPT_EXECUTORS.contains(&lower.as_str()) {
        return true;
    }
    SCRIPT_EXECUTOR_MODULE_TOKENS.contains(&lower.as_str())
}

fn extract_script_path_tokens(script: &str) -> Vec<String> {
    script
        .split(|c: char| c.is_whitespace() || matches!(c, '|' | '&' | ';' | '(' | ')'))
        .map(|part| part.trim_matches('"').trim_matches('\'').trim_matches('`'))
        .filter(|part| !part.is_empty())
        .filter(|part| !part.starts_with('-'))
        .filter(|part| !part.starts_with('$'))
        .filter(|part| !is_known_executor_token(part))
        .filter(|part| {
            if !part.contains('=') {
                return true;
            }
            looks_like_script_path_token(part)
        })
        .filter(|part| looks_like_script_path_token(part))
        .map(String::from)
        .collect()
}

fn resolve_from_scripts(
    package_dir: &Path,
    output_dir: &Path,
    source: EntryPointSource,
) -> Option<ResolvedEntryPoint> {
    let pkg = package_json::PackageJson::load(package_dir)?;

    for name in SCRIPT_HINT_PRIORITY {
        if let Some(script) = pkg.scripts.get(*name) {
            for token in extract_script_path_tokens(script) {
                if let Some(path) = resolve_candidate_path(&token, output_dir, package_dir) {
                    return Some(ResolvedEntryPoint { path, source });
                }
            }
        }
    }

    let mut rest: Vec<_> = pkg
        .scripts
        .iter()
        .filter(|(name, _)| !SCRIPT_HINT_PRIORITY.contains(&name.as_str()))
        .filter(|(name, _)| is_runtime_script_name(name))
        .collect();
    rest.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (_, script) in rest {
        for token in extract_script_path_tokens(script) {
            if let Some(path) = resolve_candidate_path(&token, output_dir, package_dir) {
                return Some(ResolvedEntryPoint { path, source });
            }
        }
    }

    None
}

fn resolve_bun_default_index(output_dir: &Path) -> Option<ResolvedEntryPoint> {
    for ext in RUNNABLE_EXTENSIONS {
        let candidate = format!("index.{ext}");
        if output_dir.join(&candidate).is_file() {
            return Some(ResolvedEntryPoint {
                path: candidate,
                source: EntryPointSource::BunIndexDefault,
            });
        }
    }
    None
}

fn resolve_root_patterns(output_dir: &Path) -> EntryPointResolution {
    let mut candidates = Vec::new();
    for base in ROOT_ENTRY_BASENAMES {
        for ext in RUNNABLE_EXTENSIONS {
            let candidate = format!("{base}.{ext}");
            if output_dir.join(&candidate).is_file() {
                candidates.push(candidate);
            }
        }
    }

    match candidates.len() {
        0 => EntryPointResolution::NotFound,
        1 => EntryPointResolution::Found(ResolvedEntryPoint {
            path: candidates[0].clone(),
            source: EntryPointSource::RootPattern,
        }),
        _ => {
            let mut ranked: Vec<((i32, i32), String)> = candidates
                .iter()
                .map(|path| (root_candidate_rank(path), path.clone()))
                .collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

            if ranked.len() == 1 || ranked[0].0 > ranked[1].0 {
                return EntryPointResolution::Found(ResolvedEntryPoint {
                    path: ranked[0].1.clone(),
                    source: EntryPointSource::RootPattern,
                });
            }

            let top_rank = ranked[0].0;
            let tied: Vec<String> = ranked
                .into_iter()
                .take_while(|(rank, _)| *rank == top_rank)
                .map(|(_, path)| path)
                .collect();
            EntryPointResolution::Ambiguous(tied)
        }
    }
}

fn should_skip_scan_dir(name: &str) -> bool {
    ENTRY_SCAN_SKIP_DIRS.contains(&name)
}

fn root_base_rank(stem: &str) -> i32 {
    match stem {
        "server" => 6,
        "main" => 5,
        "app" => 4,
        "start" => 3,
        "entry" => 2,
        "index" => 1,
        _ => 0,
    }
}

fn root_ext_rank(ext: &str) -> i32 {
    match ext {
        "js" => 8,
        "mjs" => 7,
        "cjs" => 6,
        "ts" => 5,
        "mts" => 4,
        "cts" => 3,
        "jsx" => 2,
        "tsx" => 1,
        _ => 0,
    }
}

fn root_candidate_rank(path: &str) -> (i32, i32) {
    let p = Path::new(path);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
    (root_base_rank(stem), root_ext_rank(ext))
}

fn is_runnable_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| RUNNABLE_EXTENSIONS.contains(&ext))
}

fn collect_runnable_files_recursive(
    base: &Path,
    current: &Path,
    out: &mut Vec<PathBuf>,
    limit: usize,
) -> anyhow::Result<()> {
    if out.len() >= limit {
        return Ok(());
    }

    let entries = std::fs::read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?;
    for entry in entries {
        if out.len() >= limit {
            return Ok(());
        }

        let entry = entry.with_context(|| {
            format!(
                "failed to read directory entry while scanning {}",
                current.display()
            )
        })?;

        let ft = entry
            .file_type()
            .with_context(|| format!("failed to read file type: {}", entry.path().display()))?;

        if ft.is_symlink() {
            continue;
        }

        let path = entry.path();
        if ft.is_file() {
            if is_runnable_file(&path)
                && let Ok(rel) = path.strip_prefix(base)
            {
                out.push(rel.to_path_buf());
            }
            continue;
        }

        if ft.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && !should_skip_scan_dir(name)
        {
            collect_runnable_files_recursive(base, &path, out, limit)?;
        }
    }

    Ok(())
}

fn path_depth(path: &Path) -> i32 {
    path.components().count().saturating_sub(1) as i32
}

fn looks_hashed_name(stem: &str) -> bool {
    if stem.len() < 8 {
        return false;
    }
    let has_digit = stem.chars().any(|c| c.is_ascii_digit());
    let has_dash = stem.contains('-') || stem.contains('_') || stem.contains('.');
    has_digit && has_dash
}

fn score_candidate(path: &Path) -> i32 {
    let mut score = 0;
    let rel = stringify_path(path);
    let rel_l = rel.to_ascii_lowercase();

    score += 120 - path_depth(path) * 12;

    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        match stem {
            "server" => score += 240,
            "main" => score += 210,
            "app" => score += 200,
            "start" => score += 190,
            "entry" => score += 180,
            "index" => score += 170,
            _ => {}
        }

        if looks_hashed_name(stem) {
            score -= 60;
        }
    }

    if rel_l.contains("/server/") || rel_l.starts_with("server/") {
        score += 60;
    }
    if rel_l.contains("standalone") {
        score += 50;
    }
    if rel_l.contains("/.output/") || rel_l.starts_with(".output/") {
        score += 40;
    }
    if rel_l.contains("/dist/") || rel_l.starts_with("dist/") {
        score += 25;
    }
    if rel_l.contains("/build/") || rel_l.starts_with("build/") {
        score += 20;
    }
    if rel_l.contains("/chunks/")
        || rel_l.contains("/assets/")
        || rel_l.contains("/static/")
        || rel_l.contains("/client/")
    {
        score -= 80;
    }
    if rel_l.contains("/node_modules/") {
        score -= 100;
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        score += match ext {
            "js" => 25,
            "mjs" | "cjs" => 24,
            "ts" | "mts" | "cts" => 20,
            "jsx" | "tsx" => 12,
            _ => 0,
        };
    }

    score
}

fn resolve_by_heuristic_scan(output_dir: &Path) -> EntryPointResolution {
    let mut candidates = Vec::new();
    if let Err(err) =
        collect_runnable_files_recursive(output_dir, output_dir, &mut candidates, ENTRY_SCAN_LIMIT)
    {
        return EntryPointResolution::Error(format!(
            "failed to scan output directory {}: {err:#}",
            output_dir.display()
        ));
    }
    if candidates.is_empty() {
        return EntryPointResolution::NotFound;
    }

    let mut scored: Vec<(i32, String)> = candidates
        .into_iter()
        .map(|p| {
            let score = score_candidate(&p);
            (score, stringify_path(&p))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let best_score = scored[0].0;
    let best: Vec<String> = scored
        .iter()
        .take_while(|(score, _)| *score == best_score)
        .map(|(_, path)| path.clone())
        .collect();

    if best.len() > 1 {
        return EntryPointResolution::Ambiguous(best);
    }

    EntryPointResolution::Found(ResolvedEntryPoint {
        path: scored[0].1.clone(),
        source: EntryPointSource::HeuristicScan,
    })
}

/// Resolve entry point for a PROCESS deployment with diagnostics.
pub fn resolve_entry_point_detailed(
    framework: &str,
    output_dir: &Path,
    project_dir: &Path,
) -> EntryPointResolution {
    // 1. Framework-specific hint
    if let Some(entry) = framework_entry_point(framework)
        && output_dir.join(&entry).is_file()
    {
        return EntryPointResolution::Found(ResolvedEntryPoint {
            path: entry,
            source: EntryPointSource::FrameworkHint,
        });
    }

    // 2. package.json fields in output_dir (main/module)
    if let Some(resolved) = resolve_package_field(
        output_dir,
        output_dir,
        EntryPointSource::PackageMainOutput,
        EntryPointSource::PackageModuleOutput,
    ) {
        return EntryPointResolution::Found(resolved);
    }

    // 3. package.json fields in project_dir when output_dir != project_dir
    if output_dir != project_dir
        && let Some(resolved) = resolve_package_field(
            project_dir,
            output_dir,
            EntryPointSource::PackageMainProject,
            EntryPointSource::PackageModuleProject,
        )
    {
        return EntryPointResolution::Found(resolved);
    }

    // 4. script hints (`start`, `serve`, ...)
    if let Some(resolved) =
        resolve_from_scripts(output_dir, output_dir, EntryPointSource::ScriptHintOutput)
    {
        return EntryPointResolution::Found(resolved);
    }
    if output_dir != project_dir
        && let Some(resolved) =
            resolve_from_scripts(project_dir, output_dir, EntryPointSource::ScriptHintProject)
    {
        return EntryPointResolution::Found(resolved);
    }

    // 5. Common root entry names (includes index.*; fail fast on ambiguity)
    match resolve_root_patterns(output_dir) {
        EntryPointResolution::NotFound => {}
        other => return other,
    }

    // 6. Bun default index.* in output root (defensive fallback)
    if let Some(resolved) = resolve_bun_default_index(output_dir) {
        return EntryPointResolution::Found(resolved);
    }

    // 7. Heuristic recursive scan (last resort)
    resolve_by_heuristic_scan(output_dir)
}

/// Resolve entry point for a PROCESS deployment.
///
/// Returns `Some(path)` only when resolution is unambiguous.
#[allow(dead_code)]
pub fn resolve_entry_point(
    framework: &str,
    output_dir: &Path,
    project_dir: &Path,
) -> Option<String> {
    match resolve_entry_point_detailed(framework, output_dir, project_dir) {
        EntryPointResolution::Found(resolved) => Some(resolved.path),
        EntryPointResolution::Ambiguous(_)
        | EntryPointResolution::NotFound
        | EntryPointResolution::Error(_) => None,
    }
}

/// Detect package manager name (backward-compatible wrapper for init/deploy).
pub fn detect_package_manager_name(project_dir: &Path) -> String {
    let pkg = PackageJson::load(project_dir);
    let pm = package_manager::detect_package_manager(project_dir, pkg.as_ref());
    pm.map(|p| p.pm_type.as_str().to_string())
        .unwrap_or_else(|| "npm".to_string())
}
