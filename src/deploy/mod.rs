pub(crate) mod bundle;
#[cfg(test)]
mod bundle_tests;
#[cfg(test)]
mod deploy_tests;
pub(crate) mod health_check;
#[cfg(test)]
mod health_check_tests;
pub(crate) mod pack_v1;
#[cfg(test)]
mod pack_v1_tests;

use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use bytes::Bytes;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::{ApiClient, PresignedHeadVerify, PresignedPutHeaders};
use crate::auth;
use crate::build;
use crate::build::manifest as build_manifest;
use crate::cli::{BuildArgs, DeployArgs};
use crate::deploy::pack_v1::{
    CompletedMultipartPart, ComputeBundleUpload, IsolateUploadPlan, MultipartCompleteTarget,
    PackPlan, PresignedUpload, build_compute_bundle_uploads, build_isolate_upload_plan,
    build_static_pack_plan, files_in_dirs, read_file_slice, read_pack_part_bytes,
    read_pack_part_chunk_bytes, static_layer_dirs,
};
use crate::detect::types::ComputeType;
use crate::link;
use crate::output;
use nrz::config::{HealthCheckPathConfig, ProjectConfig};
use uuid::Uuid;

const UPLOAD_COMPLETE_RETRY_BUDGET: Duration = Duration::from_secs(30 * 60);
const UPLOAD_COMPLETE_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const UPLOAD_COMPLETE_MAX_RETRY_DELAY: Duration = Duration::from_secs(5);

// ── Workspace / plan ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    id: String,
    #[allow(dead_code)]
    slug: String,
}

// ── Project settings from server ─────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInfo {
    framework_preset: Option<String>,
    install_command: Option<String>,
    install_command_source: Option<build::BuildSettingSource>,
    build_command: Option<String>,
    build_command_source: Option<build::BuildSettingSource>,
    output_directory: Option<String>,
    output_directory_source: Option<build::BuildSettingSource>,
}

#[derive(Debug, Clone, Copy)]
struct CommandHint<'a> {
    command: Option<&'a str>,
}

fn command_hint<'a>(
    command: Option<&'a str>,
    source: Option<build::BuildSettingSource>,
) -> Option<CommandHint<'a>> {
    let command = command.filter(|v| !v.trim().is_empty());

    match source {
        // PRESET is a fallback/default from the platform, not a user intent.
        // Let local package-manager/script detection decide whether there is
        // anything to run, otherwise static/non-JS repos fail on npm ENOENT.
        Some(build::BuildSettingSource::Preset) => None,
        Some(source) => {
            if command.is_some() || source.is_authoritative_command_absence() {
                Some(CommandHint { command })
            } else {
                None
            }
        }
        // Older APIs did not expose source metadata. Preserve compatibility
        // for explicit command values, but do not treat missing fields as an
        // authoritative absence.
        None => command.map(|command| CommandHint {
            command: Some(command),
        }),
    }
}

fn authoritative_server_framework_preset(preset: Option<&str>) -> Option<&str> {
    let preset = preset?.trim();
    if preset.is_empty() || preset.eq_ignore_ascii_case("other") {
        return None;
    }
    Some(preset)
}

async fn fetch_project_settings(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<ProjectInfo> {
    client
        .get(&format!("/v1/projects/{project_id}"))
        .await
        .context("failed to fetch project settings")
}

// ── API structs ──────────────────────────────────────────────

/// Per-file identity entry used by local PACK_V1 planning and by the
/// deployment-create body. PACK_V1 upload planning converts these files into
/// pack ranges before calling `prepare-upload`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEntry {
    path: String,
    size: u64,
    content_hash: String,
}

/// COMPUTE bundle descriptor sent in the deployment-create body.
///
/// Wire schema: `{ sha256, size }`. PACK_V1 prepare-upload later receives the
/// per-COMPUTE-layer `computeBundles[]` plan and issues the conditioned PUTs.
///
/// `size` is invariably `bytes.len()` of the same buffer hashed into `sha256`;
/// construct via `BundleManifest::of` to keep the two in sync.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleManifest {
    sha256: String,
    size: u64,
}

impl BundleManifest {
    fn of(bytes: &[u8], sha256_hex: &str) -> Self {
        Self {
            sha256: sha256_hex.to_string(),
            size: bytes.len() as u64,
        }
    }
}

