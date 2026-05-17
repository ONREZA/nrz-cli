pub(crate) mod manifest;

#[cfg(test)]
mod manifest_tests;

#[cfg(test)]
mod build_tests;

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;

use crate::cli::BuildArgs;
use crate::output;
pub(crate) use nrz::config::BuildSettingSource;
use nrz::config::{EffectiveProjectConfig, ProjectConfig};

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct OutputDirectoryHint<'a> {
    pub path: &'a str,
    pub source: BuildSettingSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedOutputDirectory {
    path: PathBuf,
    has_manifest: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputCandidateRole {
    DetectedFrameworkRefinement,
    DetectedHint,
    Framework,
    PresetHint,
    ConfigDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputCandidate {
    path: String,
    role: OutputCandidateRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputCandidateTier {
    role: OutputCandidateRole,
    candidates: Vec<OutputCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UserOutputRefinement {
    FrameworkContainerOnly,
    FrameworkContainerAndProjectRoot,
}

impl UserOutputRefinement {
    fn for_framework(framework: &str) -> Self {
        if is_nextjs_standalone_framework(framework) {
            Self::FrameworkContainerAndProjectRoot
        } else {
            Self::FrameworkContainerOnly
        }
    }

    fn allows_project_root(self) -> bool {
        self == Self::FrameworkContainerAndProjectRoot
    }
}

pub async fn run(
    args: BuildArgs,
    json: bool,
    config: &ProjectConfig,
) -> anyhow::Result<BuildResult> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;
    let effective = EffectiveProjectConfig::from_project_config(project_dir, config.clone());
    run_with_effective_config(args, json, &effective, None).await
}

#[cfg(test)]
pub async fn run_with_hint(
    args: BuildArgs,
    json: bool,
    config: &ProjectConfig,
    detection: Option<&crate::detect::types::DetectionResult>,
    server_output_dir: Option<OutputDirectoryHint<'_>>,
) -> anyhow::Result<BuildResult> {
    let project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;
    let mut effective = EffectiveProjectConfig::from_project_config(project_dir, config.clone());
    if let Some(hint) = server_output_dir {
        let settings = nrz::config::ProjectBuildSettings {
            output_directory: Some(hint.path.to_string()),
            output_directory_source: Some(hint.source),
            ..Default::default()
        };
        effective.apply_server_settings(Some(&settings));
    }

    run_with_effective_config(args, json, &effective, detection).await
}

pub(crate) async fn run_with_effective_config(
    args: BuildArgs,
    json: bool,
    effective: &EffectiveProjectConfig,
    detection: Option<&crate::detect::types::DetectionResult>,
) -> anyhow::Result<BuildResult> {
    let project_dir = effective.project_dir();

    let internal_detection;
    let detection = match detection {
        Some(d) => d,
        None => {
            internal_detection = crate::detect::detect_with_framework_override(
                project_dir,
                effective.framework_override(),
            );
            &internal_detection
        }
    };
    let fw_dirs = compute_aware_output_dirs(detection);
    let output_directory_hint = effective
        .output_directory()
        .and_then(|setting| setting.value())
        .map(|path| OutputDirectoryHint {
            path,
            source: effective
                .output_directory()
                .map(|setting| setting.source_or_preset())
                .unwrap_or(BuildSettingSource::Preset),
        });
    let (output_dir, has_manifest) = detect_output_dir_for_framework(
        project_dir,
        &effective.output_dirs(),
        &fw_dirs,
        output_directory_hint,
        &detection.framework,
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
    } else if is_nextjs_standalone_framework(&detection.framework)
        && (detection
            .metadata
            .ssr_analysis
            .as_ref()
            .is_some_and(|ssr| ssr.has_standalone_output())
            || resolve_nextjs_standalone_server(project_dir, &output_dir).is_some())
    {
        let Some(standalone) = resolve_nextjs_standalone_server(project_dir, &output_dir) else {
            return Err(output::coded_error(
                "MISSING_BUILD_OUTPUT",
                format!(
                    "server.js not found in standalone output {}. \
                     Ensure `output: 'standalone'` is set in next.config.js \
                     and `next build` completed successfully.",
                    output_dir.display()
                ),
            ));
        };

        prepare_nextjs_standalone_for_server(
            project_dir,
            &output_dir,
            &standalone.server_dir,
            json,
        )?;
        let has_public = standalone.server_dir.join("public").is_dir();
        let auto = manifest::generate_nextjs_standalone_manifest_for_server(
            has_public,
            standalone.server_dir_relative.as_deref().unwrap_or(""),
            &standalone.entry,
        );
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
/// Returns `(path, has_manifest)` — first selects a precedence tier, then prefers dirs
/// with `.onreza/` within that tier before plain existing dirs.
///
/// Priority:
/// - USER outputDirectory: exact directory only; never falls back.
/// - DETECTED outputDirectory: current framework refinements of the detected hint,
///   then detected path, then framework dirs, then config dirs.
/// - PRESET/default outputDirectory: framework dirs > preset path > config dirs.
#[cfg(test)]
fn detect_output_dir(
    project_dir: &Path,
    config_dirs: &[&str],
    framework_dirs: &[&str],
    server_output_dir: Option<OutputDirectoryHint<'_>>,
) -> anyhow::Result<(std::path::PathBuf, bool)> {
    let resolved = resolve_output_directory(
        project_dir,
        config_dirs,
        framework_dirs,
        server_output_dir,
        UserOutputRefinement::FrameworkContainerOnly,
    )?;
    Ok((resolved.path, resolved.has_manifest))
}

fn detect_output_dir_for_framework(
    project_dir: &Path,
    config_dirs: &[&str],
    framework_dirs: &[&str],
    server_output_dir: Option<OutputDirectoryHint<'_>>,
    framework: &str,
) -> anyhow::Result<(std::path::PathBuf, bool)> {
    let resolved = resolve_output_directory(
        project_dir,
        config_dirs,
        framework_dirs,
        server_output_dir,
        UserOutputRefinement::for_framework(framework),
    )?;
    Ok((resolved.path, resolved.has_manifest))
}

fn resolve_output_directory(
    project_dir: &Path,
    config_dirs: &[&str],
    framework_dirs: &[&str],
    server_output_dir: Option<OutputDirectoryHint<'_>>,
    user_output_refinement: UserOutputRefinement,
) -> anyhow::Result<ResolvedOutputDirectory> {
    if let Some(hint) = server_output_dir
        && hint.source.is_user_explicit()
    {
        if let Some(refined) = resolve_user_framework_container_artifact(
            project_dir,
            hint.path,
            framework_dirs,
            user_output_refinement,
        ) {
            return Ok(refined);
        }

        let candidate = project_dir.join(hint.path);
        if candidate.is_dir() {
            return Ok(ResolvedOutputDirectory {
                has_manifest: candidate.join(".onreza").is_dir(),
                path: candidate,
            });
        }

        return Err(output::coded_error(
            "MISSING_BUILD_OUTPUT",
            format!(
                "explicit outputDirectory '{}' was not found in {}. \
                 User-configured outputDirectory is authoritative, so no fallback output directory was used.",
                hint.path,
                project_dir.display()
            ),
        ));
    }

    // Log when non-explicit server-provided directory doesn't exist on disk.
    if let Some(hint) = server_output_dir
        && !project_dir.join(hint.path).is_dir()
    {
        tracing::debug!(
            server_output_dir = hint.path,
            source = ?hint.source,
            "server-configured output directory not found on disk, will try other candidates"
        );
    }

    // Build candidate tiers according to the source-aware precedence matrix.
    // Manifest preference applies within a tier only; it must not let stale
    // lower-priority outputs outrank a higher-priority existing directory.
    let mut seen = std::collections::HashSet::<String>::new();
    let mut all_dirs = Vec::<String>::new();
    let mut tiers = Vec::<OutputCandidateTier>::new();
    let server_path = server_output_dir.map(|hint| (hint.path, hint.source));

    if let Some((path, BuildSettingSource::Detected)) = server_path {
        // A detected framework container like `.next` is a pre-build hint. If current
        // framework analysis has a more precise artifact root, prefer that refinement
        // over the generic container.
        push_output_candidate_tier(
            &mut tiers,
            &mut all_dirs,
            &mut seen,
            OutputCandidateRole::DetectedFrameworkRefinement,
            framework_dirs
                .iter()
                .copied()
                .filter(|candidate| is_framework_refinement_of_detected_hint(path, candidate)),
        );
        push_output_candidate_tier(
            &mut tiers,
            &mut all_dirs,
            &mut seen,
            OutputCandidateRole::DetectedHint,
            std::iter::once(path),
        );
    }

    push_output_candidate_tier(
        &mut tiers,
        &mut all_dirs,
        &mut seen,
        OutputCandidateRole::Framework,
        framework_dirs.iter().copied(),
    );

    if let Some((path, BuildSettingSource::Preset)) = server_path {
        push_output_candidate_tier(
            &mut tiers,
            &mut all_dirs,
            &mut seen,
            OutputCandidateRole::PresetHint,
            std::iter::once(path),
        );
    }

    push_output_candidate_tier(
        &mut tiers,
        &mut all_dirs,
        &mut seen,
        OutputCandidateRole::ConfigDefault,
        config_dirs.iter().copied(),
    );

    for tier in &tiers {
        if let Some(found) = select_output_dir_from_tier(project_dir, tier) {
            return Ok(found);
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

fn resolve_user_framework_container_artifact(
    project_dir: &Path,
    hint_path: &str,
    framework_dirs: &[&str],
    refinement: UserOutputRefinement,
) -> Option<ResolvedOutputDirectory> {
    // Some historic/server-side settings store the framework container as a
    // USER value. For Next.js, `.next` is that container; the deployable
    // artifact is mode-specific (`.next/standalone` for PROCESS or `out` for
    // static export). Keep arbitrary USER paths exact, but resolve known
    // framework containers through the same adapter-derived candidates.
    let hint_path = hint_path.trim_matches('/');
    let is_project_root_hint = hint_path.is_empty() || hint_path == ".";
    let is_next_container_hint = hint_path == ".next";
    if is_project_root_hint && !refinement.allows_project_root() {
        return None;
    }
    if is_project_root_hint && project_dir.join(".onreza/manifest.json").is_file() {
        return None;
    }
    if !is_project_root_hint && !is_next_container_hint {
        return None;
    }

    framework_dirs
        .iter()
        .copied()
        .filter(|candidate| {
            if is_project_root_hint {
                matches!(candidate.trim_matches('/'), ".next/standalone" | "out")
            } else {
                matches!(candidate.trim_matches('/'), ".next/standalone" | "out")
                    && is_framework_refinement_of_detected_hint(".next", candidate)
            }
        })
        .find_map(|candidate| {
            let path = project_dir.join(candidate);
            path.is_dir().then(|| ResolvedOutputDirectory {
                has_manifest: path.join(".onreza").is_dir(),
                path,
            })
        })
}

fn push_output_candidate_tier<'a>(
    tiers: &mut Vec<OutputCandidateTier>,
    all_dirs: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    role: OutputCandidateRole,
    candidates: impl IntoIterator<Item = &'a str>,
) {
    let mut tier = Vec::new();

    for candidate in candidates {
        let candidate = candidate.to_string();
        if seen.insert(candidate.clone()) {
            all_dirs.push(candidate.clone());
            tier.push(OutputCandidate {
                path: candidate,
                role,
            });
        }
    }

    if !tier.is_empty() {
        tiers.push(OutputCandidateTier {
            role,
            candidates: tier,
        });
    }
}

fn select_output_dir_from_tier(
    project_dir: &Path,
    tier: &OutputCandidateTier,
) -> Option<ResolvedOutputDirectory> {
    for candidate in &tier.candidates {
        debug_assert_eq!(candidate.role, tier.role);
        let path = project_dir.join(&candidate.path);
        if path.is_dir() && path.join(".onreza").is_dir() {
            return Some(ResolvedOutputDirectory {
                path,
                has_manifest: true,
            });
        }
    }

    for candidate in &tier.candidates {
        debug_assert_eq!(candidate.role, tier.role);
        let path = project_dir.join(&candidate.path);
        if path.is_dir() {
            return Some(ResolvedOutputDirectory {
                path,
                has_manifest: false,
            });
        }
    }

    None
}

fn is_framework_refinement_of_detected_hint(hint: &str, candidate: &str) -> bool {
    is_project_root_output_dir(hint) && !is_project_root_output_dir(candidate)
        || is_nested_output_dir(hint, candidate)
        || is_nextjs_export_refinement(hint, candidate)
}

fn is_project_root_output_dir(path: &str) -> bool {
    matches!(path.trim_matches('/'), "." | "./")
}

fn is_nested_output_dir(parent: &str, candidate: &str) -> bool {
    let parent = parent.trim_end_matches('/');
    let candidate = candidate.trim_end_matches('/');
    candidate
        .strip_prefix(parent)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_nextjs_export_refinement(hint: &str, candidate: &str) -> bool {
    let hint = hint.trim_end_matches('/');
    let candidate = candidate.trim_end_matches('/');
    matches!(
        (hint, candidate),
        (".next", "out") | (".next/standalone", "out")
    )
}

fn is_nextjs_standalone_framework(framework: &str) -> bool {
    matches!(framework, "nextjs" | "blitzjs" | "payload")
}

#[derive(Debug, Clone)]
struct NextStandaloneServer {
    server_dir: std::path::PathBuf,
    server_dir_relative: Option<String>,
    entry: String,
}

#[derive(Debug)]
struct NextStandaloneServerCandidate {
    dir: std::path::PathBuf,
    is_generated_entry: bool,
    has_next_runtime: bool,
    matches_project_name: bool,
}

fn resolve_nextjs_standalone_server(
    project_dir: &Path,
    output_dir: &Path,
) -> Option<NextStandaloneServer> {
    let root_server = output_dir.join("server.js");
    if root_server.is_file() {
        return Some(NextStandaloneServer {
            server_dir: output_dir.to_path_buf(),
            server_dir_relative: None,
            entry: "server.js".to_string(),
        });
    }

    if !is_nextjs_standalone_dir(output_dir) {
        return None;
    }

    let server_dir = find_nested_standalone_server_dir(project_dir, output_dir)?;
    let server_dir_relative = relative_manifest_path(output_dir, &server_dir)?;
    let entry = join_manifest_path(&server_dir_relative, "server.js");

    tracing::info!(
        output_dir = %output_dir.display(),
        server_dir = %server_dir.display(),
        entry,
        "using nested Next.js standalone server entry"
    );

    Some(NextStandaloneServer {
        server_dir,
        server_dir_relative: Some(server_dir_relative),
        entry,
    })
}

fn relative_manifest_path(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(path_to_manifest_string)
        .filter(|path| !path.is_empty())
}

fn path_to_manifest_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn join_manifest_path(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_matches('/');
    let path = path.trim_matches('/');
    if prefix.is_empty() {
        path.to_string()
    } else if path.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{path}")
    }
}

fn is_nextjs_standalone_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "standalone")
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".next")
}

fn find_nested_standalone_server_dir(
    project_dir: &Path,
    output_dir: &Path,
) -> Option<std::path::PathBuf> {
    let mut matches = Vec::new();
    collect_nested_standalone_server_dirs(project_dir, output_dir, 0, &mut matches);

    let generated_app_dirs = matches
        .iter()
        .filter(|candidate| candidate.is_generated_entry && candidate.has_next_runtime)
        .collect::<Vec<_>>();
    if let [single] = generated_app_dirs.as_slice() {
        return Some(single.dir.clone());
    }

    let app_dirs = matches
        .iter()
        .filter(|candidate| candidate.has_next_runtime)
        .collect::<Vec<_>>();
    if let [single] = app_dirs.as_slice() {
        return Some(single.dir.clone());
    }

    let generated = matches
        .iter()
        .filter(|candidate| candidate.is_generated_entry)
        .collect::<Vec<_>>();
    if let [single] = generated.as_slice() {
        return Some(single.dir.clone());
    }

    let project_named = matches
        .iter()
        .filter(|candidate| candidate.matches_project_name)
        .collect::<Vec<_>>();
    if let [single] = project_named.as_slice() {
        return Some(single.dir.clone());
    }

    match matches.as_slice() {
        [single] => Some(single.dir.clone()),
        multiple => {
            if !multiple.is_empty() {
                tracing::warn!(
                    output_dir = %output_dir.display(),
                    count = multiple.len(),
                    "multiple nested Next.js standalone server.js files found; cannot infer entry"
                );
            }
            None
        }
    }
}

fn collect_nested_standalone_server_dirs(
    project_dir: &Path,
    dir: &Path,
    depth: usize,
    matches: &mut Vec<NextStandaloneServerCandidate>,
) {
    if depth > 8 {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| matches!(name, "node_modules" | ".next" | ".onreza"))
            {
                continue;
            }
            collect_nested_standalone_server_dirs(project_dir, &path, depth + 1, matches);
            continue;
        }

        if file_type.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "server.js")
            && let Some(parent) = path.parent()
        {
            matches.push(NextStandaloneServerCandidate {
                dir: parent.to_path_buf(),
                is_generated_entry: looks_like_nextjs_standalone_server(&path),
                has_next_runtime: parent.join(".next").is_dir(),
                matches_project_name: path_file_name_matches(parent, project_dir),
            });
        }
    }
}

fn looks_like_nextjs_standalone_server(path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return false;
    };

    contents.contains("__NEXT_PRIVATE_STANDALONE_CONFIG")
        || contents.contains("next/dist/server/lib/start-server")
}

