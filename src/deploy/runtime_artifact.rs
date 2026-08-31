use super::*;

const MANIFEST_RUNTIME_MEMORY_MB_MIN: i64 = 32;
const MANIFEST_RUNTIME_MEMORY_MB_MAX: i64 = 8192;

pub(super) struct NodeProjectRuntimePlan {
    runtime_root: PathBuf,
    build_output_prefix: String,
    project_prefix: String,
}

pub(super) fn resolve_runtime_artifact(
    workspace_root_dir: &Path,
    project_dir: &Path,
    build_output_dir: PathBuf,
    manifest: build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
    json: bool,
) -> anyhow::Result<RuntimeArtifact> {
    let Some(plan) = plan_node_project_runtime_artifact(
        workspace_root_dir,
        project_dir,
        &build_output_dir,
        &manifest,
        detection,
    ) else {
        let scan = if detection.metadata.runtime.runtime_type == RuntimeType::Node
            && manifest_has_compute_layer(&manifest)
        {
            RuntimeArtifactScan::NodeRuntimeRoot
        } else {
            RuntimeArtifactScan::All
        };
        return Ok(RuntimeArtifact {
            root_dir: build_output_dir,
            manifest,
            scan,
        });
    };

    validate_node_project_runtime_dependencies(&plan.runtime_root, project_dir)?;
    let manifest = rewrite_manifest_for_node_project_runtime(manifest, &plan.build_output_prefix)?;
    build_manifest::verify_files(&plan.runtime_root, &manifest)
        .map_err(|e| output::with_default_code(e, "MISSING_BUILD_OUTPUT"))?;
    let roots = node_project_runtime_scan_roots(
        &plan.runtime_root,
        &plan.project_prefix,
        &plan.build_output_prefix,
    );
    let symlink_roots = workspace_package_runtime_roots(&plan.runtime_root);

    let runtime_root_label = if plan.runtime_root == workspace_root_dir {
        "workspace root"
    } else {
        "project root"
    };
    output::status(
        json,
        "~",
        format!(
            "Runtime artifact: Node {runtime_root_label} (entry and dependencies share one runtime root)"
        ),
        output::Phase::Deploy,
    );

    Ok(RuntimeArtifact {
        root_dir: plan.runtime_root,
        manifest,
        scan: RuntimeArtifactScan::Selected {
            roots,
            symlink_roots,
        },
    })
}

/// Plans relocation of a Node PROCESS deploy onto a runtime root that carries
/// `node_modules`. Returns `None` — scan the build output as-is — when the
/// project isn't an eligible Node server project, or when the build output lives
/// outside the runtime root (e.g. an out-of-tree `outputDirectory`).
pub(super) fn plan_node_project_runtime_artifact(
    workspace_root_dir: &Path,
    project_dir: &Path,
    build_output_dir: &Path,
    manifest: &build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
) -> Option<NodeProjectRuntimePlan> {
    if !is_node_project_runtime_candidate(project_dir, build_output_dir, manifest, detection) {
        return None;
    }
    let runtime_root = select_node_project_runtime_root(workspace_root_dir, project_dir);
    let build_output_prefix =
        relative_runtime_artifact_path(&runtime_root, build_output_dir).ok()?;
    let project_prefix = relative_runtime_artifact_path(&runtime_root, project_dir).ok()?;
    Some(NodeProjectRuntimePlan {
        runtime_root,
        build_output_prefix,
        project_prefix,
    })
}

pub(super) fn is_node_project_runtime_candidate(
    project_dir: &Path,
    build_output_dir: &Path,
    manifest: &build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
) -> bool {
    if detection.metadata.runtime.runtime_type != RuntimeType::Node {
        return false;
    }
    if !manifest_has_compute_layer(manifest) {
        return false;
    }
    if compute_layer_count(manifest) != 1 {
        return false;
    }
    if build_output_dir == project_dir || build_output_dir.join("node_modules").is_dir() {
        return false;
    }
    is_node_project_runtime_framework(&detection.framework)
}

pub(super) fn compute_layer_count(manifest: &build_manifest::Manifest) -> usize {
    manifest
        .layers
        .iter()
        .filter(|layer| layer.target == build_manifest::LayerTarget::Compute)
        .count()
}