/// Body for `POST /v1/projects/:id/deployments`.
///
/// `manifest` and `commitSha` are required by the server (`deployments.ts` body schema:
/// `manifest: ManifestSchema`, `commitSha: z.string().min(1)`). We mirror that on the
/// CLI side so a missing value fails Rust type checks instead of producing a cryptic
/// 400 from zod. `manifest` is filled by the build step (or the auto-gen fallbacks
/// in `run`); `commitSha` falls back to `synthetic_sha(&files)` when git isn't available.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeploymentBody {
    manifest: serde_json::Value,
    files: Vec<FileEntry>,
    production: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    commit_sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle: Option<BundleManifest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeploymentResponse {
    id: String,
    #[allow(dead_code)]
    status: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackPartUploadTarget {
    part_index: u32,
    #[allow(dead_code)]
    object_key: String,
    #[allow(dead_code)]
    bucket: String,
    upload: PresignedUpload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComputeBundleUploadTarget {
    layer_name: String,
    bundle_sha256: String,
    #[allow(dead_code)]
    bucket: String,
    #[allow(dead_code)]
    object_key: String,
    upload: Option<PresignedUpload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IsolateModuleUploadTarget {
    layer_name: String,
    files: Vec<IsolateModuleFileTarget>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IsolateModuleFileTarget {
    path: String,
    sha256: String,
    #[allow(dead_code)]
    bucket: String,
    #[allow(dead_code)]
    object_key: String,
    upload: Option<PresignedUpload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentStatusResponse {
    #[allow(dead_code)]
    id: String,
    status: String,
    url: Option<String>,
    #[allow(dead_code)]
    production: Option<bool>,
    error: Option<String>,
    #[allow(dead_code)]
    created_at: Option<String>,
    #[allow(dead_code)]
    ready_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeployOutput {
    deployment_id: String,
    url: String,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_check: Option<HealthCheckInfo>,
}

/// JSON output for health check configuration.
///
/// Serializes as `{"mode":"http","path":"/health","source":"config"}`
/// or `{"mode":"tcp","source":"default"}` (no `path` field for TCP).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum HealthCheckInfo {
    Http {
        path: String,
        source: HealthCheckSourceTag,
    },
    Tcp {
        source: HealthCheckSourceTag,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum HealthCheckSourceTag {
    Flag,
    Config,
    Detected,
    Default,
}

/// Resolved health check configuration for a PROCESS deployment.
#[derive(Debug, Clone)]
struct ResolvedHealthCheck {
    /// The HTTP path (e.g. `/health`), or `None` for TCP-only.
    path: Option<String>,
    /// Where the value came from.
    source: HealthCheckSource,
}

#[derive(Debug, Clone, Copy)]
enum HealthCheckSource {
    Flag,
    Config,
    Detected,
    Default,
}

impl HealthCheckSource {
    fn to_tag(self) -> HealthCheckSourceTag {
        match self {
            Self::Flag => HealthCheckSourceTag::Flag,
            Self::Config => HealthCheckSourceTag::Config,
            Self::Detected => HealthCheckSourceTag::Detected,
            Self::Default => HealthCheckSourceTag::Default,
        }
    }
}

impl ResolvedHealthCheck {
    fn to_info(&self) -> HealthCheckInfo {
        match &self.path {
            Some(path) => HealthCheckInfo::Http {
                path: path.clone(),
                source: self.source.to_tag(),
            },
            None => HealthCheckInfo::Tcp {
                source: self.source.to_tag(),
            },
        }
    }
}

// ── Main deploy flow ─────────────────────────────────────────

pub async fn run(
    args: DeployArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let mut project_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    // Resolve --app for monorepo: detect workspaces and switch to the target app directory
    // Priority: CLI --app > [deploy] app in onreza.toml
    let effective_app = args.app.as_deref().or(config.deploy_app());
    if let Some(app_name) = effective_app {
        let mono_fs = crate::detect::fs::LocalFs::new(&project_dir);
        let mono_pkg = crate::detect::package_json::PackageJson::load_from_fs(&mono_fs);
        let mono_pm =
            crate::detect::package_manager::detect_package_manager(&mono_fs, mono_pkg.as_ref());
        let mono_info =
            crate::detect::monorepo::detect_monorepo(&mono_fs, mono_pkg.as_ref(), mono_pm.as_ref());

        match mono_info {
            Some(info) => match crate::detect::monorepo::resolve_app(&info, app_name) {
                Some(app_path) => {
                    let resolved = project_dir.join(&app_path);
                    if !resolved.is_dir() {
                        return Err(output::coded_error(
                            "MONOREPO_APP_NOT_FOUND",
                            format!(
                                "resolved app directory does not exist: {}",
                                resolved.display()
                            ),
                        ));
                    }
                    output::status(
                        json,
                        "~",
                        format!("Monorepo: deploying app \"{app_name}\" from {app_path}/"),
                        output::Phase::Deploy,
                    );
                    project_dir = resolved
                        .canonicalize()
                        .with_context(|| format!("failed to resolve app path: {app_path}"))?;
                }
                None => {
                    let available: Vec<String> = info
                        .packages
                        .iter()
                        .map(|p| p.name.as_deref().unwrap_or(&p.path).to_string())
                        .collect();
                    return Err(output::coded_error(
                        "MONOREPO_APP_NOT_FOUND",
                        format!(
                            "app \"{app_name}\" not found in monorepo workspaces.\n\
                             Available packages: {}",
                            if available.is_empty() {
                                "(none resolved)".to_string()
                            } else {
                                available.join(", ")
                            }
                        ),
                    ));
                }
            },
            None => {
                let source = if args.app.is_some() {
                    "--app"
                } else {
                    "[deploy] app in onreza.toml"
                };
                return Err(output::coded_error(
                    "MONOREPO_APP_NOT_FOUND",
                    format!(
                        "{source} was specified but no monorepo detected in {}",
                        project_dir.display()
                    ),
                ));
            }
        }
    }

    // Verify auth early to avoid wasting time on build if token is invalid
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;

    // Early-resolve project_id (without interactive fallback) to fetch server settings
    let early_project_id = args
        .project_id
        .as_deref()
        .or(config.project.id.as_deref())
        .map(String::from);

    // Fetch project settings from server if project_id is known
    let server_settings = if let Some(ref pid) = early_project_id {
        match fetch_project_settings(&client, pid).await {
            Ok(info) => {
                tracing::info!(
                    ?info.build_command,
                    ?info.build_command_source,
                    ?info.install_command,
                    ?info.install_command_source,
                    ?info.output_directory,
                    ?info.output_directory_source,
                    ?info.framework_preset,
                    "fetched project settings from server"
                );
                Some(info)
            }
            Err(e) => {
                // 4xx errors (wrong project ID, no permissions) should abort early
                let err_msg = format!("{e:#}");
                let is_client_error = err_msg.contains("API error (4");
                if is_client_error {
                    return Err(e.context(format!(
                        "failed to fetch settings for project '{pid}'. \
                         Verify the project ID is correct"
                    )));
                }
                // Transient/network errors: warn and continue with local config
                output::warn(
                    json,
                    format!("Could not fetch project settings: {e}. Using local configuration."),
                    output::Phase::Deploy,
                );
                None
            }
        }
    } else {
        None
    };

    let server_install_cmd = server_settings
        .as_ref()
        .and_then(|s| command_hint(s.install_command.as_deref(), s.install_command_source));
    let server_build_cmd = server_settings
        .as_ref()
        .and_then(|s| command_hint(s.build_command.as_deref(), s.build_command_source));
    let server_output_dir = server_settings
        .as_ref()
        .and_then(|s| s.output_directory.as_deref())
        .filter(|v| !v.trim().is_empty());
    let server_output_dir_hint = server_output_dir.map(|path| build::OutputDirectoryHint {
        path,
        source: server_settings
            .as_ref()
            .and_then(|s| s.output_directory_source)
            .unwrap_or(build::BuildSettingSource::Preset),
    });
    let server_framework_preset = server_settings
        .as_ref()
        .and_then(|s| authoritative_server_framework_preset(s.framework_preset.as_deref()));

    // Run install step (default: enabled, skip with --skip-install or --skip-build)
    if !args.skip_build && !args.skip_install {
        run_install_step(&project_dir, json, server_install_cmd)?;
    }

    // Pre-build env injection for framework compatibility.
    let build_env: Vec<(&str, &str)> = {
        let mut env = Vec::new();

        // Next.js: inject NEXT_PRIVATE_STANDALONE=1 so users don't have to manually set
        // `output: 'standalone'` in next.config. No-op when user has `output: 'export'`.
        if is_nextjs_project(&project_dir) {
            output::status(
                json,
                "~",
                "Next.js detected, enabling standalone output (NEXT_PRIVATE_STANDALONE=1)",
                output::Phase::Deploy,
            );
            env.push(("NEXT_PRIVATE_STANDALONE", "1"));
        }

        // SvelteKit: adapter-auto checks platform env vars to pick an adapter.
        // GCP_BUILDPACKS makes it choose adapter-node, which produces the build/
        // output we expect. Without this, adapter-auto fails silently on unknown hosts.
        if is_sveltekit_with_adapter_auto(&project_dir) {
            output::status(
                json,
                "~",
                "SvelteKit adapter-auto detected, enabling adapter-node (GCP_BUILDPACKS=1)",
                output::Phase::Deploy,
            );
            env.push(("GCP_BUILDPACKS", "1"));
        }

        env
    };

    // Run build step (default: enabled, skip with --skip-build)
    if !args.skip_build
        && let Some(cmd) = resolve_build_command(
            args.build_command.as_deref(),
            &project_dir,
            config,
            server_build_cmd,
        )
    {
        run_build_step(&cmd, &project_dir, json, &build_env)?;
    }

    // Detect framework once — shared by build (output dir search) and deploy (compute type).
    // Server project settings are authoritative for builder-driven deploys;
    // local onreza.toml is the fallback for direct CLI deploys.
    let framework_override = server_framework_preset.or(config.project.framework.as_deref());
    let detection = crate::detect::detect_with_framework_override(&project_dir, framework_override);

    // Validate build output
    output::status(
        json,
        "~",
        "Validating build output...",
        output::Phase::Deploy,
    );
    let build_result = build::run_with_hint(
        BuildArgs {
            dir: project_dir.to_string_lossy().into_owned(),
            skip_validation: false,
        },
        json,
        config,
        Some(&detection),
        server_output_dir_hint,
    )
    .await?;

    let output_dir = build_result.output_dir;
    let loaded_manifest = build_result.manifest;
    let has_manifest = loaded_manifest.is_some();

    // Scan output directory into a flat file list with streaming SHA-256 + size per
    // file. Pre-compression is gone (RFC: EDGE_DYNAMIC_ENCODING) — the edge serves
    // identity bytes and compresses on the fly, so the CLI only ships raw content
    // addressed by sha256. Run in a blocking task: filesystem I/O + CPU-bound hashing.
    output::status(
        json,
        "~",
        "Scanning output directory...",
        output::Phase::Deploy,
    );
    let output_dir_for_scan = output_dir.clone();
    let files = tokio::task::spawn_blocking(move || scan_dir(&output_dir_for_scan))
        .await
        .context("file scan task failed (panic or runtime shutdown)")??;
    if files.is_empty() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("output directory is empty: {}", output_dir.display()),
        ));
    }

    // Resolve compute type: CLI flag > config > manifest layers > detect
    let compute = if let Some(ref m) = loaded_manifest {
        if args.compute.is_none() && config.deploy_compute().is_none() {
            match build_manifest::primary_compute_target(m) {
                build_manifest::LayerTarget::Compute => ComputeType::Process,
                build_manifest::LayerTarget::Isolate => ComputeType::Isolate,
                build_manifest::LayerTarget::Static => ComputeType::Static,
            }
        } else {
            resolve_compute_type(args.compute.as_deref(), config.deploy_compute(), &detection)?
        }
    } else {
        resolve_compute_type(args.compute.as_deref(), config.deploy_compute(), &detection)?
    };
    let mut warnings: Vec<String> = Vec::new();

    // Inform about SSR framework compute mode when auto-detected
    if !has_manifest
        && args.compute.is_none()
        && config.deploy_compute().is_none()
        && crate::detect::presets::is_ssr_framework(&detection.framework)
    {
        let msg = match compute {
            ComputeType::Process => {
                let mut m = format!("{} deploying as PROCESS (server runtime).", detection.name);
                let hint = framework_static_hint(&detection.framework);
                if !hint.is_empty() {
                    m.push_str(&format!(
                        " For a fully static export, {hint} and redeploy with --compute static."
                    ));
                }
                m
            }
            ComputeType::Static => format!(
                "{} deploying as STATIC. \
                 For server-side rendering, use --compute process.",
                detection.name
            ),
            _ => format!("{} deploying as {}.", detection.name, compute),
        };
        output::warn(json, &msg, output::Phase::Deploy);
        warnings.push(msg);
    }

    let is_process = compute == ComputeType::Process;

    // manifest_raw starts from build result (may already be Some for STATIC auto-gen)
    let mut manifest_raw: Option<serde_json::Value> = loaded_manifest
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .context("failed to serialize manifest")?;

    // When a manifest is present, process entry comes from the manifest's COMPUTE layer —
    // skip pre-flight validation and auto-detection.
    // When no manifest: find entry and auto-generate COMPUTE manifest.
    if is_process && !has_manifest {
        validate_process_output(&output_dir, &project_dir, &detection)
            .map_err(|e| output::with_default_code(e, "MISSING_PROCESS_ENTRY"))?;
        let (entry, warning) = ensure_process_entry(
            &output_dir,
            &project_dir,
            config.deploy_entry(),
            &detection,
            json,
        )
        .map_err(|e| output::with_default_code(e, "MISSING_PROCESS_ENTRY"))?;
        if let Some(ref w) = warning {
            output::warn(json, w, output::Phase::Deploy);
            warnings.push(w.clone());
        }
        match entry {
            Some(ref e) => {
                let auto = build_manifest::generate_compute_manifest(e);
                output::status(
                    json,
                    "~",
                    format!("Auto-generated COMPUTE manifest (entry: {e})"),
                    output::Phase::Deploy,
                );
                manifest_raw = Some(
                    serde_json::to_value(&auto)
                        .context("failed to serialize auto-generated manifest")?,
                );
            }
            None => {
                return Err(output::coded_error(
                    "MISSING_PROCESS_ENTRY",
                    format!(
                        "Cannot auto-generate COMPUTE manifest: entry point not detected in {}.\n\n\
                         Create .onreza/manifest.json manually.\n\
                         See: docs.onreza.ru/manifest",
                        output_dir.display()
                    ),
                ));
            }
        }
    }

    let manifest_raw = resolve_manifest_for_compute(compute, manifest_raw, &detection)?;
    let manifest_for_planning: build_manifest::Manifest =
        serde_json::from_value(manifest_raw.clone())
            .context("failed to parse resolved deployment manifest")?;
    let has_compute_layer = manifest_has_compute_layer(&manifest_for_planning);

    // Resolve health check path (PROCESS only)
    let health_check = if has_compute_layer {
        Some(resolve_health_check(
            args.health_check_path.as_deref(),
            config,
            &project_dir,
            &detection,
            &output_dir,
            json,
        )?)
    } else {
        None
    };

    // PACK_V1 prepare-upload needs the authenticated workspace ID for all entry
    // points (direct CLI deploy and builder resume). Admission/limits are enforced
    // server-side from the complete STATIC + COMPUTE + ISOLATE upload plan.
    let ws_info: WorkspaceInfo = client
        .get("/v1/workspace")
        .await
        .context("failed to fetch workspace info")?;

    // ── Resume mode: builder calls us with an existing deployment ID ──
    if let Some(deployment_id) = &args.resume_deployment {
        let deployment_id = deployment_id.trim();
        if deployment_id.is_empty() {
            return Err(output::coded_error(
                "INVALID_ARGUMENT",
                "--resume-deployment requires a non-empty deployment ID".to_string(),
            ));
        }
        let project_id = resolve_project_id_for_resume(
            &client,
            deployment_id,
            args.project_id.as_deref(),
            config,
        )
        .await?;
        return resume_deploy(
            &client,
            deployment_id,
            &ws_info.id,
            &project_id,
            manifest_raw,
            manifest_for_planning,
            files,
            &output_dir,
            json,
            warnings,
        )
        .await;
    }

    // ── Normal flow continues below ─────────────────────────────────

    // Resolve project: --project-id > onreza.toml > interactive
    let project_id = if let Some(pid) = &args.project_id {
        pid.clone()
    } else if let Some(id) = &config.project.id {
        id.clone()
    } else {
        if json {
            bail!(
                "no linked project. Use --project-id, set [project] id in onreza.toml, or run `nrz link` first."
            );
        }
        output::warn(
            false,
            "No linked project. Select one:",
            output::Phase::Deploy,
        );
        let selected = link::select_project_interactive(&client).await?;
        nrz::config::save_or_update(
            &project_dir,
            &selected.project_id,
            Some(&selected.project_name),
            None,
        )?;
        crate::init::add_to_gitignore(&project_dir);
        output::success(
            false,
            format!(
                "Linked to {}",
                console::style(&selected.project_name).bold()
            ),
            output::Phase::Deploy,
        );
        selected.project_id
    };

    // Git info
    let branch = git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit_sha = git_cmd(&["rev-parse", "HEAD"]).unwrap_or_else(|| {
        output::warn(
            json,
            "git not available, using synthetic commit SHA",
            output::Phase::Deploy,
        );
        synthetic_sha(&files)
    });

    // Validate required env vars from [env] declarations
    if !args.skip_env_check {
        crate::cli::env_handler::validate_env_for_deploy(&client, &project_id, json, config)
            .await?;
    }

    // Sync detection results to API (best-effort, non-blocking)
    let sync_client = client.clone();
    let sync_project_id = project_id.clone();
    let _sync = tokio::spawn(async move {
        crate::detect_sync::sync_detection_to_api(&sync_client, &sync_project_id, &detection).await;
    });

    // Sync compute config (health check path) for PROCESS deployments
    if let Some(ref hc) = health_check {
        let hc_client = client.clone();
        let hc_project_id = project_id.clone();
        let hc_clone = hc.clone();
        let _hc = tokio::spawn(async move {
            sync_compute_config(&hc_client, &hc_project_id, &hc_clone, json).await;
        });
    }

    // Create artifact plan for PACK_V1. STATIC content is packed into ordered
    // pack parts; COMPUTE and ISOLATE targets are first-class upload targets in
    // the same prepare-upload session.
    let bundle_data = maybe_create_bundle(&output_dir, has_compute_layer, json)?;
    let manifest_for_api = bind_compute_bundle_to_manifest_value(
        manifest_raw,
        bundle_data.as_ref().map(|(_, sha)| sha.as_str()),
    )?;
    let upload_plan = build_pack_v1_upload_plan(
        &output_dir,
        &manifest_for_planning,
        &files,
        bundle_data.as_ref(),
    )
    .context("failed to prepare PACK_V1 upload plan")?;
    let deployment_attempt_id = Uuid::now_v7().to_string();

    // Create deployment
    output::status(json, "~", "Creating deployment...", output::Phase::Deploy);
    let body = CreateDeploymentBody {
        manifest: manifest_for_api.clone(),
        files: files.clone(),
        production: args.prod,
        branch,
        commit_sha,
        bundle: bundle_data
            .as_ref()
            .map(|(bytes, sha)| BundleManifest::of(bytes, sha)),
    };

    let deployment: CreateDeploymentResponse = client
        .post(&format!("/v1/projects/{}/deployments", project_id), &body)
        .await
        .context("failed to create deployment")?;

    prepare_upload_and_complete(
        &client,
        &deployment.id,
        &ws_info.id,
        &project_id,
        &deployment_attempt_id,
        Some(manifest_for_api),
        &output_dir,
        json,
        upload_plan,
        bundle_data.as_ref(),
    )
    .await?;

    // Poll for ready status
    let spinner = make_spinner(json, "Deploying...");
    let poll_interval = std::time::Duration::from_secs(3);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);

    loop {
        tokio::time::sleep(poll_interval).await;

        if tokio::time::Instant::now() >= deadline {
            finish_spinner(spinner, "");
            bail!("deployment timed out after 120s");
        }

        let status: DeploymentStatusResponse = client
            .get(&format!("/v1/deployments/{}/status", deployment.id))
            .await
            .context("failed to check deployment status")?;

        match status.status.as_str() {
            "live" => {
                finish_spinner(spinner, "");
                let url = status.url.as_deref().unwrap_or(&deployment.url);

                if json {
                    output::json_output(&DeployOutput {
                        deployment_id: deployment.id,
                        url: url.to_string(),
                        status: "live".into(),
                        warnings,
                        health_check: health_check.as_ref().map(|hc| hc.to_info()),
                    });
                } else {
                    eprintln!();
                    eprintln!(
                        "  {} Deployed to {}",
                        console::style("✓").green().bold(),
                        console::style(url).underlined().bold(),
                    );
                    eprintln!();
                }
                return Ok(());
            }
            "failed" => {
                finish_spinner(spinner, "");
                let msg = status.error.unwrap_or_else(|| "unknown error".into());
                bail!("deployment failed: {msg}");
            }
            other => {
                if let Some(ref s) = spinner {
                    s.set_message(format!("Status: {other}..."));
                }
                continue;
            }
        }
    }
}

// ── Health check resolution ──────────────────────────────────

/// Resolve health check path for PROCESS deployments.
///
/// Priority: CLI flag > config > autodetect > TCP default.
fn resolve_health_check(
    cli_flag: Option<&str>,
    config: &ProjectConfig,
    project_dir: &Path,
    detection: &crate::detect::types::DetectionResult,
    output_dir: &Path,
    json: bool,
) -> anyhow::Result<ResolvedHealthCheck> {
    // 1. CLI flag
    if let Some(flag) = cli_flag {
        if flag.eq_ignore_ascii_case("none")
            || flag.eq_ignore_ascii_case("false")
            || flag.eq_ignore_ascii_case("tcp")
        {
            output::success(
                json,
                "Health check: TCP (from --health-check-path)",
                output::Phase::Deploy,
            );
            return Ok(ResolvedHealthCheck {
                path: None,
                source: HealthCheckSource::Flag,
            });
        }
        validate_health_path(flag, "--health-check-path")?;
        output::success(
            json,
            format!("Health check: HTTP {flag} (from --health-check-path)"),
            output::Phase::Deploy,
        );
        return Ok(ResolvedHealthCheck {
            path: Some(flag.to_string()),
            source: HealthCheckSource::Flag,
        });
    }

    // 2. Config
    if let Some(hc) = config.health_check_path() {
        match hc {
            HealthCheckPathConfig::Tcp => {
                output::success(
                    json,
                    "Health check: TCP (configured)",
                    output::Phase::Deploy,
                );
                return Ok(ResolvedHealthCheck {
                    path: None,
                    source: HealthCheckSource::Config,
                });
            }
            HealthCheckPathConfig::Http(path) => {
                output::success(
                    json,
                    format!("Health check: HTTP {path} (from config)"),
                    output::Phase::Deploy,
                );
                return Ok(ResolvedHealthCheck {
                    path: Some(path.clone()),
                    source: HealthCheckSource::Config,
                });
            }
        }
    }

    // 3. Autodetect
    if let Some(det) =
        health_check::detect_health_path(project_dir, &detection.framework, output_dir)
    {
        output::success(
            json,
            format!(
                "Found health endpoint: {} (source: {})",
                det.path, det.source_description
            ),
            output::Phase::Deploy,
        );
        return Ok(ResolvedHealthCheck {
            path: Some(det.path),
            source: HealthCheckSource::Detected,
        });
    }

    // 4. Default: TCP
    output::status(
        json,
        "ℹ",
        "No health check endpoint detected. Using TCP readiness check.\n    \
         To add HTTP health check, create a /health endpoint or set\n    \
         deploy.health_check_path in onreza.toml",
        output::Phase::Deploy,
    );
    Ok(ResolvedHealthCheck {
        path: None,
        source: HealthCheckSource::Default,
    })
}

/// Validate an HTTP health check path.
fn validate_health_path(path: &str, source: &str) -> anyhow::Result<()> {
    if !path.starts_with('/') {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must start with '/', got: \"{path}\""),
        ));
    }
    if path.contains("..") {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must not contain '..', got: \"{path}\""),
        ));
    }
    if path.contains('?') || path.contains('#') {
        return Err(output::coded_error(
            "INVALID_ARGUMENT",
            format!("{source} must not contain query or fragment, got: \"{path}\""),
        ));
    }
    Ok(())
}