fn path_file_name_matches(path: &Path, expected: &Path) -> bool {
    let Some(path_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(expected_name) = expected.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    path_name == expected_name
}

/// Prepare Next.js standalone output by copying static assets and public files
/// into the correct directory structure for STATIC + COMPUTE layers.
///
/// Safe to call after a partial run: skips copy steps when the destination directory already exists.
#[cfg(test)]
fn prepare_nextjs_standalone(
    project_dir: &Path,
    output_dir: &Path,
    json: bool,
) -> anyhow::Result<()> {
    prepare_nextjs_standalone_for_server(project_dir, output_dir, output_dir, json)
}

fn prepare_nextjs_standalone_for_server(
    project_dir: &Path,
    bundle_root: &Path,
    server_dir: &Path,
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
        // 1. Copy .next/static/ → {server}/.next/static/ (for server.js)
        let server_static_dst = server_dir.join(".next/static");
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

        // 2. Copy .next/static/ → {server}/_static/_next/static/ (for CDN STATIC layer)
        let cdn_static_dst = server_dir.join("_static/_next/static");
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

    // 3. Copy public/ → {server}/public/ (STATIC layer for root-level assets)
    let public_src = project_dir.join("public");
    if public_src.is_dir() {
        let public_dst = server_dir.join("public");
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

    // 4. Copy metadata route .body files → {server}/public/<name>
    //
    // Next.js App Router compiles static metadata files (favicon.ico, robots.txt,
    // opengraph-image.png, etc.) into .next/server/app/**/*.body. In standalone mode,
    // server.js may return 404 for these routes because it expects a reverse proxy to
    // serve them. Copying the .body files into the STATIC public/ layer ensures they
    // are served from CDN without hitting the COMPUTE layer.
    copy_metadata_routes(server_dir, json)?;

    // 5. Copy missing Prisma external packages into the standalone bundle root node_modules.
    //
    // Prisma 6+ generates a client into `@prisma/client-<hash>` (a content-addressed
    // package). Next.js standalone file tracing may not include these dynamically-named
    // packages, causing runtime "Cannot find module" errors. We detect any `@prisma/client-*`
    // directories in the project's node_modules that are absent from the standalone output
    // and copy them over.
    copy_missing_prisma_packages(project_dir, bundle_root, json)?;

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