pub(super) fn is_node_project_runtime_framework(framework: &str) -> bool {
    if matches!(framework, "nextjs" | "blitzjs" | "payload" | "nitro") {
        return false;
    }
    // These adapters emit Node entrypoints that still resolve packages from the
    // installed project runtime instead of producing a self-contained output.
    if matches!(framework, "astro" | "sveltekit" | "remix" | "react-router") {
        return true;
    }
    if framework == "other" {
        return true;
    }
    crate::detect::presets::get_preset_by_slug(framework)
        .is_some_and(|preset| preset.category == crate::detect::types::PresetCategory::Server)
}

pub(super) fn select_node_project_runtime_root(
    workspace_root_dir: &Path,
    project_dir: &Path,
) -> PathBuf {
    if workspace_root_dir != project_dir
        && project_dir.starts_with(workspace_root_dir)
        && workspace_root_dir.join("node_modules").is_dir()
    {
        // Node resolves modules by walking parent directories. In workspaces,
        // root node_modules is part of the app runtime even when the app also
        // has its own node_modules.
        return workspace_root_dir.to_path_buf();
    }
    project_dir.to_path_buf()
}

pub(super) fn validate_node_project_runtime_dependencies(
    runtime_root: &Path,
    project_dir: &Path,
) -> anyhow::Result<()> {
    let Some(package_json) = crate::detect::package_json::PackageJson::load_strict(project_dir)?
    else {
        return Ok(());
    };
    if package_json.dependencies.is_empty() {
        return Ok(());
    }
    if project_dir.join("node_modules").is_dir() || runtime_root.join("node_modules").is_dir() {
        return Ok(());
    }
    Err(output::coded_error(
        "MISSING_RUNTIME_DEPENDENCIES",
        format!(
            "Node PROCESS runtime artifact requires node_modules, but none was found in {} or {}. \
             Run the install step before deploy, or remove --skip-install.",
            project_dir.display(),
            runtime_root.display()
        ),
    ))
}

pub(super) fn rewrite_manifest_for_node_project_runtime(
    mut manifest: build_manifest::Manifest,
    build_output_prefix: &str,
) -> anyhow::Result<build_manifest::Manifest> {
    for layer in &mut manifest.layers {
        match layer.target {
            build_manifest::LayerTarget::Compute => {
                let entry = layer
                    .entry
                    .as_deref()
                    .context("COMPUTE layer missing entry")?;
                let entry = join_runtime_artifact_paths(
                    &join_runtime_artifact_paths(build_output_prefix, &layer.directory)?,
                    entry,
                )?;
                layer.directory = ".".to_string();
                layer.entry = Some(entry);
            }
            build_manifest::LayerTarget::Static => {
                layer.directory =
                    join_runtime_artifact_paths(build_output_prefix, &layer.directory)?;
            }
        }
    }
    build_manifest::validate(&manifest)
        .map_err(|e| output::with_default_code(e, "INVALID_MANIFEST"))?;
    Ok(manifest)
}

pub(super) fn relative_runtime_artifact_path(root: &Path, path: &Path) -> anyhow::Result<String> {
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "{} is not inside runtime root {}",
            path.display(),
            root.display()
        )
    })?;
    path_to_runtime_artifact_string(relative)
}

pub(super) fn path_to_runtime_artifact_string(path: &Path) -> anyhow::Result<String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("unsafe runtime artifact path: {}", path.display());
            }
        }
    }
    if out.as_os_str().is_empty() {
        Ok(".".to_string())
    } else {
        Ok(out.to_string_lossy().replace('\\', "/"))
    }
}

pub(super) fn normalize_runtime_artifact_path(path: &str) -> anyhow::Result<String> {
    path_to_runtime_artifact_string(Path::new(path))
}

pub(super) fn join_runtime_artifact_paths(left: &str, right: &str) -> anyhow::Result<String> {
    let left = normalize_runtime_artifact_path(left)?;
    let right = normalize_runtime_artifact_path(right)?;
    match (left.as_str(), right.as_str()) {
        (".", ".") => Ok(".".to_string()),
        (".", _) => Ok(right),
        (_, ".") => Ok(left),
        _ => Ok(format!("{left}/{right}")),
    }
}