// ── Compute config sync ──────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ComputeConfigBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    health_check_path: Option<String>,
}

/// Best-effort sync of compute config (health check path) to the platform.
async fn sync_compute_config(
    client: &ApiClient,
    project_id: &str,
    health_check: &ResolvedHealthCheck,
    json: bool,
) {
    let body = ComputeConfigBody {
        health_check_path: health_check.path.clone(),
    };

    let path = format!("/v1/compute-config/{project_id}");
    let resp: Result<serde_json::Value, _> = client.put(&path, &body).await;
    if let Err(e) = resp {
        output::warn(
            json,
            format!("failed to sync compute config: {e}"),
            output::Phase::Deploy,
        );
    }
}

// ── Shared upload step ───────────────────────────────────────

struct PackV1UploadPlan {
    static_pack: PackPlan,
    compute_bundles: Vec<ComputeBundleUpload>,
    isolate: IsolateUploadPlan,
}

fn build_pack_v1_upload_plan(
    output_dir: &Path,
    manifest: &build_manifest::Manifest,
    files: &[FileEntry],
    bundle_data: Option<&(Vec<u8>, String)>,
) -> anyhow::Result<PackV1UploadPlan> {
    let static_dirs = static_layer_dirs(manifest);
    let static_files = files_in_dirs(files, &static_dirs);
    let static_pack = build_static_pack_plan(output_dir, &static_files)?;
    let isolate = build_isolate_upload_plan(output_dir, manifest, files)?;

    let compute_bundles = if let Some((bytes, sha)) = bundle_data {
        build_compute_bundle_uploads(manifest, sha, bytes.len() as u64, Some(bytes))?
    } else {
        if manifest_has_compute_layer(manifest) {
            bail!("COMPUTE manifest requires a tar.zst bundle upload plan");
        }
        Vec::new()
    };

    Ok(PackV1UploadPlan {
        static_pack,
        compute_bundles,
        isolate,
    })
}

/// Drive the PACK_V1 upload protocol:
/// prepare-upload → S3 PUTs → multipart-complete (if needed) → upload-complete.
#[allow(clippy::too_many_arguments)]
async fn prepare_upload_and_complete(
    client: &ApiClient,
    deployment_id: &str,
    workspace_id: &str,
    project_id: &str,
    deployment_attempt_id: &str,
    manifest: Option<serde_json::Value>,
    output_dir: &Path,
    json: bool,
    plan: PackV1UploadPlan,
    bundle_data: Option<&(Vec<u8>, String)>,
) -> anyhow::Result<()> {
    output::status(
        json,
        "~",
        "Preparing PACK_V1 upload...",
        output::Phase::Deploy,
    );

    let body = PrepareUploadBody {
        deployment_id: deployment_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: project_id.to_string(),
        deployment_attempt_id: deployment_attempt_id.to_string(),
        operation_id: Uuid::now_v7().to_string(),
        manifest,
        manifest_summary: plan.static_pack.summary.clone(),
        compute_bundles: plan.compute_bundles.clone(),
        isolate_modules: plan.isolate.modules.clone(),
    };

    let prepared: PrepareUploadResponse = match client
        .post(
            &format!("/v1/deployments/{deployment_id}/prepare-upload"),
            &body,
        )
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            if json
                && let Some(api_err) = e.downcast_ref::<crate::api::StructuredApiError>()
                && (api_err.code == "LIMIT_EXCEEDED" || api_err.code == "SUBSCRIPTION_REQUIRED")
            {
                output::log_error_structured(
                    "deploy",
                    &api_err.message,
                    &api_err.code,
                    api_err.details.as_ref(),
                );
            }
            return Err(e.context("failed to prepare upload"));
        }
    };

    let mut multipart_targets = Vec::new();

    if let PrepareUploadResponse::ColdPath { pack_parts, .. } = &prepared {
        upload_pack_parts(
            client,
            output_dir,
            &plan.static_pack,
            pack_parts,
            json,
            &mut multipart_targets,
        )
        .await?;
    }

    upload_compute_bundle_targets(
        client,
        prepared.compute_bundle_targets(),
        bundle_data,
        json,
        &mut multipart_targets,
    )
    .await?;
    upload_isolate_module_targets(
        client,
        output_dir,
        &plan.isolate,
        prepared.isolate_module_targets(),
        json,
        &mut multipart_targets,
    )
    .await?;

    if !multipart_targets.is_empty() {
        let body = MultipartCompleteBody {
            deployment_id: deployment_id.to_string(),
            upload_session_id: prepared.upload_session_id().to_string(),
            deployment_attempt_id: deployment_attempt_id.to_string(),
            operation_id: Uuid::now_v7().to_string(),
            targets: multipart_targets,
        };
        let _: MultipartCompleteResponse = client
            .post(
                &format!("/v1/deployments/{deployment_id}/multipart-complete"),
                &body,
            )
            .await
            .context("failed to complete multipart uploads")?;
    }

    complete_upload_with_retry(
        client,
        deployment_id,
        prepared.upload_session_id(),
        deployment_attempt_id,
        json,
    )
    .await?;

    Ok(())
}

