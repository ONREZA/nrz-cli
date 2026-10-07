use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;

use crate::artifact::source_bundle_v1::{
    RuntimeDependencyPackaging, RuntimeReadinessContract, SOURCE_BUNDLE_FORMAT, SourceBundlePlan,
    build_source_bundle_plan_with_scan,
};
use crate::artifact::{
    ArtifactFileCollection, ArtifactRootScope, BuildArtifact, BuildManifestSource, FileEntry,
    RuntimeArtifact, RuntimeArtifactFileBreakdown, RuntimeArtifactScan,
};
use crate::build;
use crate::build::manifest as build_manifest;
use crate::cli::{BuildArgs, DeployArgs};
use crate::detect::types::{ComputeType, RuntimeType};
use crate::output;

pub(super) struct DeployPlanRequest<'a> {
    pub(super) args: &'a DeployArgs,
    pub(super) command: &'a crate::context::CommandContext,
    pub(super) explicit_compute: Option<ComputeType>,
    pub(super) build_logs: Option<&'a super::BuildLogEmitter>,
    pub(super) execution_env: &'a [(String, String)],
    pub(super) target_production: Option<bool>,
    pub(super) platform_runner: bool,
}

/// A typed recipe owns only its generated output, never the project or a symlink target.
pub(super) fn clear_native_build_output(
    project_dir: &Path,
    output_directory: &str,
) -> anyhow::Result<()> {
    if output_directory.contains(['\\', ':'])
        || output_directory
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        anyhow::bail!("native build output must be a canonical relative directory");
    }
    let mut output = project_dir.canonicalize()?;
    for component in Path::new(output_directory).components() {
        let std::path::Component::Normal(component) = component else {
            anyhow::bail!("native build output must stay inside the project directory");
        };
        output.push(component);
        let metadata = match std::fs::symlink_metadata(&output) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error).context("cannot inspect native build output"),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            anyhow::bail!("native build output must not contain symlinks or source files");
        }
    }
    std::fs::remove_dir_all(&output).context("cannot remove stale native build output")
}

pub(super) async fn scan_runtime_artifact_for_plan(
    root_dir: PathBuf,
    scan: RuntimeArtifactScan,
) -> anyhow::Result<Vec<FileEntry>> {
    tokio::task::spawn_blocking(move || super::scan_runtime_artifact(&root_dir, &scan))
        .await
        .map_err(|error| {
            output::coded_error(
                "UPLOAD_FAILED",
                format!("runtime artifact scan task failed: {error}"),
            )
        })?
        .map_err(|error| {
            if error.chain().any(|cause| cause.is::<output::CodedError>()) {
                return error.context("failed to scan runtime artifact");
            }
            output::coded_error(
                "UPLOAD_FAILED",
                format!("failed to scan runtime artifact: {error:#}"),
            )
        })
}

pub(super) struct ArtifactPlan {
    pub(super) build: BuildArtifact,
    pub(super) runtime: RuntimeArtifact,
    pub(super) files: ArtifactFileCollection,
    pub(super) file_breakdown: RuntimeArtifactFileBreakdown,
}

pub(super) struct DeployPlan {
    pub(super) artifact: ArtifactPlan,
    pub(super) compute: ComputeType,
    pub(super) build_command: Option<String>,
    pub(super) build_skipped: bool,
    pub(super) production: Option<bool>,
    pub(super) manifest_raw: serde_json::Value,
    pub(super) manifest_source: BuildManifestSource,
    pub(super) files: Vec<FileEntry>,
    pub(super) functions: Option<crate::functions::FunctionPublishPayload>,
    pub(super) has_compute_layer: bool,
    pub(super) health_check: Option<super::ResolvedHealthCheck>,
    pub(super) warnings: Vec<String>,
}

