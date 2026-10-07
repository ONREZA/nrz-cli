use super::*;

pub(super) struct NodeProjectRuntimePlan {
    runtime_root: PathBuf,
    build_output_prefix: String,
    project_prefix: String,
}

#[cfg(test)]
pub(super) fn resolve_runtime_artifact(
    workspace_root_dir: &Path,
    project_dir: &Path,
    build_output_dir: PathBuf,
    manifest: build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
    json: bool,
) -> anyhow::Result<RuntimeArtifact> {
    resolve_runtime_artifact_with_manifest_source(
        workspace_root_dir,
        project_dir,
        build_output_dir,
        manifest,
        detection,
        json,
        crate::artifact::BuildManifestSource::File,
    )
}

pub(super) fn resolve_runtime_artifact_with_manifest_source(
    workspace_root_dir: &Path,
    project_dir: &Path,
    build_output_dir: PathBuf,
    mut manifest: build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
    json: bool,
    manifest_source: crate::artifact::BuildManifestSource,
) -> anyhow::Result<RuntimeArtifact> {
    let python_minor = detection
        .metadata
        .source_build_context
        .as_ref()
        .and_then(|context| context.build_toolchain.resolved_python_minor())
        .or_else(|| {
            detection
                .metadata
                .application_runtime()
                .filter(|runtime| {
                    runtime.family == nrz_source_bundle::ApplicationRuntimeFamily::Python
                })
                .and_then(|runtime| runtime.python_version)
        })
        .unwrap_or_default();
    let python_runtime = detection
        .metadata
        .application_runtime()
        .is_some_and(|runtime| {
            runtime.family == nrz_source_bundle::ApplicationRuntimeFamily::Python
        })
        || manifest.layers.iter().any(|layer| {
            layer.target == build_manifest::LayerTarget::Compute
                && layer
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .is_some_and(|runtime| {
                        runtime.family == nrz_source_bundle::ApplicationRuntimeFamily::Python
                    })
        });
    let mut javascript_dependency_closure = false;
    let javascript_dependency_owner =
        declared_javascript_dependency_owner(project_dir, &build_output_dir, &manifest, detection);
    if python_runtime && manifest_has_compute_layer(&manifest) {
        let artifact = resolve_python_runtime_artifact(
            project_dir,
            build_output_dir.clone(),
            manifest,
            json,
            python_minor,
            manifest_source == crate::artifact::BuildManifestSource::Generated
                && detection
                    .metadata
                    .application_runtime()
                    .is_some_and(|runtime| {
                        runtime.family == nrz_source_bundle::ApplicationRuntimeFamily::Python
                            && runtime.entry.as_deref()
                                == Some(crate::detect::python_launch::PYTHON_BOOTSTRAP_ENTRY)
                    }),
            detection,
        )?;
        if validate_python_layer_dependencies(
            workspace_root_dir,
            project_dir,
            &artifact,
            detection,
            python_minor,
        )? {
            javascript_dependency_closure = true;
            manifest = artifact.manifest;
            if let RuntimeArtifactScan::Relocated { ownership, .. } = artifact.scan {
                manifest.layers = ownership.layers;
            }
        } else {
            validate_compute_entry_ownership(&artifact.manifest, &artifact.scan)?;
            return Ok(artifact);
        }
    }
    let Some(plan) = plan_node_project_runtime_artifact(
        workspace_root_dir,
        project_dir,
        &build_output_dir,
        &manifest,
        detection,
    ) else {
        let javascript_layer = manifest.layers.iter().any(|layer| {
            layer.target == build_manifest::LayerTarget::Compute
                && layer
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .is_some_and(|runtime| {
                        matches!(
                            runtime.family,
                            nrz_source_bundle::ApplicationRuntimeFamily::Node
                                | nrz_source_bundle::ApplicationRuntimeFamily::Bun
                        )
                    })
        });
        let dependency_root = select_node_project_runtime_root(workspace_root_dir, project_dir);
        let external_javascript_dependencies =
            javascript_runtime_dependency_roots(&build_output_dir, &dependency_root, &manifest)
                .iter()
                .any(|root| !root.starts_with(&build_output_dir));
        if external_javascript_dependencies
            || javascript_dependency_closure
                && dependency_root != build_output_dir
                && !build_output_dir.join("node_modules").is_dir()
        {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "JavaScript dependency closure requires a supported runtime root relocation; the declared layer layout cannot retain dependencies outside the build output",
            ));
        }
        let scan = if (javascript_layer
            || matches!(
                detection.metadata.runtime.runtime_type,
                RuntimeType::Node | RuntimeType::Bun
            ))
            && manifest_has_compute_layer(&manifest)
        {
            RuntimeArtifactScan::NodeRuntimeRoot
        } else {
            RuntimeArtifactScan::All
        };
        validate_javascript_dependency_owners(
            &build_output_dir,
            &build_output_dir,
            &manifest,
            &scan,
        )?;
        if compute_layer_count(&manifest) > 1 {
            validate_compute_entry_ownership(&manifest, &scan)?;
        }
        return Ok(RuntimeArtifact {
            root_dir: build_output_dir,
            manifest,
            scan,
        });
    };

    let ownership = crate::artifact::RuntimeArtifactSourceOwnership {
        build_output_prefix: plan.build_output_prefix.clone(),
        layers: manifest.layers.clone(),
        javascript_dependency_owner,
        python_dependency_owner: None,
        python_primary_declared: false,
    };
    let manifest =
        rewrite_manifest_for_project_runtime(manifest, &plan.build_output_prefix, false)?;
    validate_node_project_runtime_dependencies(&plan.runtime_root, project_dir, &manifest)?;
    build_manifest::verify_files(&plan.runtime_root, &manifest)
        .map_err(|e| output::with_default_code(e, "MISSING_BUILD_OUTPUT"))?;
    let mut roots = node_project_runtime_scan_roots(
        &plan.runtime_root,
        &plan.project_prefix,
        &plan.build_output_prefix,
    );
    for root in
        javascript_runtime_dependency_roots(&plan.runtime_root, &plan.runtime_root, &manifest)
    {
        push_existing_runtime_scan_root(
            &mut roots,
            &plan.runtime_root,
            &relative_runtime_artifact_path(&plan.runtime_root, &root)?,
            crate::artifact::RuntimeArtifactScanRootKind::NodeModules,
        );
    }
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
            "Runtime artifact: JavaScript {runtime_root_label} (entry and dependencies share one runtime root)"
        ),
        output::Phase::Deploy,
    );

    let scan = RuntimeArtifactScan::Relocated {
        base: Box::new(RuntimeArtifactScan::Selected {
            roots,
            symlink_roots,
        }),
        ownership,
    };
    validate_javascript_dependency_owners(
        &plan.runtime_root,
        &plan.runtime_root,
        &manifest,
        &scan,
    )?;
    if compute_layer_count(&manifest) > 1 {
        validate_compute_entry_ownership(&manifest, &scan)?;
    }
    Ok(RuntimeArtifact {
        root_dir: plan.runtime_root,
        manifest,
        scan,
    })
}