// ── Resume deploy flow ───────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrepareUploadBody {
    deployment_id: String,
    workspace_id: String,
    project_id: String,
    deployment_attempt_id: String,
    operation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<serde_json::Value>,
    manifest_summary: pack_v1::ManifestSummary,
    compute_bundles: Vec<ComputeBundleUpload>,
    isolate_modules: Vec<pack_v1::IsolateModuleUpload>,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum PrepareUploadResponse {
    FastPath {
        upload_session_id: String,
        #[allow(dead_code)]
        manifest_key: String,
        #[allow(dead_code)]
        manifest_sha256: String,
        #[allow(dead_code)]
        logical_bytes_reserved: String,
        compute_bundle_targets: Vec<ComputeBundleUploadTarget>,
        isolate_module_targets: Vec<IsolateModuleUploadTarget>,
    },
    Waiting {
        upload_session_id: String,
        #[allow(dead_code)]
        owner_session_id: String,
        #[allow(dead_code)]
        expires_at: String,
        compute_bundle_targets: Vec<ComputeBundleUploadTarget>,
        isolate_module_targets: Vec<IsolateModuleUploadTarget>,
    },
    ColdPath {
        upload_session_id: String,
        #[allow(dead_code)]
        expires_at: String,
        #[allow(dead_code)]
        manifest_key: String,
        #[allow(dead_code)]
        manifest_sha256: String,
        #[allow(dead_code)]
        bucket: String,
        pack_parts: Vec<PackPartUploadTarget>,
        compute_bundle_targets: Vec<ComputeBundleUploadTarget>,
        isolate_module_targets: Vec<IsolateModuleUploadTarget>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadCompleteBody {
    deployment_id: String,
    upload_session_id: String,
    deployment_attempt_id: String,
    operation_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum UploadCompleteResponse {
    IngestStarted {
        #[allow(dead_code)]
        deployment_id: String,
        #[allow(dead_code)]
        upload_session_id: String,
    },
    FastPathCompleted {
        #[allow(dead_code)]
        deployment_id: String,
        #[allow(dead_code)]
        upload_session_id: String,
    },
    Expired {
        #[allow(dead_code)]
        deployment_id: String,
        #[allow(dead_code)]
        expired_at: String,
    },
    Incomplete {
        #[allow(dead_code)]
        missing_part_indexes: Vec<u32>,
    },
    #[serde(rename = "noop_already_completed")]
    NoopAlreadyCompleted {
        #[allow(dead_code)]
        deployment_id: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MultipartCompleteBody {
    deployment_id: String,
    upload_session_id: String,
    deployment_attempt_id: String,
    operation_id: String,
    targets: Vec<MultipartCompleteTarget>,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum MultipartCompleteResponse {
    Completed {
        #[allow(dead_code)]
        deployment_id: String,
        #[allow(dead_code)]
        upload_session_id: String,
        #[allow(dead_code)]
        completed_targets: u32,
    },
    #[serde(rename = "noop_already_completed")]
    NoopAlreadyCompleted {
        #[allow(dead_code)]
        deployment_id: String,
    },
}

impl PrepareUploadResponse {
    fn upload_session_id(&self) -> &str {
        match self {
            Self::FastPath {
                upload_session_id, ..
            }
            | Self::Waiting {
                upload_session_id, ..
            }
            | Self::ColdPath {
                upload_session_id, ..
            } => upload_session_id,
        }
    }

    fn compute_bundle_targets(&self) -> &[ComputeBundleUploadTarget] {
        match self {
            Self::FastPath {
                compute_bundle_targets,
                ..
            }
            | Self::Waiting {
                compute_bundle_targets,
                ..
            }
            | Self::ColdPath {
                compute_bundle_targets,
                ..
            } => compute_bundle_targets,
        }
    }

    fn isolate_module_targets(&self) -> &[IsolateModuleUploadTarget] {
        match self {
            Self::FastPath {
                isolate_module_targets,
                ..
            }
            | Self::Waiting {
                isolate_module_targets,
                ..
            }
            | Self::ColdPath {
                isolate_module_targets,
                ..
            } => isolate_module_targets,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UploadCompleteRetryReason {
    S3Visibility,
    OwnerVerifyInProgress,
}

impl UploadCompleteRetryReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::S3Visibility => "S3 objects are not visible yet",
            Self::OwnerVerifyInProgress => "owner verification is still in progress",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UploadCompleteAttempt {
    Terminal,
    Retry(UploadCompleteRetryReason),
}

async fn complete_upload_with_retry(
    client: &ApiClient,
    deployment_id: &str,
    upload_session_id: &str,
    deployment_attempt_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut delay = UPLOAD_COMPLETE_INITIAL_RETRY_DELAY;
    let mut attempts = 0u32;

    loop {
        attempts += 1;
        match post_upload_complete_once(
            client,
            deployment_id,
            upload_session_id,
            deployment_attempt_id,
        )
        .await?
        {
            UploadCompleteAttempt::Terminal => return Ok(()),
            UploadCompleteAttempt::Retry(reason) => {
                let elapsed = started.elapsed();
                if elapsed >= UPLOAD_COMPLETE_RETRY_BUDGET {
                    bail!(
                        "upload-complete did not reach a terminal state after {:?} (last state: {})",
                        UPLOAD_COMPLETE_RETRY_BUDGET,
                        reason.as_str()
                    );
                }

                if attempts == 1 {
                    output::status(
                        json,
                        "~",
                        format!("Waiting for upload-complete ({})...", reason.as_str()),
                        output::Phase::Deploy,
                    );
                }

                let sleep_for = delay.min(UPLOAD_COMPLETE_RETRY_BUDGET.saturating_sub(elapsed));
                tokio::time::sleep(sleep_for).await;
                delay = delay.saturating_mul(2).min(UPLOAD_COMPLETE_MAX_RETRY_DELAY);
            }
        }
    }
}

async fn post_upload_complete_once(
    client: &ApiClient,
    deployment_id: &str,
    upload_session_id: &str,
    deployment_attempt_id: &str,
) -> anyhow::Result<UploadCompleteAttempt> {
    let body = UploadCompleteBody {
        deployment_id: deployment_id.to_string(),
        upload_session_id: upload_session_id.to_string(),
        deployment_attempt_id: deployment_attempt_id.to_string(),
        operation_id: Uuid::now_v7().to_string(),
    };

    match client
        .post::<_, UploadCompleteResponse>(
            &format!("/v1/deployments/{deployment_id}/upload-complete"),
            &body,
        )
        .await
    {
        Ok(response) => classify_upload_complete_response(response),
        Err(error) => match classify_upload_complete_retry_error(&error) {
            Some(reason) => Ok(UploadCompleteAttempt::Retry(reason)),
            None => Err(error.context("failed to signal upload complete")),
        },
    }
}

fn classify_upload_complete_response(
    response: UploadCompleteResponse,
) -> anyhow::Result<UploadCompleteAttempt> {
    match response {
        UploadCompleteResponse::IngestStarted { .. }
        | UploadCompleteResponse::FastPathCompleted { .. }
        | UploadCompleteResponse::NoopAlreadyCompleted { .. } => {
            Ok(UploadCompleteAttempt::Terminal)
        }
        UploadCompleteResponse::Incomplete { .. } => Ok(UploadCompleteAttempt::Retry(
            UploadCompleteRetryReason::S3Visibility,
        )),
        UploadCompleteResponse::Expired { expired_at, .. } => {
            bail!("upload window expired at {expired_at}; create a new deployment and upload again")
        }
    }
}

fn classify_upload_complete_retry_error(
    error: &anyhow::Error,
) -> Option<UploadCompleteRetryReason> {
    let api_error = error.downcast_ref::<crate::api::StructuredApiError>()?;
    match api_error.code.as_str() {
        "OPERATION_IN_PROGRESS" => Some(UploadCompleteRetryReason::OwnerVerifyInProgress),
        "VALIDATION_ERROR"
            if api_error
                .message
                .to_ascii_lowercase()
                .contains("upload is incomplete")
                && api_error
                    .details
                    .as_ref()
                    .and_then(|details| details.get("field"))
                    .and_then(serde_json::Value::as_str)
                    == Some("packParts") =>
        {
            Some(UploadCompleteRetryReason::S3Visibility)
        }
        _ => None,
    }
}

async fn upload_pack_parts(
    client: &ApiClient,
    output_dir: &Path,
    plan: &PackPlan,
    targets: &[PackPartUploadTarget],
    json: bool,
    multipart_targets: &mut Vec<MultipartCompleteTarget>,
) -> anyhow::Result<()> {
    if targets.is_empty() {
        return Ok(());
    }

    let spinner = make_spinner(
        json,
        &format!(
            "Uploading {} PACK part(s) ({})...",
            targets.len(),
            format_u64_bytes(plan.total_logical_bytes)
        ),
    );

    for (idx, target) in targets.iter().enumerate() {
        let part = plan
            .parts
            .iter()
            .find(|part| part.part_index == target.part_index)
            .with_context(|| {
                format!(
                    "server requested unknown PACK part index {}",
                    target.part_index
                )
            })?;
        if let Some(s) = &spinner {
            s.set_message(format!(
                "[{}/{}] PACK part {}",
                idx + 1,
                targets.len(),
                target.part_index
            ));
        }

        match &target.upload {
            PresignedUpload::Single {
                url,
                content_length,
                sha256,
                verify_head,
                headers,
            } => {
                let bytes = read_pack_part_bytes(output_dir, part).await?;
                let headers = headers.require_if_none_match_any()?;
                upload_single_put(
                    client,
                    SinglePutUpload {
                        url,
                        bytes,
                        content_length: *content_length,
                        sha256,
                        headers: &headers,
                        verify_head: verify_head.as_ref(),
                        label: format!("PACK part {}", target.part_index),
                    },
                )
                .await?;
            }
            PresignedUpload::Multipart {
                upload_id,
                chunk_size,
                chunks,
            } => {
                let parts = upload_multipart_chunks(
                    client,
                    chunks,
                    *chunk_size,
                    &format!("PACK part {}", target.part_index),
                    |offset, size| read_pack_part_chunk_bytes(output_dir, part, offset, size),
                )
                .await?;
                multipart_targets.push(MultipartCompleteTarget::PackPart {
                    part_index: target.part_index,
                    upload_id: upload_id.clone(),
                    parts,
                });
            }
        }
    }

    finish_spinner(
        spinner,
        &format!(
            "Uploaded {} PACK part(s) ({})",
            targets.len(),
            format_u64_bytes(plan.total_logical_bytes)
        ),
    );
    Ok(())
}

async fn upload_compute_bundle_targets(
    client: &ApiClient,
    targets: &[ComputeBundleUploadTarget],
    bundle_data: Option<&(Vec<u8>, String)>,
    json: bool,
    multipart_targets: &mut Vec<MultipartCompleteTarget>,
) -> anyhow::Result<()> {
    if targets.is_empty() {
        return Ok(());
    }
    let (bundle_bytes, bundle_sha) = bundle_data
        .with_context(|| "server returned COMPUTE bundle targets but no local bundle was built")?;
    let bundle = Bytes::from(bundle_bytes.clone());

    let spinner = make_spinner(
        json,
        &format!(
            "Uploading {} COMPUTE bundle target(s) ({})...",
            targets.len(),
            format_bytes(bundle.len())
        ),
    );

    for (idx, target) in targets.iter().enumerate() {
        if target.bundle_sha256 != *bundle_sha {
            bail!(
                "server requested bundle SHA {} for layer {}, but local bundle SHA is {}",
                target.bundle_sha256,
                target.layer_name,
                bundle_sha
            );
        }
        if let Some(s) = &spinner {
            s.set_message(format!(
                "[{}/{}] COMPUTE bundle {}",
                idx + 1,
                targets.len(),
                target.layer_name
            ));
        }

        let Some(upload) = &target.upload else {
            continue;
        };

        match upload {
            PresignedUpload::Single {
                url,
                content_length,
                sha256,
                verify_head,
                headers,
            } => {
                upload_single_put(
                    client,
                    SinglePutUpload {
                        url,
                        bytes: bundle.clone(),
                        content_length: *content_length,
                        sha256,
                        headers,
                        verify_head: verify_head.as_ref(),
                        label: format!("COMPUTE bundle {}", target.layer_name),
                    },
                )
                .await?;
            }
            PresignedUpload::Multipart {
                upload_id,
                chunk_size,
                chunks,
            } => {
                let parts = upload_multipart_chunks(
                    client,
                    chunks,
                    *chunk_size,
                    &format!("COMPUTE bundle {}", target.layer_name),
                    |offset, size| {
                        let bundle = bundle.clone();
                        async move {
                            let start =
                                usize::try_from(offset).context("bundle chunk offset too large")?;
                            let end = usize::try_from(offset + size)
                                .context("bundle chunk end too large")?;
                            if end > bundle.len() {
                                bail!(
                                    "bundle chunk range [{offset}, {}) exceeds bundle size {}",
                                    offset + size,
                                    bundle.len()
                                );
                            }
                            Ok(bundle.slice(start..end))
                        }
                    },
                )
                .await?;
                multipart_targets.push(MultipartCompleteTarget::ComputeBundle {
                    layer_name: target.layer_name.clone(),
                    bundle_sha256: target.bundle_sha256.clone(),
                    upload_id: upload_id.clone(),
                    parts,
                });
            }
        }
    }

    finish_spinner(
        spinner,
        &format!(
            "Uploaded {} COMPUTE bundle target(s)",
            targets
                .iter()
                .filter(|target| target.upload.is_some())
                .count()
        ),
    );
    Ok(())
}

async fn upload_isolate_module_targets(
    client: &ApiClient,
    output_dir: &Path,
    plan: &IsolateUploadPlan,
    targets: &[IsolateModuleUploadTarget],
    json: bool,
    multipart_targets: &mut Vec<MultipartCompleteTarget>,
) -> anyhow::Result<()> {
    if targets.is_empty() {
        return Ok(());
    }

    let file_count: usize = targets.iter().map(|target| target.files.len()).sum();
    let spinner = make_spinner(
        json,
        &format!("Uploading {file_count} ISOLATE module file(s)..."),
    );
    let mut uploaded = 0usize;

    for target in targets {
        for file in &target.files {
            uploaded += 1;
            if let Some(s) = &spinner {
                s.set_message(format!(
                    "[{uploaded}/{file_count}] ISOLATE {}:{}",
                    target.layer_name, file.path
                ));
            }

            let local_path = plan
                .local_path_for_target(&target.layer_name, &file.path, &file.sha256)
                .with_context(|| {
                    format!(
                        "server requested unknown ISOLATE module file {}:{} ({})",
                        target.layer_name, file.path, file.sha256
                    )
                })?
                .to_string();
            let full_path = output_dir.join(&local_path);

            let Some(upload) = &file.upload else {
                continue;
            };

            match upload {
                PresignedUpload::Single {
                    url,
                    content_length,
                    sha256,
                    verify_head,
                    headers,
                } => {
                    let bytes = Bytes::from(
                        tokio::fs::read(&full_path)
                            .await
                            .with_context(|| format!("failed to read {}", full_path.display()))?,
                    );
                    upload_single_put(
                        client,
                        SinglePutUpload {
                            url,
                            bytes,
                            content_length: *content_length,
                            sha256,
                            headers,
                            verify_head: verify_head.as_ref(),
                            label: format!("ISOLATE module {}:{}", target.layer_name, file.path),
                        },
                    )
                    .await?;
                }
                PresignedUpload::Multipart {
                    upload_id,
                    chunk_size,
                    chunks,
                } => {
                    let parts = upload_multipart_chunks(
                        client,
                        chunks,
                        *chunk_size,
                        &format!("ISOLATE module {}:{}", target.layer_name, file.path),
                        |offset, size| read_file_slice(&full_path, offset, size),
                    )
                    .await?;
                    multipart_targets.push(MultipartCompleteTarget::IsolateModule {
                        layer_name: target.layer_name.clone(),
                        path: file.path.clone(),
                        upload_id: upload_id.clone(),
                        parts,
                    });
                }
            }
        }
    }

    finish_spinner(
        spinner,
        &format!("Uploaded {file_count} ISOLATE module file(s)"),
    );
    Ok(())
}

struct SinglePutUpload<'a> {
    url: &'a str,
    bytes: Bytes,
    content_length: u64,
    sha256: &'a str,
    headers: &'a PresignedPutHeaders,
    verify_head: Option<&'a PresignedHeadVerify>,
    label: String,
}

async fn upload_single_put(client: &ApiClient, upload: SinglePutUpload<'_>) -> anyhow::Result<()> {
    verify_upload_payload(
        &upload.label,
        &upload.bytes,
        upload.content_length,
        upload.sha256,
    )?;
    client
        .put_blob_with_headers_and_verify(
            upload.url,
            upload.bytes,
            upload.sha256,
            upload.headers,
            upload.verify_head,
        )
        .await
        .with_context(|| format!("failed to upload {}", upload.label))?;
    Ok(())
}

async fn upload_multipart_chunks<F, Fut>(
    client: &ApiClient,
    chunks: &[pack_v1::PresignedMultipartChunk],
    chunk_size: u64,
    label: &str,
    mut read_chunk: F,
) -> anyhow::Result<Vec<CompletedMultipartPart>>
where
    F: FnMut(u64, u64) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<Bytes>>,
{
    let mut parts = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let offset = u64::from(chunk.part_number.saturating_sub(1))
            .checked_mul(chunk_size)
            .context("multipart chunk offset overflow")?;
        let bytes = read_chunk(offset, chunk.content_length).await?;
        verify_upload_payload(
            &format!("{label} multipart part {}", chunk.part_number),
            &bytes,
            chunk.content_length,
            &chunk.sha256,
        )?;
        let result = client
            .put_blob_capture(&chunk.url, bytes, &chunk.sha256)
            .await
            .with_context(|| {
                format!(
                    "failed to upload {label} multipart part {}",
                    chunk.part_number
                )
            })?;
        let e_tag = result.e_tag.with_context(|| {
            format!(
                "multipart upload for {label} part {} did not return an ETag",
                chunk.part_number
            )
        })?;
        parts.push(CompletedMultipartPart {
            part_number: chunk.part_number,
            e_tag,
        });
    }
    Ok(parts)
}

fn verify_upload_payload(
    label: &str,
    bytes: &[u8],
    content_length: u64,
    sha256: &str,
) -> anyhow::Result<()> {
    if bytes.len() as u64 != content_length {
        bail!(
            "{label} size drifted between prepare-upload and upload (server signed {} bytes, local materialized {} bytes)",
            content_length,
            bytes.len()
        );
    }
    let actual_sha = format!("{:x}", Sha256::digest(bytes));
    if actual_sha != sha256 {
        bail!(
            "{label} SHA drifted between prepare-upload and upload (server signed {sha256}, local materialized {actual_sha})"
        );
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentProjectLookup {
    project: DeploymentProjectInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentProjectInfo {
    id: String,
}

#[derive(Debug, Serialize)]
struct ResumeDeployOutput {
    deployment_id: String,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

async fn resolve_project_id_for_resume(
    client: &ApiClient,
    deployment_id: &str,
    explicit_project_id: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<String> {
    if let Some(project_id) = explicit_project_id.filter(|value| !value.trim().is_empty()) {
        return Ok(project_id.to_string());
    }
    if let Some(project_id) = config
        .project
        .id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(project_id.to_string());
    }

    let deployment: DeploymentProjectLookup = client
        .get(&format!("/v1/deployments/{deployment_id}"))
        .await
        .context("failed to resolve project for resumed deployment")?;
    Ok(deployment.project.id)
}

#[allow(clippy::too_many_arguments)]
async fn resume_deploy(
    client: &ApiClient,
    deployment_id: &str,
    workspace_id: &str,
    project_id: &str,
    manifest: serde_json::Value,
    manifest_for_planning: build_manifest::Manifest,
    files: Vec<FileEntry>,
    output_dir: &Path,
    json: bool,
    warnings: Vec<String>,
) -> anyhow::Result<()> {
    output::status(
        json,
        "~",
        format!("Resuming deployment {deployment_id}"),
        output::Phase::Deploy,
    );

    let bundle_data = maybe_create_bundle(
        output_dir,
        manifest_has_compute_layer(&manifest_for_planning),
        json,
    )?;
    let manifest_for_api = bind_compute_bundle_to_manifest_value(
        manifest,
        bundle_data.as_ref().map(|(_, sha)| sha.as_str()),
    )?;
    let upload_plan = build_pack_v1_upload_plan(
        output_dir,
        &manifest_for_planning,
        &files,
        bundle_data.as_ref(),
    )
    .context("failed to prepare PACK_V1 upload plan")?;
    let deployment_attempt_id = Uuid::now_v7().to_string();

    prepare_upload_and_complete(
        client,
        deployment_id,
        workspace_id,
        project_id,
        &deployment_attempt_id,
        Some(manifest_for_api),
        output_dir,
        json,
        upload_plan,
        bundle_data.as_ref(),
    )
    .await?;

    // Output result (no polling in resume mode — builder handles status)
    if json {
        let data = ResumeDeployOutput {
            deployment_id: deployment_id.to_string(),
            status: "upload-complete".into(),
            warnings,
        };
        if let Ok(s) = serde_json::to_string(&data) {
            output::log_line("debug", "info", "deploy", &s);
        }
    } else {
        eprintln!();
        eprintln!(
            "  {} Deployment {} upload completed",
            console::style("✓").green().bold(),
            console::style(deployment_id).bold(),
        );
        eprintln!();
    }

    Ok(())
}

// ── PROCESS validation ───────────────────────────────────────

fn manifest_has_compute_layer(manifest: &build_manifest::Manifest) -> bool {
    manifest
        .layers
        .iter()
        .any(|layer| layer.target == build_manifest::LayerTarget::Compute)
}

fn bind_compute_bundle_to_manifest_value(
    mut manifest: serde_json::Value,
    bundle_sha: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let Some(bundle_sha) = bundle_sha else {
        return Ok(manifest);
    };

    let layers = manifest
        .get_mut("layers")
        .and_then(serde_json::Value::as_array_mut)
        .context("deployment manifest must contain a layers array")?;
    let mut bound = false;

    for layer in layers {
        let Some(object) = layer.as_object_mut() else {
            continue;
        };
        if object.get("target").and_then(serde_json::Value::as_str) != Some("COMPUTE") {
            continue;
        }
        if let Some(existing) = object
            .get("bundleSha256")
            .and_then(serde_json::Value::as_str)
            && existing != bundle_sha
        {
            bail!(
                "COMPUTE layer bundleSha256 ({existing}) does not match built bundle ({bundle_sha})"
            );
        }
        object.insert(
            "bundleSha256".to_string(),
            serde_json::Value::String(bundle_sha.to_string()),
        );
        bound = true;
    }

    if !bound {
        bail!("bundle was built but manifest contains no COMPUTE layer");
    }

    Ok(manifest)
}

/// Resolve a guaranteed-present manifest for the given compute mode.
///
/// Returns `Value` (not `Option<Value>`) because the server requires `manifest`
/// on `POST /v1/projects/:id/deployments` (DEP-326 schema). Three cases:
///
/// - manifest already present → passes through `validate_compute_manifest_contract`
///   (guards compute/manifest combinations) and is returned as-is.
/// - STATIC without manifest → auto-gen via `generate_static_manifest()`. The
///   build step auto-gens this only when detection suggested Static; this branch
///   covers `--compute static` overrides for projects detected as Process/Isolate.
/// - ISOLATE/PROCESS without manifest → `validate_compute_manifest_contract`
///   bails with a user-facing error (PROCESS auto-gen runs earlier in `run()`,
///   so this case here means PROCESS auto-gen failed somewhere upstream).
///
/// Replaces the `manifest_raw.expect(...)` runtime invariant with a typed
/// signature, so missing-manifest bugs surface at type-check time on call-sites.
fn resolve_manifest_for_compute(
    compute: ComputeType,
    manifest_raw: Option<serde_json::Value>,
    detection: &crate::detect::types::DetectionResult,
) -> anyhow::Result<serde_json::Value> {
    if let Some(manifest) = manifest_raw {
        validate_compute_manifest_contract(compute, true, detection)?;
        return Ok(manifest);
    }

    if compute == ComputeType::Static {
        let auto = build_manifest::generate_static_manifest();
        return serde_json::to_value(&auto)
            .context("failed to serialize auto-generated STATIC manifest");
    }

    // ISOLATE/PROCESS without manifest: defer to validate for the user-facing message.
    validate_compute_manifest_contract(compute, false, detection)?;
    // validate_compute_manifest_contract must return Err for these; reaching here
    // means its contract was changed. Surface as a bug, not a panic.
    bail!(
        "Internal error: validate_compute_manifest_contract accepted {compute:?} without a manifest.\n\
         This is a CLI bug — please report at github.com/onreza/nrz-cli/issues."
    );
}

fn validate_compute_manifest_contract(
    compute: ComputeType,
    has_manifest: bool,
    detection: &crate::detect::types::DetectionResult,
) -> anyhow::Result<()> {
    // ISOLATE without a manifest is always an error — it requires a pre-built manifest.
    if compute == ComputeType::Isolate && !has_manifest {
        let framework = &detection.name;
        return Err(output::coded_error(
            "MISSING_MANIFEST",
            format!(
                "{framework} project detected but no .onreza/manifest.json found.\n\n\
                 ISOLATE compute requires a manifest with ISOLATE layers.\n\n\
                 Options:\n\
                 \x20 1. Create .onreza/manifest.json manually\n\
                 \x20 2. Use --compute static if your build output is static files only\n\
                 \x20 3. Use --compute process for standalone server deployment"
            ),
        ));
    }

    // Safety net: PROCESS auto-generation should have produced a manifest
    // before this point. Reaching here without one is an unexpected internal state.
    if compute == ComputeType::Process && !has_manifest {
        bail!(
            "Internal error: PROCESS deploy reached validation without a manifest.\n\
             This is unexpected — please report this at github.com/onreza/nrz-cli/issues.\n\n\
             If you see this consistently, work around it by creating .onreza/manifest.json\n\
             manually."
        );
    }

    // When a manifest is present, its layers define the compute targets —
    // any compute type derived from the manifest is valid.

    Ok(())
}

/// Framework-specific hint about switching to static export.
fn framework_static_hint(framework: &str) -> &'static str {
    match framework {
        "nextjs" | "blitzjs" | "payload" => "add `output: 'export'` to next.config",
        "nuxt" => "set `ssr: false` in nuxt.config",
        "sveltekit" => "use `adapter-static` in svelte.config.js",
        "astro" => "remove `output: 'server'` from astro.config",
        "react-router" | "hydrogen" => "set `ssr: false` in react-router.config.ts",
        "tanstack-start" => "set `ssr: false` in app.config.ts",
        "remix" => "set `ssr: false` in the Remix Vite plugin options",
        "solidstart" => "set `ssr: false` in app.config.ts",
        "qwik" => "use the static adaptor in vite.config",
        "analog" => "set `ssr: false` in the Analog plugin options",
        _ => "",
    }
}

/// Pre-flight validation for PROCESS deployments.
///
/// Checks that the output directory is compatible with PROCESS before
/// expensive operations (entry resolution, bundling, upload).
fn validate_process_output(
    output_dir: &Path,
    project_dir: &Path,
    detection: &crate::detect::types::DetectionResult,
) -> anyhow::Result<()> {
    if let Some(msg) = detect_workers_runtime_target(project_dir, output_dir)? {
        return Err(output::coded_error("FRAMEWORK_UNSUPPORTED", msg));
    }

    match detection.framework.as_str() {
        "nextjs" | "blitzjs" | "payload" => {
            let has_standalone = detection
                .metadata
                .ssr_analysis
                .as_ref()
                .is_some_and(|ssr| ssr.has_standalone_output());
            let standalone_server = output_dir.join("server.js");

            if !standalone_server.is_file() {
                if has_standalone {
                    bail!(
                        "Next.js `output: 'standalone'` is configured, but PROCESS output is invalid.\n\n\
                         Missing file: {}/server.js\n\n\
                         Make sure build output points to `.next/standalone` and `next build` completed successfully.",
                        output_dir.display()
                    );
                }

                let dir_name = output_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if dir_name == "standalone" {
                    bail!(
                        "Next.js standalone output directory found, but server.js is missing.\n\n\
                         Expected: {}/server.js\n\n\
                         Make sure `next build` completed successfully and the standalone \
                         output contains server.js.",
                        output_dir.display()
                    );
                }

                bail!(
                    "Next.js PROCESS deployment requires `output: 'standalone'` \
                     in next.config.\n\n\
                     Current output is not a runnable standalone server directory \
                     (missing `{}/server.js`).\n\n\
                     Add to your next.config.{{js,mjs,ts}}:\n\
                     \x20 module.exports = {{ output: 'standalone' }}\n\n\
                     Then rebuild and redeploy. For static export, use --compute static \
                     with `output: 'export'`.",
                    output_dir.display()
                );
            }
        }
        "nuxt" => {
            let server_entry = output_dir.join("server/index.mjs");
            if !server_entry.is_file() {
                bail!(
                    "Nuxt PROCESS deployment expects server/index.mjs in {}.\n\n\
                     This file is created by `npx nuxi build`. If you used \
                     `nuxi generate`, the output is static-only and should be \
                     deployed with --compute static.",
                    output_dir.display()
                );
            }
        }
        _ => {}
    }
    Ok(())
}

/// Detect projects targeting a Workers-style runtime (Cloudflare workerd,
/// Shopify Oxygen) whose build output cannot execute on Node/Bun.
///
/// These outputs export an ESM module with a `fetch` handler instead of
/// listening on a port, so PROCESS compute would silently 404 every route.
/// We fail fast with framework-specific guidance on how to switch.
///
/// Signals, in order of preference (most specific first):
/// - `@cloudflare/vite-plugin` in package.json → Cloudflare Workers
/// - `@shopify/mini-oxygen` in package.json → Shopify Oxygen
/// - `server/wrangler.json` in build output → Cloudflare Workers (fallback)
/// - `server/oxygen.json` in build output → Shopify Oxygen (fallback)
fn detect_workers_runtime_target(
    project_dir: &Path,
    output_dir: &Path,
) -> anyhow::Result<Option<String>> {
    // Strict load: an unreadable or malformed package.json is propagated as an
    // error instead of silently yielding "no signal". Otherwise a corrupted
    // manifest would let a Workers bundle ship as PROCESS — the exact failure
    // mode this detector exists to prevent.
    let pkg = crate::detect::package_json::PackageJson::load_strict(project_dir)?;
    let has_cf_plugin = pkg
        .as_ref()
        .is_some_and(|p| p.has_dependency("@cloudflare/vite-plugin"));
    let has_mini_oxygen = pkg
        .as_ref()
        .is_some_and(|p| p.has_dependency("@shopify/mini-oxygen"));
    let has_wrangler_output = output_dir.join("server/wrangler.json").is_file();
    let has_oxygen_output = output_dir.join("server/oxygen.json").is_file();

    if !has_cf_plugin && !has_mini_oxygen && !has_wrangler_output && !has_oxygen_output {
        return Ok(None);
    }

    let (runtime, trigger, remedy) = if has_cf_plugin {
        (
            "Cloudflare Workers",
            "@cloudflare/vite-plugin is in your package.json",
            "Replace @cloudflare/vite-plugin with Nitro in vite.config.ts:\n\
             \x20      import { nitro } from 'nitro/vite'\n\
             \x20      // plugins: [tanstackStart(), nitro(), viteReact()]\n\
             \x20    Nitro's default `node-server` preset emits .output/server/index.mjs, \
             which PROCESS can run directly.",
        )
    } else if has_mini_oxygen {
        (
            "Shopify Oxygen (Cloudflare Workers)",
            "@shopify/mini-oxygen is in your package.json",
            "Apply the Hydrogen Express recipe to switch to a Node runtime:\n\
             \x20    https://github.com/Shopify/hydrogen/tree/main/cookbook/recipes/express\n\
             \x20    It replaces the Oxygen server with Express and emits build/server/index.js \
             plus a server.mjs entry at the project root.",
        )
    } else if has_wrangler_output {
        (
            "Cloudflare Workers",
            "server/wrangler.json was emitted into the build output",
            "Remove the Cloudflare Vite plugin from vite.config.ts and rebuild with \
             a Node-compatible preset (e.g. Nitro's node-server).",
        )
    } else {
        (
            "Shopify Oxygen (Cloudflare Workers)",
            "server/oxygen.json was emitted into the build output",
            "Apply the Hydrogen Express recipe to rebuild for Node:\n\
             \x20    https://github.com/Shopify/hydrogen/tree/main/cookbook/recipes/express",
        )
    };

    Ok(Some(format!(
        "{runtime} target detected ({trigger}).\n\n\
         ONREZA PROCESS compute runs Node/Bun servers, not the Workers runtime (workerd), \
         so this build cannot be deployed as-is.\n\n\
         Pick one:\n\
         \x20 1. Deploy as static (if your app has no server functions):\n\
         \x20    nrz deploy --compute static\n\n\
         \x20 2. Switch to a Node server build.\n\
         \x20    {remedy}"
    )))
}

/// Framework-specific diagnostic when entry point resolution fails.
fn framework_process_diagnostic(
    framework: &str,
    detection: &crate::detect::types::DetectionResult,
    output_dir: &Path,
) -> Option<String> {
    match framework {
        "nextjs" | "blitzjs" | "payload" => {
            let has_standalone = detection
                .metadata
                .ssr_analysis
                .as_ref()
                .is_some_and(|ssr| ssr.has_standalone_output());

            if has_standalone {
                Some(format!(
                    "Next.js `output: 'standalone'` is configured, but server.js not found \
                     in {}.\n\n\
                     Make sure `next build` completed successfully.\n\
                     Expected: .next/standalone/server.js",
                    output_dir.display()
                ))
            } else {
                Some(
                    "Next.js PROCESS deployment requires `output: 'standalone'` \
                     in next.config.\n\n\
                     Add to your next.config.{js,mjs,ts}:\n\
                     \x20 module.exports = { output: 'standalone' }\n\n\
                     Then rebuild and redeploy. This creates a self-contained \
                     server at .next/standalone/server.js."
                        .to_string(),
                )
            }
        }
        "nuxt" => Some(
            "Nuxt PROCESS deployment expects server/index.mjs in the .output/ directory.\n\n\
             Make sure you ran `npx nuxi build` (not `nuxi generate`).\n\
             The build should create .output/server/index.mjs."
                .to_string(),
        ),
        "sveltekit" => Some(
            "SvelteKit PROCESS deployment requires adapter-node.\n\n\
             Install it:\n\
             \x20 npm install -D @sveltejs/adapter-node\n\n\
             Update svelte.config.js:\n\
             \x20 import adapter from '@sveltejs/adapter-node';\n\n\
             Rebuild and redeploy."
                .to_string(),
        ),
        "react-router" => Some(format!(
            "React Router PROCESS deployment expects server/index.js in {}.\n\n\
             Make sure you ran `npx react-router build` and the build \
             output contains build/server/index.js.",
            output_dir.display()
        )),
        "remix" => Some(format!(
            "Remix PROCESS deployment expects server/index.js in {}.\n\n\
             Make sure you ran the build command and the output \
             contains build/server/index.js.",
            output_dir.display()
        )),
        "hono" => Some(
            "Hono PROCESS deployment requires a built entry point.\n\n\
             Make sure your build script produces a runnable file in dist/ \
             (e.g. dist/index.js)."
                .to_string(),
        ),
        "elysia" => Some(
            "Elysia PROCESS deployment requires a built entry point.\n\n\
             Make sure your build script produces a runnable file in dist/ \
             (e.g. dist/index.js). Elysia runs on Bun."
                .to_string(),
        ),
        "nestjs" => Some(
            "NestJS PROCESS deployment expects main.js in the dist/ directory.\n\n\
             Make sure you ran `npm run build` (nest build).\n\
             The build should create dist/main.js."
                .to_string(),
        ),
        "fastify" => Some(
            "Fastify PROCESS deployment requires a runnable entry point.\n\n\
             Set \"main\" in package.json to your server file, \
             or add a \"start\" script."
                .to_string(),
        ),
        "adonis" => Some(
            "AdonisJS PROCESS deployment expects bin/server.js in the build/ directory.\n\n\
             Make sure you ran `node ace build`.\n\
             The build should create build/bin/server.js."
                .to_string(),
        ),
        "express" => Some(
            "Express PROCESS deployment requires a runnable entry point.\n\n\
             Set \"main\" in package.json to your server file \
             (e.g. \"main\": \"server.js\"), or add a \"start\" script."
                .to_string(),
        ),
        "koa" => Some(
            "Koa PROCESS deployment requires a runnable entry point.\n\n\
             Set \"main\" in package.json to your server file \
             (e.g. \"main\": \"server.js\"), or add a \"start\" script."
                .to_string(),
        ),
        "h3" => Some(
            "H3 PROCESS deployment requires a runnable entry point.\n\n\
             Make sure your build produces a file in dist/ \
             (e.g. dist/index.mjs), or set \"main\" in package.json."
                .to_string(),
        ),
        "nitro" => Some(
            "Nitro PROCESS deployment expects server/index.mjs in the .output/ directory.\n\n\
             Make sure you ran the build command.\n\
             The build should create .output/server/index.mjs."
                .to_string(),
        ),
        "tanstack-start" => Some(format!(
            "TanStack Start PROCESS deployment expects server/index.mjs in {}.\n\n\
             Make sure you ran `npm run build` and the output \
             contains .output/server/index.mjs (Nitro default preset).\n\n\
             If you see `dist/server/` with a worker-entry-*.js instead, your project is \
             configured for Cloudflare Workers via @cloudflare/vite-plugin, which is not \
             supported by PROCESS. Either remove that plugin and use Nitro's default \
             node-server preset, or deploy with --compute static.",
            output_dir.display()
        )),
        "hydrogen" => Some(format!(
            "Hydrogen PROCESS deployment could not resolve a runnable entry in {}.\n\n\
             Hydrogen has two build layouts:\n\
             \x20 - Oxygen (default): emits dist/server/index.js as a Workers bundle — \
             not executable by PROCESS. Apply the Hydrogen Express recipe \
             (https://github.com/Shopify/hydrogen/tree/main/cookbook/recipes/express) \
             to switch to Node.\n\
             \x20 - Express recipe: emits build/server/index.js plus a server.mjs at the \
             project root; the `start` script runs `node server.mjs`.\n\n\
             If you used the Express recipe, make sure `npm run build` completed and the \
             `start` script or package.json `main` points to server.mjs.",
            output_dir.display()
        )),
        _ => None,
    }
}

/// Frameworks where PROCESS output must be explicit and validated.
///
/// For these, a failed entry-point resolution triggers `framework_process_diagnostic`
/// and `bail!` with actionable guidance. Non-strict frameworks silently fall back
/// to `bun <output_dir>`, which for SSR frameworks is almost always a 404-machine —
/// so every SSR framework we ship support for is listed here explicitly.
fn is_strict_process_framework(framework: &str) -> bool {
    matches!(
        framework,
        "nextjs"
            | "nuxt"
            | "sveltekit"
            | "astro"
            | "remix"
            | "react-router"
            | "solidstart"
            | "qwik"
            | "analog"
            | "blitzjs"
            | "payload"
            | "tanstack-start"
            | "hydrogen"
    )
}

// ── PROCESS entry point ──────────────────────────────────────

fn is_windows_drive_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn sanitize_config_entry(entry: &str) -> anyhow::Result<String> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        bail!("[deploy] entry in onreza.toml must not be empty");
    }

    let normalized = trimmed.replace('\\', "/");
    let lowered = normalized.to_ascii_lowercase();
    let path = Path::new(&normalized);
    if path.is_absolute() || lowered.starts_with("file:") || is_windows_drive_absolute(&normalized)
    {
        bail!(
            "[deploy] entry must be a relative path within the output directory, got: \"{entry}\""
        );
    }

    let mut cleaned = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(seg) => cleaned.push(seg),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "[deploy] entry must be a relative path within the output directory, got: \"{entry}\""
                );
            }
        }
    }

    if cleaned.as_os_str().is_empty() {
        bail!("[deploy] entry in onreza.toml must not be empty");
    }

    Ok(cleaned.to_string_lossy().replace('\\', "/"))
}

/// Resolve and ensure entry point for PROCESS deployments.
///
/// 1. Resolve entry: config `[deploy] entry` > framework auto-detect
/// 2. Validate file existence when entry is resolved
/// 3. If unresolved for non-strict frameworks, fallback to runtime default (`bun <output_dir>`)
fn ensure_process_entry(
    output_dir: &Path,
    project_dir: &Path,
    config_entry: Option<&str>,
    detection: &crate::detect::types::DetectionResult,
    json: bool,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    // 1. Resolve entry point
    let entry = if let Some(e) = config_entry {
        Some(
            sanitize_config_entry(e)
                .map_err(|err| output::with_default_code(err, "INVALID_DEPLOY_ENTRY"))?,
        )
    } else {
        match crate::detect::resolve_entry_point_detailed(
            &detection.framework,
            output_dir,
            project_dir,
        ) {
            crate::detect::EntryPointResolution::Found(resolved) => {
                output::status(
                    json,
                    "~",
                    format!(
                        "Entry point resolved from {:?}: {}",
                        resolved.source, resolved.path
                    ),
                    output::Phase::Deploy,
                );
                Some(resolved.path)
            }
            crate::detect::EntryPointResolution::Ambiguous(candidates) => {
                if is_strict_process_framework(&detection.framework) {
                    return Err(output::coded_error(
                        "ENTRY_POINT_AMBIGUOUS",
                        format!(
                            "Cannot determine entry point for PROCESS deployment: multiple candidates found.\n\n\
                             Candidates in {}:\n\
                             {}\n\n\
                             Set [deploy] entry in onreza.toml to pick one explicitly.",
                            output_dir.display(),
                            candidates
                                .iter()
                                .map(|c| format!("  - {c}"))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ),
                    ));
                }

                let warning = format!(
                    "Entry point auto-detection is ambiguous for PROCESS deployment ({} candidates in {}).\n\
                     Falling back to runtime default (`bun` in output directory).\n\
                     Set [deploy] entry in onreza.toml to make startup explicit.",
                    candidates.len(),
                    output_dir.display()
                );
                return Ok((None, Some(warning)));
            }
            crate::detect::EntryPointResolution::NotFound => {
                if !is_strict_process_framework(&detection.framework) {
                    let warning = format!(
                        "Entry point auto-detection did not find a runnable file in {}.\n\
                         Falling back to runtime default (`bun` in output directory).\n\
                         Set [deploy] entry in onreza.toml to avoid runtime guesswork.",
                        output_dir.display()
                    );
                    return Ok((None, Some(warning)));
                }

                if let Some(diagnostic) =
                    framework_process_diagnostic(&detection.framework, detection, output_dir)
                {
                    bail!("{diagnostic}");
                }
                bail!(
                    "Cannot determine entry point for PROCESS deployment.\n\n\
                     No entry point found in output directory: {}\n\n\
                     Options:\n\
                     \x20 1. Set [deploy] entry = \"server.ts\" in onreza.toml\n\
                     \x20 2. Add \"main\" or \"module\" field to package.json\n\
                     \x20 3. Add a start/serve script with an explicit file path",
                    output_dir.display()
                );
            }
            crate::detect::EntryPointResolution::Error(err) => {
                bail!(
                    "Cannot determine entry point for PROCESS deployment.\n\n\
                     {err}\n\n\
                     Set [deploy] entry in onreza.toml to override auto-detection."
                );
            }
        }
    };

    // 2. Validate file exists and is within output_dir
    if let Some(ref entry) = entry {
        let entry_path = output_dir.join(entry);
        if !entry_path.is_file() {
            return Err(output::coded_error(
                "INVALID_DEPLOY_ENTRY",
                format!(
                    "Entry point \"{entry}\" not found in output directory: {}\n\n\
                     Make sure the file exists after running your build command.",
                    output_dir.display()
                ),
            ));
        }
        let canonical_entry = entry_path
            .canonicalize()
            .with_context(|| format!("failed to resolve entry point path: {entry}"))?;
        let canonical_output = output_dir
            .canonicalize()
            .context("failed to resolve output directory path")?;
        if !canonical_entry.starts_with(&canonical_output) {
            return Err(output::coded_error(
                "INVALID_DEPLOY_ENTRY",
                format!("entry point must be inside the output directory, got: \"{entry}\""),
            ));
        }

        output::status(
            json,
            "~",
            format!("Entry point: {entry}"),
            output::Phase::Deploy,
        );
    } else {
        output::status(
            json,
            "~",
            format!(
                "Entry point: <runtime default bun {}>",
                output_dir.display()
            ),
            output::Phase::Deploy,
        );
    }

    Ok((entry, None))
}

// ── Bundle helpers ───────────────────────────────────────────

fn maybe_create_bundle(
    output_dir: &Path,
    is_process: bool,
    json: bool,
) -> anyhow::Result<Option<(Vec<u8>, String)>> {
    if !is_process {
        return Ok(None);
    }
    output::status(
        json,
        "~",
        "Creating tar.zst bundle (PROCESS deployment)...",
        output::Phase::Deploy,
    );
    let stats = bundle::create_bundle(output_dir).context("failed to create tar.zst bundle")?;

    let mut summary = format!(
        "Bundle created ({}, {} files",
        format_bytes(stats.bytes.len()),
        stats.files
    );
    if stats.symlinks_preserved > 0 {
        summary.push_str(&format!(", {} symlinks", stats.symlinks_preserved));
    }
    summary.push_str(&format!(", sha256: {}…)", &stats.sha256_hex[..12]));
    output::success(json, summary, output::Phase::Deploy);

    if stats.symlinks_skipped > 0 {
        output::warn(
            json,
            format!(
                "Skipped {} symlink(s) that resolve outside the bundle root (see warnings above).",
                stats.symlinks_skipped
            ),
            output::Phase::Deploy,
        );
    }

    Ok(Some((stats.bytes, stats.sha256_hex)))
}

// ── Build step ───────────────────────────────────────────────

/// Resolve build command. Priority: CLI flag > config > server > auto-detect.
fn resolve_build_command(
    explicit: Option<&str>,
    project_dir: &Path,
    config: &ProjectConfig,
    server_command: Option<CommandHint<'_>>,
) -> Option<String> {
    if let Some(cmd) = explicit {
        return Some(cmd.to_string());
    }
    if let Some(cmd) = config.build_command() {
        return Some(cmd.to_string());
    }
    if let Some(hint) = server_command {
        return hint.command.map(str::to_string);
    }
    // Only auto-detect if package.json has a "build" script
    let pkg = crate::detect::package_json::PackageJson::load(project_dir)?;
    if !pkg.scripts.contains_key("build") {
        return None;
    }
    let pm = crate::detect::detect_package_manager_name(project_dir);
    Some(format!("{pm} run build"))
}

/// Lightweight pre-build check: does the project use Next.js (directly or via wrapper like Payload v3)?
fn is_nextjs_project(project_dir: &Path) -> bool {
    let Some(pkg) = crate::detect::package_json::PackageJson::load(project_dir) else {
        return false;
    };
    pkg.has_dependency("next")
}

/// Check if the project uses SvelteKit with adapter-auto (needs GCP_BUILDPACKS env injection).
fn is_sveltekit_with_adapter_auto(project_dir: &Path) -> bool {
    let Some(pkg) = crate::detect::package_json::PackageJson::load(project_dir) else {
        return false;
    };
    if !pkg.has_dependency("@sveltejs/kit") {
        return false;
    }
    if pkg.has_dependency("@sveltejs/adapter-node")
        || pkg.has_dependency("@sveltejs/adapter-static")
        || pkg.has_dependency("@sveltejs/adapter-vercel")
        || pkg.has_dependency("@sveltejs/adapter-cloudflare")
        || pkg.has_dependency("@sveltejs/adapter-netlify")
    {
        return false;
    }
    let config_content = ["svelte.config.js", "svelte.config.ts"]
        .iter()
        .map(|n| project_dir.join(n))
        .find(|p| p.is_file())
        .and_then(|p| std::fs::read_to_string(p).ok());
    match config_content {
        Some(content) => content.contains("adapter-auto"),
        None => true,
    }
}

/// Run a shell command, streaming stdout/stderr through structured JSON in JSON mode.
///
/// In JSON mode: pipes stdout/stderr, wraps each line via `output::log_line()`.
/// In non-JSON mode: inherits stdio (unchanged behavior).
///
/// `child_stream` controls the `s` field for child stdout lines ("user" or "debug").
/// Child stderr always goes to "debug" stream with "warn" level.
fn run_command_streaming(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    phase: output::Phase,
    child_stream: &str,
    extra_env: &[(&str, &str)],
) -> anyhow::Result<()> {
    use std::io::BufRead;

    #[cfg(unix)]
    let (shell, shell_args) = ("sh", ["-c", cmd]);
    #[cfg(windows)]
    let (shell, shell_args) = ("cmd", ["/C", cmd]);

    if !json {
        let status = std::process::Command::new(shell)
            .args(shell_args)
            .current_dir(project_dir)
            .envs(extra_env.iter().copied())
            .status()
            .with_context(|| format!("failed to start command: {cmd}"))?;
        if !status.success() {
            match status.code() {
                Some(code) => {
                    return Err(output::coded_error(
                        format!("{}_EXIT_CODE", phase.as_str().to_uppercase()),
                        format!("{phase} command `{cmd}` failed with exit code {code}"),
                    ));
                }
                None => {
                    return Err(output::coded_error(
                        format!("{}_SIGNAL_KILLED", phase.as_str().to_uppercase()),
                        format!("{phase} process `{cmd}` was killed by signal"),
                    ));
                }
            }
        }
        return Ok(());
    }

    // JSON mode: capture stdout/stderr and emit structured log lines
    let mut child = std::process::Command::new(shell)
        .args(shell_args)
        .current_dir(project_dir)
        .envs(extra_env.iter().copied())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start command: {cmd}"))?;

    let stdout = child
        .stdout
        .take()
        .context("expected piped stdout on child process")?;
    let stderr = child
        .stderr
        .take()
        .context("expected piped stderr on child process")?;

    let phase_out = phase.to_string();
    let stream_out = child_stream.to_string();
    let stdout_handle = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for result in reader.lines() {
            match result {
                Ok(line) if line.is_empty() => {} // skip blank lines
                Ok(line) => output::log_line(&stream_out, "info", &phase_out, &line),
                Err(e) => {
                    output::log_line(
                        "debug",
                        "warn",
                        &phase_out,
                        &format!("[nrz] failed to read stdout: {e}"),
                    );
                    break;
                }
            }
        }
    });

    let phase_err = phase.to_string();
    // Install stderr → "user" stream (errors visible to user), other phases follow child_stream
    let stream_err = if phase == output::Phase::Install {
        "user".to_string()
    } else {
        child_stream.to_string()
    };
    let stderr_handle = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for result in reader.lines() {
            match result {
                Ok(line) if line.is_empty() => {} // skip blank lines
                Ok(line) => output::log_line(&stream_err, "warn", &phase_err, &line),
                Err(e) => {
                    output::log_line(
                        "debug",
                        "warn",
                        &phase_err,
                        &format!("[nrz] failed to read stderr: {e}"),
                    );
                    break;
                }
            }
        }
    });

    let status = child
        .wait()
        .with_context(|| format!("failed to wait for command: {cmd}"))?;
    if let Err(e) = stdout_handle.join() {
        tracing::warn!("stdout reader thread panicked: {e:?}");
    }
    if let Err(e) = stderr_handle.join() {
        tracing::warn!("stderr reader thread panicked: {e:?}");
    }

    if !status.success() {
        match status.code() {
            Some(code) => {
                return Err(output::coded_error(
                    format!("{}_EXIT_CODE", phase.as_str().to_uppercase()),
                    format!("{phase} command failed with exit code {code}"),
                ));
            }
            None => {
                return Err(output::coded_error(
                    format!("{}_SIGNAL_KILLED", phase.as_str().to_uppercase()),
                    format!("{phase} process was killed by signal"),
                ));
            }
        }
    }
    Ok(())
}