impl DeployPlan {
    pub(super) fn explain(
        &self,
        command: &crate::context::CommandContext,
        project_id: Option<&str>,
        source_bundle: &SourceBundlePlan,
    ) -> DeployPlanExplain {
        let detection = &self.artifact.build.detection;
        DeployPlanExplain {
            schema_version: "DEPLOY_PLAN_V1",
            project_id: project_id.map(str::to_string),
            root_dir: command.root_dir.to_string_lossy().into_owned(),
            project_dir: command.project_dir.to_string_lossy().into_owned(),
            selected_app: command.selected_app.as_ref().map(|app| SelectedAppExplain {
                requested: app.requested.clone(),
                path: app.path.clone(),
                source: app.source.as_str(),
            }),
            framework: FrameworkExplain {
                slug: detection.framework.clone(),
                name: detection.name.clone(),
                version: detection.version.clone(),
                reason: detection.reason.clone(),
                config_files: detection.metadata.config_files.clone(),
                structure: detection.metadata.structure.clone(),
                runtime: format!("{:?}", detection.metadata.runtime.runtime_type).to_lowercase(),
                runtime_version: detection.metadata.runtime.version.clone(),
                python_patch_version: detection
                    .metadata
                    .runtime
                    .version
                    .as_deref()
                    .filter(|_| detection.metadata.runtime.runtime_type == RuntimeType::Python)
                    .and_then(nrz_source_bundle::PythonMinor::from_version)
                    .map(|minor| minor.exact_version().to_string()),
            },
            build: BuildPlanExplain {
                command: self.build_command.clone(),
                skipped: self.build_skipped,
                output_dir: self
                    .artifact
                    .build
                    .output_dir
                    .to_string_lossy()
                    .into_owned(),
                output_manifest_source: self.artifact.build.manifest_source.as_str(),
                deployment_manifest_source: self.manifest_source.as_str(),
            },
            compute: self.compute,
            target: DeployTargetExplain {
                production: self.production,
                environment: match self.production {
                    Some(true) => "production",
                    Some(false) => "preview",
                    None => "default",
                },
            },
            runtime_artifact: RuntimeArtifactExplain {
                root_dir: self
                    .artifact
                    .runtime
                    .root_dir
                    .to_string_lossy()
                    .into_owned(),
                scan: self.artifact.runtime.scan.explain(),
                upload_strategy: "source_bundle_v1",
                has_compute_layer: self.has_compute_layer,
                file_breakdown: self.artifact.file_breakdown.clone(),
            },
            source_bundle: SourceBundleExplain {
                format: SOURCE_BUNDLE_FORMAT,
                source_size_bytes: source_bundle.source_size_bytes,
                source_sha256: source_bundle.source_sha256.clone(),
                logical_manifest_sha256: source_bundle.logical_manifest_sha256.clone(),
                multipart: nrz_source_publisher::source_uses_multipart(
                    source_bundle.source_size_bytes,
                ),
            },
            files: self.artifact.files.summary.clone(),
            health_check: self
                .health_check
                .as_ref()
                .and_then(|health_check| serde_json::to_value(health_check.to_info()).ok()),
            warnings: self.warnings.clone(),
        }
    }