fn validate_compute_entry_ownership(
    manifest: &build_manifest::Manifest,
    scan: &RuntimeArtifactScan,
) -> anyhow::Result<()> {
    for layer in manifest
        .layers
        .iter()
        .filter(|layer| layer.target == build_manifest::LayerTarget::Compute)
    {
        let entry = layer
            .entry
            .as_deref()
            .context("COMPUTE layer missing entry")?;
        let path = join_runtime_artifact_paths(&layer.directory, entry)?;
        if scan
            .source_layer_match(manifest, &path)
            .map(|owner| owner.name.as_str())
            != Some(layer.name.as_str())
        {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                format!(
                    "COMPUTE entry '{path}' is not owned by its declared layer '{}'",
                    layer.name
                ),
            ));
        }
    }
    Ok(())
}

/// The automatic installer owns one Python minor. A typed serving layer cannot
/// replace that materialization ABI, including when the primary serves STATIC.
fn validate_python_layer_dependencies(
    workspace_root_dir: &Path,
    project_dir: &Path,
    artifact: &RuntimeArtifact,
    detection: &crate::detect::types::DetectionResult,
    minor: nrz_source_bundle::PythonMinor,
) -> anyhow::Result<bool> {
    use nrz_source_bundle::{ApplicationRuntimeFamily as Family, PythonMinor};
    let manifest = &artifact.manifest;
    let installed_minor = detection
        .metadata
        .source_build_context
        .as_ref()
        .and_then(|context| context.build_toolchain.resolved_python_minor());
    let primary = detection.metadata.application_runtime();
    let primary_javascript =
        primary.is_some_and(|runtime| matches!(runtime.family, Family::Node | Family::Bun));
    let python_materialization = installed_minor.is_some() && !primary_javascript
        || primary.is_some_and(|runtime| runtime.family == Family::Python);
    let authored_dependencies = python_materialization
        && crate::detect::python::requires_dependency_stage(&crate::detect::fs::LocalFs::new(
            project_dir,
        ))?;
    let staged = project_dir
        .join(minor.site_packages_root())
        .read_dir()
        .is_ok_and(|mut entries| entries.next().is_some());
    let scan = &artifact.scan;
    // Without a primary, an unambiguous typed owner can explicitly claim a
    // retained serving tree. A primary's code-only siblings do not claim stale trees.
    if primary.is_none() {
        for other_minor in PythonMinor::ALL
            .into_iter()
            .filter(|candidate| *candidate != minor)
        {
            let claimed = scan
                .source_layer_match(manifest, other_minor.site_packages_root())
                .is_some_and(|layer| {
                    layer.target == build_manifest::LayerTarget::Compute
                        && layer.runtime.as_ref().is_some_and(|runtime| {
                            runtime
                                .application_runtime
                                .as_ref()
                                .is_some_and(|intent| intent.family == Family::Python)
                                && runtime.build_runtime_version.as_deref()
                                    == Some(other_minor.target())
                        })
                });
            if claimed
                && project_dir
                    .join(other_minor.site_packages_root())
                    .read_dir()
                    .is_ok_and(|mut entries| entries.next().is_some())
            {
                return Err(output::coded_error(
                    "APPLICATION_RUNTIME_INVALID",
                    "Python runtime dependencies require matching build and serving Python minors; independent toolchains are supported for code-only output",
                ));
            }
        }
    }
    let staged_owner = scan.source_layer_match(manifest, minor.site_packages_root());
    let staged_runtime_dependencies = staged
        && (python_materialization
            || staged_owner
                .and_then(|layer| layer.runtime.as_ref())
                .and_then(|runtime| runtime.application_runtime.as_ref())
                .is_some_and(|runtime| runtime.family == Family::Python));
    let dependencies = authored_dependencies || staged_runtime_dependencies;
    if dependencies {
        let owner = scan.source_layer_match(manifest, minor.site_packages_root());
        let compatible = owner.is_some_and(|layer| {
            let runtime = layer.runtime.as_ref();
            let primary = detection.metadata.application_runtime();
            layer.target == build_manifest::LayerTarget::Compute
                && runtime
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .map(|runtime| runtime.family)
                    .or_else(|| primary.map(|runtime| runtime.family))
                    == Some(Family::Python)
                && runtime
                    .and_then(|runtime| runtime.build_runtime_version.as_deref())
                    .or_else(|| {
                        primary
                            .and_then(|runtime| runtime.python_version)
                            .map(PythonMinor::target)
                    })
                    == Some(minor.target())
        });
        if !compatible
            || detection.metadata.source_build_context.is_some() && installed_minor != Some(minor)
        {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                format!(
                    "Python dependency root '{}' conflicts with selected installer {} or owning layer '{}'; runtime dependency owners require matching build and serving Python minors",
                    minor.site_packages_root(),
                    minor.version(),
                    owner.map_or("<none>", |layer| layer.name.as_str())
                ),
            ));
        }
    }
    if authored_dependencies && !project_dir.join(minor.site_packages_root()).is_dir() {
        return Err(output::coded_error(
            "MISSING_RUNTIME_DEPENDENCIES",
            "Python PROCESS runtime requires installed dependencies. Run the install step before deploy, or remove --skip-install.",
        ));
    }
    let javascript_root = select_node_project_runtime_root(workspace_root_dir, project_dir);
    if !javascript_runtime_dependency_roots(&artifact.root_dir, &javascript_root, manifest)
        .is_empty()
    {
        if dependencies {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "Python and JavaScript runtime dependency trees require different materialization ABIs; mixed runtime layers are supported for code-only output",
            ));
        }
        let javascript_scan = match scan {
            RuntimeArtifactScan::Relocated { ownership, .. } => RuntimeArtifactScan::Relocated {
                base: Box::new(RuntimeArtifactScan::NodeRuntimeRoot),
                ownership: ownership.clone(),
            },
            _ => RuntimeArtifactScan::NodeRuntimeRoot,
        };
        validate_javascript_dependency_owners(
            &artifact.root_dir,
            &javascript_root,
            manifest,
            &javascript_scan,
        )?;
        // The Python sibling carries code only; retain the sole JavaScript closure.
        return Ok(true);
    }
    Ok(false)
}