fn run_install_step(
    project_dir: &Path,
    json: bool,
    server_command: Option<CommandHint<'_>>,
) -> anyhow::Result<()> {
    let Some(cmd) = resolve_install_command(project_dir, server_command) else {
        return Ok(());
    };

    output::status(
        json,
        ">",
        format!("Installing dependencies: {cmd}"),
        output::Phase::Deploy,
    );
    // Install child output → debug stream (npm noise), nrz markers go through output::status/success
    run_command_streaming(
        &cmd,
        project_dir,
        json,
        output::Phase::Install,
        "debug",
        &[],
    )?;
    output::success(json, "Dependencies installed", output::Phase::Deploy);
    Ok(())
}

fn resolve_install_command(
    project_dir: &Path,
    server_command: Option<CommandHint<'_>>,
) -> Option<String> {
    // Priority: authoritative server command > auto-detect from package manager.
    // PRESET server commands are filtered out by command_hint().
    if let Some(server_cmd) = server_command {
        return server_cmd.command.map(str::to_string);
    }
    if !project_dir.join("package.json").exists() {
        return None;
    }

    let local_fs = crate::detect::fs::LocalFs::new(project_dir);
    let pkg = crate::detect::package_json::PackageJson::load_from_fs(&local_fs);
    let pm_info = crate::detect::package_manager::detect_package_manager(&local_fs, pkg.as_ref());
    match pm_info {
        Some(info) => {
            Some(crate::detect::package_manager::install_command(info.pm_type).to_string())
        }
        None => Some("npm install".to_string()),
    }
}