pub(super) fn node_project_runtime_scan_roots(
    runtime_root: &Path,
    project_prefix: &str,
    build_output_prefix: &str,
) -> Vec<crate::artifact::RuntimeArtifactScanRoot> {
    let mut roots = Vec::new();
    push_existing_runtime_scan_root(
        &mut roots,
        runtime_root,
        build_output_prefix,
        crate::artifact::RuntimeArtifactScanRootKind::BuildOutput,
    );
    // Ship the whole node_modules tree. The transitive dependency closure can't
    // be pruned without a package-manager-aware resolver, and under-shipping
    // breaks the process at runtime — over-shipping is the safe trade-off.
    push_existing_runtime_scan_root(
        &mut roots,
        runtime_root,
        "node_modules",
        crate::artifact::RuntimeArtifactScanRootKind::NodeModules,
    );
    for file in crate::artifact::NODE_RUNTIME_METADATA_FILES {
        push_existing_runtime_scan_root(
            &mut roots,
            runtime_root,
            file,
            crate::artifact::RuntimeArtifactScanRootKind::Metadata,
        );
    }

    if project_prefix != "." {
        push_existing_runtime_scan_root(
            &mut roots,
            runtime_root,
            &join_runtime_artifact_paths(project_prefix, "node_modules")
                .expect("project node_modules path must be safe"),
            crate::artifact::RuntimeArtifactScanRootKind::NodeModules,
        );
        for file in crate::artifact::NODE_RUNTIME_METADATA_FILES {
            push_existing_runtime_scan_root(
                &mut roots,
                runtime_root,
                &join_runtime_artifact_paths(project_prefix, file)
                    .expect("project metadata path must be safe"),
                crate::artifact::RuntimeArtifactScanRootKind::Metadata,
            );
        }
    }

    roots
}

fn workspace_package_runtime_roots(runtime_root: &Path) -> Vec<String> {
    let local_fs = crate::detect::fs::LocalFs::new(runtime_root);
    let package_json = crate::detect::package_json::PackageJson::load_from_fs(&local_fs);
    let package_manager =
        crate::detect::package_manager::detect_package_manager(&local_fs, package_json.as_ref());
    let Some(monorepo) = crate::detect::monorepo::detect_monorepo(
        &local_fs,
        package_json.as_ref(),
        package_manager.as_ref(),
    ) else {
        return Vec::new();
    };

    monorepo
        .packages
        .into_iter()
        .filter_map(|package| normalize_runtime_artifact_path(&package.path).ok())
        .collect()
}

pub(super) fn push_existing_runtime_scan_root(
    roots: &mut Vec<crate::artifact::RuntimeArtifactScanRoot>,
    runtime_root: &Path,
    path: &str,
    kind: crate::artifact::RuntimeArtifactScanRootKind,
) {
    if roots.iter().any(|existing| existing.path == path) {
        return;
    }
    if runtime_root.join(path).exists() {
        roots.push(crate::artifact::RuntimeArtifactScanRoot {
            path: path.to_string(),
            kind,
        });
    }
}

// ── PROCESS validation ───────────────────────────────────────