/// Resolve authored/frozen file entries against their explicit manifest root,
/// then retain every Node/Bun lookup directory through the selected runtime root.
fn javascript_runtime_dependency_roots(
    manifest_root: &Path,
    runtime_root: &Path,
    manifest: &build_manifest::Manifest,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for layer in &manifest.layers {
        if layer.target != build_manifest::LayerTarget::Compute
            || !layer
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.application_runtime.as_ref())
                .is_some_and(|runtime| {
                    matches!(
                        runtime.family,
                        nrz_source_bundle::ApplicationRuntimeFamily::Node
                            | nrz_source_bundle::ApplicationRuntimeFamily::Bun
                    )
                })
        {
            continue;
        }
        let Some(entry) = &layer.entry else {
            continue;
        };
        let entry = manifest_root.join(&layer.directory).join(entry);
        for parent in entry
            .parent()
            .into_iter()
            .flat_map(Path::ancestors)
            .take_while(|parent| parent.starts_with(runtime_root))
        {
            let root = parent.join("node_modules");
            if root
                .read_dir()
                .is_ok_and(|mut entries| entries.next().is_some())
            {
                roots.push(root);
            }
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn declared_javascript_dependency_owner(
    project_dir: &Path,
    manifest_root: &Path,
    manifest: &build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
) -> Option<String> {
    let candidates = manifest
        .layers
        .iter()
        .filter(|layer| crate::artifact::is_javascript_compute_layer(layer))
        .collect::<Vec<_>>();
    if candidates.len() == 1 {
        return Some(candidates[0].name.clone());
    }
    declared_primary_dependency_owner(project_dir, manifest_root, candidates, detection, false)
}

fn declared_python_dependency_owner(
    project_dir: &Path,
    manifest_root: &Path,
    manifest: &build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
    project_owned_entry: bool,
) -> Option<String> {
    use nrz_source_bundle::ApplicationRuntimeFamily;
    let primary = detection.metadata.application_runtime();
    if primary.is_some_and(|runtime| runtime.family != ApplicationRuntimeFamily::Python) {
        // Python packages used only by a build tool do not become serving dependencies.
        return None;
    }
    let candidates = manifest
        .layers
        .iter()
        .filter(|layer| {
            layer.target == build_manifest::LayerTarget::Compute
                && layer
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .map(|intent| intent.family)
                    .or_else(|| primary.map(|runtime| runtime.family))
                    == Some(ApplicationRuntimeFamily::Python)
        })
        .collect::<Vec<_>>();
    if primary.is_none() && candidates.len() == 1 {
        return Some(candidates[0].name.clone());
    }
    declared_primary_dependency_owner(
        project_dir,
        manifest_root,
        candidates,
        detection,
        project_owned_entry,
    )
}

fn declared_primary_dependency_owner(
    project_dir: &Path,
    manifest_root: &Path,
    candidates: Vec<&build_manifest::Layer>,
    detection: &crate::detect::types::DetectionResult,
    project_owned_entry: bool,
) -> Option<String> {
    let primary = detection.metadata.application_runtime()?;
    let entry = if project_owned_entry {
        // Generated bootstrap provenance explicitly anchors this entry at the project.
        primary.entry.clone()?
    } else {
        crate::detect::resolve_application_entry(
            primary.entry.as_deref()?,
            manifest_root,
            project_dir,
        )?
    };
    let mut matching = candidates.into_iter().filter(|layer| {
        layer
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.application_runtime.as_ref())
            .map(|intent| intent.family)
            .unwrap_or(primary.family)
            == primary.family
            && layer
                .entry
                .as_deref()
                .and_then(|layer_entry| {
                    join_runtime_artifact_paths(&layer.directory, layer_entry).ok()
                })
                .as_deref()
                == Some(entry.as_str())
    });
    let owner = matching.next()?;
    matching.next().is_none().then(|| owner.name.clone())
}

fn validate_javascript_dependency_owners(
    manifest_root: &Path,
    runtime_root: &Path,
    manifest: &build_manifest::Manifest,
    scan: &RuntimeArtifactScan,
) -> anyhow::Result<()> {
    let mut dependency_owner = None;
    for root in javascript_runtime_dependency_roots(manifest_root, runtime_root, manifest) {
        // Dependencies hoisted outside a project-root Python planning artifact
        // retain the existing external-root fallback; output-local trees use
        // their recorded relocation ownership before normalization.
        let relative = root
            .strip_prefix(manifest_root)
            .ok()
            .map(path_to_runtime_artifact_string)
            .transpose()?
            .unwrap_or_else(|| "node_modules".into());
        let owner = scan.source_layer_match(manifest, &relative);
        if !owner
            .and_then(|layer| layer.runtime.as_ref())
            .and_then(|runtime| runtime.application_runtime.as_ref())
            .is_some_and(|runtime| {
                matches!(
                    runtime.family,
                    nrz_source_bundle::ApplicationRuntimeFamily::Node
                        | nrz_source_bundle::ApplicationRuntimeFamily::Bun
                )
            })
        {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                format!(
                    "JavaScript dependency root '{relative}' is not owned by a JavaScript runtime layer"
                ),
            ));
        }
        let owner = owner.unwrap();
        if dependency_owner.is_some_and(|name| name != owner.name) {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "JavaScript runtime dependencies require a single owning layer and materialization ABI; sibling runtime layers must remain code-only",
            ));
        }
        dependency_owner = Some(owner.name.as_str());
    }
    Ok(())
}