fn run_build_step(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    extra_env: &[(&str, &str)],
) -> anyhow::Result<()> {
    if cmd.trim().is_empty() {
        return Err(output::coded_error(
            "INVALID_CONFIG",
            "empty build command".to_string(),
        ));
    }

    output::status(json, ">", format!("Building: {cmd}"), output::Phase::Deploy);
    // Build child output → user stream (webpack/vite output is useful)
    run_command_streaming(
        cmd,
        project_dir,
        json,
        output::Phase::Build,
        "user",
        extra_env,
    )?;
    output::success(json, "Build completed", output::Phase::Deploy);
    Ok(())
}

// ── Output scan ──────────────────────────────────────────────

/// Read buffer for streaming SHA-256. Sized to match a single page-cache
/// readahead window — small enough to stay in L2 cache, large enough that the
/// per-file read overhead doesn't dominate hashing throughput on big assets.
const SCAN_HASH_CHUNK_BYTES: usize = 64 * 1024;

/// Recursively scan `dir` and return a sorted list of `FileEntry { path, size, content_hash }`.
///
/// SHA-256 and size are computed **streaming**: the file is read in
/// `SCAN_HASH_CHUNK_BYTES` chunks and fed into the hasher, never buffered into
/// memory. Bytes are re-read from disk at upload time (page cache absorbs the
/// second read on any reasonable build host).
///
/// Symlinks are skipped to avoid loops and traversal escapes.
fn scan_dir(dir: &Path) -> anyhow::Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    scan_dir_recursive(dir, dir, &mut files)?;
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn scan_dir_recursive(
    base: &Path,
    current: &Path,
    files: &mut Vec<FileEntry>,
) -> anyhow::Result<()> {
    let entries = std::fs::read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", current.display()))?;
        let path = entry.path();
        let ft = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", path.display()))?;

        // Skip symlinks to avoid loops and directory traversal
        if ft.is_symlink() {
            continue;
        }

        if ft.is_dir() {
            scan_dir_recursive(base, &path, files)?;
        } else if ft.is_file() {
            let rel = path
                .strip_prefix(base)
                .context("failed to compute relative path")?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let (size, content_hash) = hash_file_streaming(&path)
                .with_context(|| format!("failed to hash {}", rel_str))?;
            files.push(FileEntry {
                path: rel_str,
                size,
                content_hash,
            });
        }
    }

    Ok(())
}