pub(super) fn manifest_has_compute_layer(manifest: &build_manifest::Manifest) -> bool {
    manifest
        .layers
        .iter()
        .any(|layer| layer.target == build_manifest::LayerTarget::Compute)
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
///   covers `--compute static` overrides for projects detected as Process.
/// - PROCESS without manifest → `validate_compute_manifest_contract` bails with a
///   user-facing error (PROCESS auto-gen runs earlier in `run()`, so this case
///   here means PROCESS auto-gen failed somewhere upstream).
///
/// Replaces the `manifest_raw.expect(...)` runtime invariant with a typed
/// signature, so missing-manifest bugs surface at type-check time on call-sites.
pub(super) fn resolve_manifest_for_compute(
    compute: ComputeType,
    manifest_raw: Option<serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    if let Some(manifest) = manifest_raw {
        validate_compute_manifest_contract(compute, true)?;
        return Ok(manifest);
    }

    if compute == ComputeType::Static {
        let auto = build_manifest::generate_static_manifest();
        return serde_json::to_value(&auto)
            .context("failed to serialize auto-generated STATIC manifest");
    }

    // PROCESS without manifest: defer to validate for the user-facing message.
    validate_compute_manifest_contract(compute, false)?;
    // validate_compute_manifest_contract must return Err for these; reaching here
    // means its contract was changed. Surface as a bug, not a panic.
    bail!(
        "Internal error: validate_compute_manifest_contract accepted {compute:?} without a manifest.\n\
         This is a CLI bug — please report at github.com/onreza/nrz-cli/issues."
    );
}

/// Wire boundary for the build manifest. `CreateDeploymentBody.manifest` is parsed
/// by the platform against its `ManifestSchema`, so the bytes the CLI sends must
/// match the server contract regardless of how the manifest is modelled internally.
/// Round-tripping through the generated contract type rejects unknown or legacy
/// fields (`deny_unknown_fields`) and enforces the schema's limits before anything
/// leaves the CLI; the internal `build_manifest::Manifest` stays ergonomic.
pub(super) fn conform_manifest_to_wire_contract(
    manifest: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let wire: nrz_contract::manifest::OnrezaBuildOutputManifest = serde_json::from_value(manifest)
        .map_err(|error| {
            anyhow::anyhow!(
                "built manifest does not match the server manifest contract: {error}.\n\
                 This nrz may be behind the platform — try upgrading with `nrz upgrade`."
            )
        })?;
    // typify currently leaves this bounded JSON Schema integer as i64, so
    // re-apply the generated contract's inclusive range at the CLI boundary.
    for layer in &wire.layers {
        if let nrz_contract::manifest::OnrezaBuildOutputManifestLayersItem::Compute {
            name,
            runtime: Some(runtime),
            ..
        } = layer
            && let Some(memory_mb) = runtime.memory_mb
            && !(MANIFEST_RUNTIME_MEMORY_MB_MIN..=MANIFEST_RUNTIME_MEMORY_MB_MAX)
                .contains(&memory_mb)
        {
            bail!(
                "built manifest does not match the server manifest contract: layer '{}' runtime.memoryMb must be between {} and {}",
                name.as_str(),
                MANIFEST_RUNTIME_MEMORY_MB_MIN,
                MANIFEST_RUNTIME_MEMORY_MB_MAX,
            );
        }
    }
    serde_json::to_value(&wire).context("failed to serialize wire manifest")
}

/// Wire boundary for the ONREZA Functions publish payload, which rides in
/// the deployment source request and is re-validated by the platform against its
/// `FunctionPublishPayloadSchema`. Round-trip through the generated contract type so an
/// unknown field or a bad origin is rejected before the bytes leave the CLI. The edge
/// rule set is passed through opaquely — the platform owns its validation.
pub(super) fn conform_functions_to_wire_contract(
    payload: Option<crate::functions::FunctionPublishPayload>,
) -> anyhow::Result<Option<serde_json::Value>> {
    let Some(payload) = payload else {
        return Ok(None);
    };
    let value =
        serde_json::to_value(&payload).context("failed to serialize functions publish payload")?;
    let wire: nrz_contract::onreza_functions_publish::OnrezaFunctionsPublishPayloadV1 =
        serde_json::from_value(value).map_err(|error| {
            anyhow::anyhow!(
                "functions publish payload does not match the server contract: {error}.\n\
                 This nrz may be behind the platform — try upgrading with `nrz upgrade`."
            )
        })?;
    Ok(Some(
        serde_json::to_value(&wire).context("failed to serialize wire functions payload")?,
    ))
}

pub(super) fn validate_compute_manifest_contract(
    compute: ComputeType,
    has_manifest: bool,
) -> anyhow::Result<()> {
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
pub(super) fn framework_static_hint(framework: &str) -> &'static str {
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
pub(super) fn validate_process_output(
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

/// Pre-build validation for PROCESS deployments.
///
/// This only checks package-level signals because it runs before the build
/// artifact exists. Output marker checks stay in `validate_process_output`.
pub(super) fn validate_prebuild_process_project(project_dir: &Path) -> anyhow::Result<()> {
    if let Some(msg) = detect_workers_runtime_package_target(project_dir)? {
        return Err(output::coded_error("FRAMEWORK_UNSUPPORTED", msg));
    }

    Ok(())
}

pub(super) fn validate_prebuild_compute_intent(
    project_dir: &Path,
    explicit_compute: Option<ComputeType>,
) -> anyhow::Result<()> {
    if explicit_compute == Some(ComputeType::Process) {
        validate_prebuild_process_project(project_dir)?;
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
pub(super) fn detect_workers_runtime_target(
    project_dir: &Path,
    output_dir: &Path,
) -> anyhow::Result<Option<String>> {
    if let Some(msg) = detect_workers_runtime_package_target(project_dir)? {
        return Ok(Some(msg));
    }

    detect_workers_runtime_output_target(output_dir)
}

pub(super) fn detect_workers_runtime_package_target(
    project_dir: &Path,
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

    if !has_cf_plugin && !has_mini_oxygen {
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
    } else {
        unreachable!("package-level Workers runtime detector has no matching signal");
    };

    Ok(Some(workers_runtime_message(runtime, trigger, remedy)))
}

pub(super) fn detect_workers_runtime_output_target(
    output_dir: &Path,
) -> anyhow::Result<Option<String>> {
    let has_wrangler_output = output_dir.join("server/wrangler.json").is_file();
    let has_oxygen_output = output_dir.join("server/oxygen.json").is_file();

    if !has_wrangler_output && !has_oxygen_output {
        return Ok(None);
    }

    let (runtime, trigger, remedy) = if has_wrangler_output {
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

    Ok(Some(workers_runtime_message(runtime, trigger, remedy)))
}

pub(super) fn workers_runtime_message(runtime: &str, trigger: &str, remedy: &str) -> String {
    format!(
        "{runtime} target detected ({trigger}).\n\n\
         ONREZA PROCESS compute runs Node/Bun servers, not the Workers runtime (workerd), \
         so this build cannot be deployed as-is.\n\n\
         Pick one:\n\
         \x20 1. Deploy as static (if your app has no server functions):\n\
         \x20    nrz deploy --compute static\n\n\
         \x20 2. Switch to a Node server build.\n\
         \x20    {remedy}"
    )
}

/// Framework-specific diagnostic when entry point resolution fails.
pub(super) fn framework_process_diagnostic(
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
        "astro" => Some(format!(
            "Astro Node adapter PROCESS deployment expects server/entry.mjs in {}.\n\n\
             Configure @astrojs/node with mode: 'standalone', run `astro build`, and keep \
             the default dist output so the build creates dist/server/entry.mjs.",
            output_dir.display()
        )),
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
pub(super) fn is_strict_process_framework(framework: &str) -> bool {
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

pub(super) fn is_windows_drive_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

pub(super) fn sanitize_config_entry(entry: &str) -> anyhow::Result<String> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        bail!("[deploy] entry in onreza.toml must not be empty");
    }
    if looks_like_shell_command_entry(trimmed) {
        bail!(
            "[deploy] entry must be a relative file path inside the build output, not a shell command. Use entry = \"index.js\" instead of \"{entry}\"."
        );
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

pub(super) fn looks_like_shell_command_entry(entry: &str) -> bool {
    let mut words = entry.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };
    words.next().is_some()
        && matches!(
            first,
            "node" | "bun" | "deno" | "tsx" | "ts-node" | "npm" | "pnpm" | "yarn" | "npx"
        )
}

/// Resolve and ensure entry point for PROCESS deployments.
///
/// 1. Resolve entry: config `[deploy] entry` > framework auto-detect
/// 2. Validate file existence when entry is resolved
/// 3. If unresolved, return an actionable error.
pub(super) fn ensure_process_entry(
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
        if detection.framework == "astro" && !output_dir.join("server/entry.mjs").is_file() {
            return Err(output::coded_error(
                "MISSING_PROCESS_ENTRY",
                framework_process_diagnostic(&detection.framework, detection, output_dir)
                    .expect("Astro PROCESS diagnostic must exist"),
            ));
        }
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

                return Err(output::coded_error(
                    "ENTRY_POINT_AMBIGUOUS",
                    format!(
                        "Entry point auto-detection is ambiguous for PROCESS deployment ({} candidates in {}).\n\
                     Set [deploy] entry in onreza.toml to make startup explicit.",
                        candidates.len(),
                        output_dir.display()
                    ),
                ));
            }
            crate::detect::EntryPointResolution::NotFound => {
                if is_strict_process_framework(&detection.framework)
                    && let Some(diagnostic) =
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
                missing_entrypoint_message(entry, output_dir),
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

pub(super) const MISPLACED_ENTRYPOINT_MAX_DEPTH: usize = 4;
pub(super) const MISPLACED_ENTRYPOINT_MAX_MATCHES: usize = 5;

pub(super) fn missing_entrypoint_message(entry: &str, output_dir: &Path) -> String {
    let mut message = format!(
        "Entry point \"{entry}\" not found in output directory: {}\n\n\
         Make sure the file exists after running your build command.",
        output_dir.display()
    );

    let candidates = find_nested_entrypoint_candidates(output_dir, entry);
    if candidates.is_empty() {
        return message;
    }

    message.push_str("\n\nFound matching entry point file outside the selected output root:");
    for candidate in &candidates {
        message.push_str(&format!("\n  - {candidate}"));
    }

    if let Some(output_hint) = output_directory_hint_for_nested_entry(&candidates[0], entry) {
        message.push_str(&format!(
            "\n\nThis usually means outputDirectory points at {} while the build emits a nested deploy artifact.\n\
             Set [build] output_directory = \"{output_hint}\" and keep [deploy] entry = \"{entry}\".",
            output_dir.display()
        ));
    } else {
        message.push_str(
            "\n\nThis usually means outputDirectory and [deploy] entry describe different roots. \
             Point outputDirectory at the directory that contains the entry point.",
        );
    }

    message
}

pub(super) fn output_directory_hint_for_nested_entry(
    candidate: &str,
    entry: &str,
) -> Option<String> {
    let candidate = Path::new(candidate);
    let entry_depth = Path::new(entry).components().count();
    let mut output_dir = candidate;
    for _ in 0..entry_depth {
        output_dir = output_dir.parent()?;
    }

    if output_dir.as_os_str().is_empty() {
        return None;
    }

    Some(output_dir.to_string_lossy().replace('\\', "/"))
}

pub(super) fn find_nested_entrypoint_candidates(output_dir: &Path, entry: &str) -> Vec<String> {
    let entry_path = Path::new(entry);
    let direct_entry = output_dir.join(entry_path);
    let mut matches = Vec::new();
    collect_nested_entrypoint_candidates(
        output_dir,
        output_dir,
        entry_path,
        &direct_entry,
        0,
        &mut matches,
    );
    matches.sort();
    matches.truncate(MISPLACED_ENTRYPOINT_MAX_MATCHES);
    matches
}

pub(super) fn collect_nested_entrypoint_candidates(
    base: &Path,
    current: &Path,
    entry: &Path,
    direct_entry: &Path,
    depth: usize,
    matches: &mut Vec<String>,
) {
    if matches.len() >= MISPLACED_ENTRYPOINT_MAX_MATCHES || depth >= MISPLACED_ENTRYPOINT_MAX_DEPTH
    {
        return;
    }

    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };

    for entry_result in entries {
        if matches.len() >= MISPLACED_ENTRYPOINT_MAX_MATCHES {
            return;
        }

        let Ok(dir_entry) = entry_result else {
            continue;
        };
        let path = dir_entry.path();
        let Ok(file_type) = dir_entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            if should_skip_misplaced_entrypoint_dir(&path) {
                continue;
            }
            collect_nested_entrypoint_candidates(
                base,
                &path,
                entry,
                direct_entry,
                depth + 1,
                matches,
            );
            continue;
        }

        if file_type.is_file()
            && path != direct_entry
            && path.ends_with(entry)
            && let Ok(rel) = path.strip_prefix(base)
        {
            matches.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

pub(super) fn should_skip_misplaced_entrypoint_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("node_modules" | ".git" | ".cache" | ".next" | "target")
    )
}