fn resolve_python_runtime_artifact(
    project_dir: &Path,
    build_output_dir: PathBuf,
    manifest: build_manifest::Manifest,
    json: bool,
    minor: nrz_source_bundle::PythonMinor,
    project_owned_bootstrap: bool,
    detection: &crate::detect::types::DetectionResult,
) -> anyhow::Result<RuntimeArtifact> {
    let build_output_prefix = relative_runtime_artifact_path(project_dir, &build_output_dir)
        .map_err(|_| {
            output::coded_error(
                "INVALID_BUILD_OUTPUT",
                "Python output directory must be inside the project directory",
            )
        })?;
    let ownership =
        crate::artifact::RuntimeArtifactSourceOwnership {
            build_output_prefix: build_output_prefix.clone(),
            layers: manifest.layers.clone(),
            javascript_dependency_owner: declared_javascript_dependency_owner(
                project_dir,
                &build_output_dir,
                &manifest,
                detection,
            ),
            python_dependency_owner: declared_python_dependency_owner(
                project_dir,
                &build_output_dir,
                &manifest,
                detection,
                project_owned_bootstrap,
            ),
            python_primary_declared: detection.metadata.application_runtime().is_some_and(
                |runtime| runtime.family == nrz_source_bundle::ApplicationRuntimeFamily::Python,
            ),
        };
    let manifest = if build_output_prefix == "." {
        manifest
    } else {
        rewrite_manifest_for_project_runtime(
            manifest,
            &build_output_prefix,
            project_owned_bootstrap,
        )?
    };
    output::status(
        json,
        "~",
        format!("Runtime artifact: CPython {} project root", minor.version()),
        output::Phase::Deploy,
    );
    Ok(RuntimeArtifact {
        root_dir: project_dir.to_path_buf(),
        manifest,
        scan: RuntimeArtifactScan::Relocated {
            base: Box::new(RuntimeArtifactScan::PythonRuntimeRoot(minor)),
            ownership,
        },
    })
}