/// Streaming SHA-256 + size for a single file. Returns `(size, lowercase_hex_sha256)`.
fn hash_file_streaming(path: &Path) -> anyhow::Result<(u64, String)> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; SCAN_HASH_CHUNK_BYTES];
    let mut size: u64 = 0;
    loop {
        let n = file
            .read(&mut buf)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}

// ── Synthetic commit SHA ─────────────────────────────────────

/// Stable per-deployment commit SHA derived from the file manifest. Used as a
/// fallback when `git rev-parse HEAD` is unavailable. Includes per-file content
/// hashes so two deploys of byte-identical bundles produce the same synthetic
/// SHA — which is exactly what cross-deploy CAS dedup keys off.
fn synthetic_sha(files: &[FileEntry]) -> String {
    let mut hasher = Sha256::new();
    for f in files {
        hasher.update(f.path.as_bytes());
        hasher.update(b":");
        hasher.update(f.content_hash.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

// ── Compute type resolution ──────────────────────────────────

fn resolve_compute_type(
    cli_flag: Option<&str>,
    config_value: Option<&str>,
    detection: &crate::detect::types::DetectionResult,
) -> anyhow::Result<ComputeType> {
    // Priority: CLI flag > config > detection
    if let Some(val) = cli_flag.or(config_value) {
        return parse_compute_type(val);
    }
    Ok(detection.suggested_compute)
}

fn parse_compute_type(s: &str) -> anyhow::Result<ComputeType> {
    match s.to_lowercase().as_str() {
        "static" => Ok(ComputeType::Static),
        "isolate" => Ok(ComputeType::Isolate),
        "process" => Ok(ComputeType::Process),
        _ => Err(output::coded_error(
            "INVALID_COMPUTE_TYPE",
            format!("invalid compute type: \"{s}\". Must be one of: static, isolate, process"),
        )),
    }
}

fn git_cmd(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn make_spinner(json: bool, msg: &str) -> Option<ProgressBar> {
    if json {
        return None;
    }
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("  {spinner} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    spinner.set_message(msg.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    Some(spinner)
}

fn finish_spinner(spinner: Option<ProgressBar>, msg: &str) {
    if let Some(s) = spinner {
        if msg.is_empty() {
            s.finish_and_clear();
        } else {
            s.finish_with_message(msg.to_string());
        }
    }
}

fn format_u64_bytes(bytes: u64) -> String {
    format_bytes(usize::try_from(bytes).unwrap_or(usize::MAX))
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