    pub(super) fn materialize_source_bundle(
        &self,
        json: bool,
        dependency_packaging: RuntimeDependencyPackaging,
    ) -> anyhow::Result<SourceBundlePlan> {
        output::status(
            json,
            "~",
            "Validating SOURCE_BUNDLE_V1 archive...",
            output::Phase::Deploy,
        );
        build_source_bundle_plan_with_scan(
            &self.artifact.runtime.root_dir,
            &self.artifact.runtime.manifest,
            &self.files,
            &self.artifact.runtime.scan,
            dependency_packaging,
            self.health_check.as_ref().map(|health_check| {
                health_check.path.as_deref().map_or(
                    RuntimeReadinessContract::Tcp,
                    RuntimeReadinessContract::Http,
                )
            }),
        )
        .context("failed to prepare SOURCE_BUNDLE_V1 upload plan")
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeployPlanExplain {
    schema_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    root_dir: String,
    project_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_app: Option<SelectedAppExplain>,
    framework: FrameworkExplain,
    build: BuildPlanExplain,
    compute: ComputeType,
    target: DeployTargetExplain,
    runtime_artifact: RuntimeArtifactExplain,
    source_bundle: SourceBundleExplain,
    files: crate::artifact::ArtifactFileSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_check: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedAppExplain {
    requested: String,
    path: String,
    source: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameworkExplain {
    slug: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    config_files: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    structure: Vec<String>,
    runtime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    python_patch_version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildPlanExplain {
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    skipped: bool,
    output_dir: String,
    output_manifest_source: &'static str,
    deployment_manifest_source: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeployTargetExplain {
    #[serde(skip_serializing_if = "Option::is_none")]
    production: Option<bool>,
    environment: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeArtifactExplain {
    root_dir: String,
    scan: serde_json::Value,
    upload_strategy: &'static str,
    has_compute_layer: bool,
    file_breakdown: RuntimeArtifactFileBreakdown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceBundleExplain {
    format: &'static str,
    source_size_bytes: u64,
    source_sha256: String,
    logical_manifest_sha256: String,
    multipart: bool,
}

pub(super) async fn build(request: DeployPlanRequest<'_>) -> anyhow::Result<DeployPlan> {
    let args = request.args;
    let command = request.command;
    let json = command.json;
    let project_dir = &command.project_dir;
    let effective = &command.effective;
    let mut warnings = Vec::new();
    let production = request.target_production;
    let runtime_platform = request
        .platform_runner
        .then(crate::artifact::native_dependencies::RuntimePlatform::from_environment)
        .transpose()?;

    super::validate_prebuild_compute_intent(project_dir, request.explicit_compute)?;

    let mut detection =
        crate::detect::detect_with_framework_override(project_dir, effective.framework_override());
    let application_runtime =
        crate::detect::application_runtime::resolve_application_runtime_with_project(
            &crate::detect::fs::LocalFs::new(project_dir),
            &detection.framework,
            &effective.config().deploy,
            effective.framework_override(),
        )
        .map_err(|error| {
            output::coded_error("APPLICATION_RUNTIME_INVALID", format!("{error:#}"))
        })?;
    crate::detect::application_runtime::bind_application_runtime(
        &crate::detect::fs::LocalFs::new(project_dir),
        &mut detection,
        application_runtime,
        effective.deploy_entry(),
        effective.framework_override(),
    )
    .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", format!("{error:#}")))?;
    let build_runtime_version = super::validate_application_runtime_before_build(
        detection.metadata.application_runtime.as_ref(),
        &detection.metadata.runtime,
        effective,
        request.platform_runner,
        request.execution_env,
    )
    .await?;
    if !args.skip_build && !args.skip_install {
        super::run_install_step(
            project_dir,
            json,
            effective,
            request.execution_env,
            request.build_logs,
            request.platform_runner,
        )
        .await?;
    }

    let native_recipe = crate::detect::native::native_recipe(&detection.framework);
    let build_preparation = if native_recipe.is_some() {
        crate::frameworks::BuildPreparation::default()
    } else {
        crate::frameworks::prepare_build(project_dir)?
    };
    for warning in &build_preparation.warnings {
        output::warn(json, warning, output::Phase::Deploy);
        warnings.push(warning.clone());
    }
    for patch in &build_preparation.env {
        if !patch.message.is_empty() {
            output::status(json, "~", &patch.message, output::Phase::Deploy);
        }
    }
    let build_env =
        super::merge_command_environment(request.execution_env, &build_preparation.env_pairs());

    let native_plan = if !args.skip_build
        && let Some(recipe) = native_recipe
        && super::native_build::default_command(
            args.build_command.as_deref(),
            effective.build_command(),
        ) {
        Some(super::native_build::recipe_commands(
            project_dir,
            recipe,
            request.platform_runner,
        )?)
    } else {
        None
    };
    let build_command = if let Some(plan) = &native_plan {
        Some(plan.build.display())
    } else {
        super::resolve_build_command(args.build_command.as_deref(), project_dir, effective)
            .filter(|command| native_recipe.is_none() || !command.trim().is_empty())
    };
    if let Some(plan) = &native_plan {
        super::run_native_build_step(
            plan,
            native_recipe.unwrap(),
            project_dir,
            json,
            &build_env,
            request.build_logs,
            request.platform_runner,
        )
        .await?;
    } else if !args.skip_build
        && let Some(cmd) = build_command.as_deref()
    {
        crate::frameworks::clear_before_build(project_dir)?;
        super::run_build_step(cmd, project_dir, json, &build_env, request.build_logs)?;
    }

    if let Some(declaration) =
        detection
            .metadata
            .application_runtime
            .as_ref()
            .filter(|declaration| {
                declaration.family == nrz_source_bundle::ApplicationRuntimeFamily::Python
            })
    {
        let entry = declaration
            .entry
            .clone()
            .context("Python launch entry is absent")?;
        super::python_launch::materialize_python_entry(
            project_dir,
            &crate::detect::python_launch::PythonLaunch {
                entry,
                args: declaration.args.clone(),
            },
            declaration.python_version.unwrap_or_default(),
        )
        .map_err(|error| {
            output::coded_error("APPLICATION_RUNTIME_INVALID", format!("{error:#}"))
        })?;
    }

    let application_runtime = detection.metadata.application_runtime;
    detection =
        crate::detect::detect_with_framework_override(project_dir, effective.framework_override());
    crate::detect::application_runtime::bind_application_runtime(
        &crate::detect::fs::LocalFs::new(project_dir),
        &mut detection,
        application_runtime,
        effective.deploy_entry(),
        effective.framework_override(),
    )
    .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", format!("{error:#}")))?;
    if let Some(version) = &build_runtime_version
        && detection.metadata.runtime.runtime_type != RuntimeType::Python
    {
        detection.metadata.runtime.version = Some(version.clone());
    }

    output::status(
        json,
        "~",
        "Validating build output...",
        output::Phase::Deploy,
    );
    let build_result = build::run_with_effective_config(
        BuildArgs {
            dir: project_dir.to_string_lossy().into_owned(),
            skip_validation: false,
        },
        json,
        effective,
        Some(&detection),
        false,
        &command.root_dir,
    )
    .await
    .map_err(|error| {
        contextualize_missing_build_output(error, build_command.as_deref(), args.skip_build)
    })?;

    if let Some(recipe) = native_recipe {
        let mut entries = build_result
            .manifest
            .as_ref()
            .into_iter()
            .flat_map(|manifest| &manifest.layers)
            .filter(|layer| layer.target == build_manifest::LayerTarget::Compute)
            .filter(|layer| {
                layer
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .is_none_or(|runtime| {
                        runtime.family == nrz_source_bundle::ApplicationRuntimeFamily::Executable
                    })
            })
            .map(|layer| {
                (
                    build_result.output_dir.join(&layer.directory),
                    layer.entry.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            entries.push((
                build_result.output_dir.clone(),
                effective.deploy_entry().or_else(|| {
                    detection
                        .metadata
                        .build_info
                        .as_ref()
                        .and_then(|info| info.entry_point.as_deref())
                }),
            ));
        }
        for (root, entry) in entries {
            let evidence =
                super::native_build::validate_output(&root, recipe, entry).map_err(|error| {
                    output::coded_error("NATIVE_OUTPUT_INVALID", format!("{error:#}"))
                })?;
            tracing::debug!(
                entry = evidence.entry.as_deref(),
                target = evidence.target,
                interpreter = evidence.interpreter.as_deref(),
                library_count = evidence.libraries.len(),
                library_path_count = evidence.library_paths.len(),
                "native build output validated"
            );
        }
    }

    let mut deployment_manifest_source = build_result.manifest_source;
    let mut build_artifact = BuildArtifact {
        output_dir: build_result.output_dir,
        manifest: build_result.manifest,
        manifest_source: build_result.manifest_source,
        detection,
    };
    if let (Some(explicit), Some(manifest)) =
        (request.explicit_compute, build_artifact.manifest.as_ref())
        && super::compute_type_from_manifest(manifest) != explicit
    {
        if explicit == ComputeType::Process
            && build_artifact.manifest_source == BuildManifestSource::Generated
        {
            // A generic STATIC fallback is an inference, not an authored routing
            // contract. Resolve the requested PROCESS entry below before packing.
            build_artifact.manifest = None;
        } else {
            return Err(output::coded_error(
                "COMPUTE_MANIFEST_MISMATCH",
                "Explicit compute mode conflicts with the build manifest. Update the manifest or build the matching export before deploying.",
            ));
        }
    }
    let has_build_manifest = build_artifact.manifest.is_some();
    let compute = super::resolve_deploy_compute_type(
        request.explicit_compute,
        build_artifact.manifest.as_ref(),
        &build_artifact.detection,
    );

    if !has_build_manifest
        && args.compute.is_none()
        && effective.deploy_compute().is_none()
        && crate::detect::presets::is_ssr_framework(&build_artifact.detection.framework)
    {
        let msg = match compute {
            ComputeType::Process => {
                let mut message = format!(
                    "{} deploying as PROCESS (server runtime).",
                    build_artifact.detection.name
                );
                let hint = super::framework_static_hint(&build_artifact.detection.framework);
                if !hint.is_empty() {
                    message.push_str(&format!(
                        " For a fully static export, {hint} and redeploy with --compute static."
                    ));
                }
                message
            }
            ComputeType::Static => format!(
                "{} deploying as STATIC. \
                 For server-side rendering, use --compute process.",
                build_artifact.detection.name
            ),
        };
        output::warn(json, &msg, output::Phase::Deploy);
        warnings.push(msg);
    }

    let mut manifest_raw: Option<serde_json::Value> = build_artifact
        .manifest
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .context("failed to serialize manifest")?;

    if compute == ComputeType::Process && !has_build_manifest {
        super::validate_process_output(
            &build_artifact.output_dir,
            project_dir,
            &build_artifact.detection,
        )
        .map_err(|e| output::with_default_code(e, "MISSING_PROCESS_ENTRY"))?;
        let (entry, warning) = super::ensure_process_entry(
            &build_artifact.output_dir,
            project_dir,
            effective.deploy_entry(),
            &build_artifact.detection,
            json,
        )
        .map_err(|e| output::with_default_code(e, "MISSING_PROCESS_ENTRY"))?;
        if let Some(warning) = warning {
            output::warn(json, &warning, output::Phase::Deploy);
            warnings.push(warning);
        }
        match entry {
            Some(entry) => {
                let mut auto = build_manifest::generate_compute_manifest(&entry);
                let target = super::canonical_build_runtime_target(
                    &build_artifact.detection.metadata.runtime,
                    request.platform_runner,
                    Some(&auto),
                    build_artifact
                        .detection
                        .metadata
                        .application_runtime
                        .as_ref(),
                    effective.node_version(),
                )?;
                super::apply_application_runtime_manifest(
                    &mut auto,
                    build_artifact
                        .detection
                        .metadata
                        .application_runtime
                        .as_ref(),
                    target.as_deref(),
                    &build_artifact.detection.framework,
                )?;
                build_manifest::validate(&auto)
                    .map_err(|error| output::with_default_code(error, "INVALID_MANIFEST"))?;
                build_manifest::verify_files(&build_artifact.output_dir, &auto)
                    .map_err(|error| output::with_default_code(error, "MISSING_BUILD_OUTPUT"))?;
                output::status(
                    json,
                    "~",
                    format!("Auto-generated COMPUTE manifest (entry: {entry})"),
                    output::Phase::Deploy,
                );
                manifest_raw = Some(
                    serde_json::to_value(&auto)
                        .context("failed to serialize auto-generated manifest")?,
                );
                deployment_manifest_source = BuildManifestSource::Generated;
            }
            None => {
                return Err(output::coded_error(
                    "MISSING_PROCESS_ENTRY",
                    format!(
                        "Cannot auto-generate COMPUTE manifest: entry point not detected in {}.\n\n\
                         Create .onreza/manifest.json manually.\n\
                         See: docs.onreza.ru/manifest",
                        build_artifact.output_dir.display()
                    ),
                ));
            }
        }
    } else if compute == ComputeType::Static && manifest_raw.is_none() {
        deployment_manifest_source = BuildManifestSource::Generated;
    }

    let manifest_raw = super::resolve_manifest_for_compute(compute, manifest_raw)?;
    let manifest_for_planning: build_manifest::Manifest =
        serde_json::from_value(manifest_raw.clone())
            .context("failed to parse resolved deployment manifest")?;
    let mut runtime_artifact = super::resolve_runtime_artifact(
        &command.root_dir,
        project_dir,
        build_artifact.output_dir.clone(),
        manifest_for_planning,
        &build_artifact.detection,
        json,
    )?;
    if native_recipe == Some(crate::detect::native::NativeRecipe::FlutterWeb) {
        super::native_build::apply_flutter_static_cache_policy(&mut runtime_artifact.manifest);
    }
    let manifest_raw = super::conform_manifest_to_wire_contract(
        serde_json::to_value(&runtime_artifact.manifest)
            .context("failed to serialize runtime artifact manifest")?,
    )?;

    output::status(
        json,
        "~",
        "Scanning runtime artifact...",
        output::Phase::Deploy,
    );
    let runtime_artifact_root_for_scan = runtime_artifact.root_dir.clone();
    let runtime_artifact_scan = runtime_artifact.scan.clone();
    let scanned_files =
        scan_runtime_artifact_for_plan(runtime_artifact_root_for_scan, runtime_artifact_scan)
            .await?;

    let scanned_files = if let Some(platform) = runtime_platform
        .as_ref()
        .filter(|_| compute != ComputeType::Static)
    {
        let pruned = crate::artifact::native_dependencies::prune_optional_native_dependencies(
            &runtime_artifact.root_dir,
            scanned_files,
            platform,
        )?;
        if pruned.packages > 0 {
            output::status(
                json,
                "~",
                format!(
                    "Excluded {} incompatible optional runtime packages ({} bytes)",
                    pruned.packages, pruned.bytes
                ),
                output::Phase::Deploy,
            );
        }
        pruned.files
    } else {
        scanned_files
    };

    let artifact_files = super::prepare_artifact_files(
        &runtime_artifact.manifest,
        scanned_files,
        &build_artifact.detection,
        artifact_root_scope(&runtime_artifact.root_dir, project_dir),
        &runtime_artifact.scan,
        json,
    );
    let files = artifact_files.deployable_entries();
    if files.is_empty() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "output directory has no deployable files after framework normalization: {}",
                runtime_artifact.root_dir.display()
            ),
        ));
    }
    super::ensure_no_unresolved_lfs_pointers(
        &runtime_artifact.root_dir,
        &files,
        effective.git_lfs_enabled(),
    )?;
    let file_breakdown = runtime_artifact.scan.file_breakdown(&files);
    emit_runtime_artifact_file_breakdown(json, &file_breakdown, request.build_logs);

    let functions =
        super::build_functions_payload(effective.config(), project_dir, json, args.force_rules)
            .await?;
    let has_compute_layer = super::manifest_has_compute_layer(&runtime_artifact.manifest);
    let health_check = if has_compute_layer {
        Some(super::resolve_health_check(
            args.health_check_path.as_deref(),
            &command.config,
            project_dir,
            &build_artifact.detection,
            &build_artifact.output_dir,
            json,
        )?)
    } else {
        None
    };

    Ok(DeployPlan {
        artifact: ArtifactPlan {
            build: build_artifact,
            runtime: runtime_artifact,
            files: artifact_files,
            file_breakdown,
        },
        compute,
        build_command,
        build_skipped: args.skip_build,
        production,
        manifest_raw,
        manifest_source: deployment_manifest_source,
        files,
        functions,
        has_compute_layer,
        health_check,
        warnings,
    })
}

fn emit_runtime_artifact_file_breakdown(
    json: bool,
    breakdown: &RuntimeArtifactFileBreakdown,
    build_logs: Option<&super::BuildLogEmitter>,
) {
    let categories = breakdown
        .categories()
        .map(|(label, count)| format!("{label}: {count}"))
        .collect::<Vec<_>>();
    let summary = format!(
        "Runtime artifact files: {}; total: {}",
        categories.join(", "),
        breakdown.total
    );
    if json {
        output::log_info_structured(
            output::Phase::Deploy.as_str(),
            &summary,
            &serde_json::json!({ "runtimeArtifactFiles": breakdown }),
        );
    } else {
        output::status(false, "~", &summary, output::Phase::Deploy);
    }
    if let Some(build_logs) = build_logs {
        build_logs.info(super::BuildLogPhase::Detect, &summary);
    }
    if breakdown.includes_installed_dependencies() {
        const DEPENDENCY_EXPLANATION: &str = "PROCESS runtime includes installed dependencies because the application entry resolves packages at runtime; Output Directory only selects the build output.";
        output::status(json, "~", DEPENDENCY_EXPLANATION, output::Phase::Deploy);
        if let Some(build_logs) = build_logs {
            build_logs.info(super::BuildLogPhase::Detect, DEPENDENCY_EXPLANATION);
        }
    }
}

pub(super) fn contextualize_missing_build_output(
    error: anyhow::Error,
    build_command: Option<&str>,
    build_skipped: bool,
) -> anyhow::Error {
    let is_missing_output = error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<output::CodedError>())
        .any(|coded| coded.code == "MISSING_BUILD_OUTPUT");
    if !is_missing_output {
        return error;
    }

    let guidance = if build_skipped {
        "Build execution was skipped, but no prebuilt deployment output was found. \
         Run the build first or deploy again without `--skip-build`."
    } else if build_command.is_none() {
        "No build command was configured or detected, and no prebuilt deployment output was found. \
         Add a package build script, pass `--build-command`, or set `[build].command` in onreza.toml."
    } else {
        return error;
    };

    output::coded_error("MISSING_BUILD_OUTPUT", format!("{guidance}\n\n{error}"))
}

fn artifact_root_scope(runtime_root: &Path, project_dir: &Path) -> ArtifactRootScope {
    let runtime_root = runtime_root
        .canonicalize()
        .unwrap_or_else(|_| runtime_root.to_path_buf());
    let project_dir = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    if runtime_root == project_dir {
        ArtifactRootScope::ProjectRoot
    } else {
        ArtifactRootScope::BuildOutput
    }
}