/// Plans relocation of a Node/Bun PROCESS deploy onto a runtime root that carries
/// `node_modules`. Returns `None` — scan the build output as-is — when the
/// project isn't an eligible JavaScript server project, or when the build output lives
/// outside the runtime root (e.g. an out-of-tree `outputDirectory`).
pub(super) fn plan_node_project_runtime_artifact(
    workspace_root_dir: &Path,
    project_dir: &Path,
    build_output_dir: &Path,
    manifest: &build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
) -> Option<NodeProjectRuntimePlan> {
    let runtime_root = select_node_project_runtime_root(workspace_root_dir, project_dir);
    if !is_node_project_runtime_candidate(
        project_dir,
        build_output_dir,
        &runtime_root,
        manifest,
        detection,
    ) {
        return None;
    }
    if build_output_dir == project_dir && runtime_root == project_dir {
        return None;
    }
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
    runtime_root: &Path,
    manifest: &build_manifest::Manifest,
    detection: &crate::detect::types::DetectionResult,
) -> bool {
    let primary_javascript = detection
        .metadata
        .application_runtime()
        .is_some_and(|runtime| {
            matches!(
                runtime.family,
                nrz_source_bundle::ApplicationRuntimeFamily::Node
                    | nrz_source_bundle::ApplicationRuntimeFamily::Bun
            )
        });
    let typed_javascript = manifest.layers.iter().any(|layer| {
        layer.target == build_manifest::LayerTarget::Compute
            && layer
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.application_runtime.as_ref())
                .is_some_and(|intent| {
                    matches!(
                        intent.family,
                        nrz_source_bundle::ApplicationRuntimeFamily::Node
                            | nrz_source_bundle::ApplicationRuntimeFamily::Bun
                    )
                })
    });
    if !primary_javascript
        && !typed_javascript
        && !matches!(
            detection.metadata.runtime.runtime_type,
            RuntimeType::Node | RuntimeType::Bun
        )
    {
        return false;
    }
    if !manifest_has_compute_layer(manifest) {
        return false;
    }
    if compute_layer_count(manifest) != 1 {
        let compute_layers = manifest
            .layers
            .iter()
            .filter(|layer| layer.target == build_manifest::LayerTarget::Compute)
            .collect::<Vec<_>>();
        if compute_layers.iter().any(|layer| {
            layer
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.application_runtime.as_ref())
                .is_none()
        }) || compute_layers.iter().enumerate().any(|(index, layer)| {
            compute_layers[index + 1..]
                .iter()
                .any(|sibling| sibling.directory == layer.directory)
        }) {
            return false;
        }
    }
    if build_output_dir != project_dir
        && build_output_dir.join("node_modules").is_dir()
        && !javascript_runtime_dependency_roots(build_output_dir, runtime_root, manifest)
            .iter()
            .any(|root| !root.starts_with(build_output_dir))
    {
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
        // Node and Bun resolve modules by walking parent directories. In workspaces,
        // root node_modules is part of the app runtime even when the app also
        // has its own node_modules.
        return workspace_root_dir.to_path_buf();
    }
    project_dir.to_path_buf()
}

pub(super) fn validate_node_project_runtime_dependencies(
    runtime_root: &Path,
    project_dir: &Path,
    manifest: &build_manifest::Manifest,
) -> anyhow::Result<()> {
    let Some(package_json) = crate::detect::package_json::PackageJson::load_strict(project_dir)?
    else {
        return Ok(());
    };
    if package_json.dependencies.is_empty() {
        return Ok(());
    }
    if project_dir.join("node_modules").is_dir()
        || runtime_root.join("node_modules").is_dir()
        || !javascript_runtime_dependency_roots(runtime_root, runtime_root, manifest).is_empty()
    {
        return Ok(());
    }
    Err(output::coded_error(
        "MISSING_RUNTIME_DEPENDENCIES",
        format!(
            "JavaScript PROCESS runtime artifact requires node_modules, but none was found in {} or {}. \
             Run the install step before deploy, or remove --skip-install.",
            project_dir.display(),
            runtime_root.display()
        ),
    ))
}

fn rewrite_manifest_for_project_runtime(
    mut manifest: build_manifest::Manifest,
    build_output_prefix: &str,
    project_owned_bootstrap: bool,
) -> anyhow::Result<build_manifest::Manifest> {
    for layer in &mut manifest.layers {
        match layer.target {
            build_manifest::LayerTarget::Compute => {
                // Only the generated adapter entry is anchored at the project
                // root; authored entries keep their output-relative custody.
                if project_owned_bootstrap
                    && layer.directory == "."
                    && layer.entry.as_deref()
                        == Some(crate::detect::python_launch::PYTHON_BOOTSTRAP_ENTRY)
                {
                    continue;
                }
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
    for file in crate::artifact::NODE_RUNTIME_METADATA_FILES
        .iter()
        .copied()
        .chain(["onreza.toml"])
    {
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
        for file in crate::artifact::NODE_RUNTIME_METADATA_FILES
            .iter()
            .copied()
            .chain(["onreza.toml"])
        {
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
