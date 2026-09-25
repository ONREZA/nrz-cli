use super::*;

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
    if detection.metadata.runtime.runtime_type == RuntimeType::Python
        && manifest_has_compute_layer(&manifest)
    {
        return resolve_python_runtime_artifact(project_dir, build_output_dir, manifest, json);
    }
    let Some(plan) = plan_node_project_runtime_artifact(
        workspace_root_dir,
        project_dir,
        &build_output_dir,
        &manifest,
        detection,
    ) else {
        let scan = if matches!(
            detection.metadata.runtime.runtime_type,
            RuntimeType::Node | RuntimeType::Bun
        ) && manifest_has_compute_layer(&manifest)
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
    let ownership = crate::artifact::RuntimeArtifactSourceOwnership {
        build_output_prefix: plan.build_output_prefix.clone(),
        layers: manifest.layers.clone(),
    };
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
            "Runtime artifact: JavaScript {runtime_root_label} (entry and dependencies share one runtime root)"
        ),
        output::Phase::Deploy,
    );

    Ok(RuntimeArtifact {
        root_dir: plan.runtime_root,
        manifest,
        scan: RuntimeArtifactScan::Relocated {
            base: Box::new(RuntimeArtifactScan::Selected {
                roots,
                symlink_roots,
            }),
            ownership,
        },
    })
}

fn resolve_python_runtime_artifact(
    project_dir: &Path,
    build_output_dir: PathBuf,
    manifest: build_manifest::Manifest,
    json: bool,
) -> anyhow::Result<RuntimeArtifact> {
    let build_output_prefix = relative_runtime_artifact_path(project_dir, &build_output_dir)
        .map_err(|_| {
            output::coded_error(
                "INVALID_BUILD_OUTPUT",
                "Python output directory must be inside the project directory",
            )
        })?;
    let ownership =
        (build_output_prefix != ".").then(|| crate::artifact::RuntimeArtifactSourceOwnership {
            build_output_prefix: build_output_prefix.clone(),
            layers: manifest.layers.clone(),
        });
    let manifest = if build_output_prefix == "." {
        manifest
    } else {
        rewrite_manifest_for_node_project_runtime(manifest, &build_output_prefix)?
    };
    let dependency_root = project_dir.join(crate::artifact::PYTHON_SITE_PACKAGES_ROOT);
    if crate::detect::python::dependency_manifest(&crate::detect::fs::LocalFs::new(project_dir))
        .is_some()
        && !dependency_root.is_dir()
    {
        return Err(output::coded_error(
            "MISSING_RUNTIME_DEPENDENCIES",
            "Python PROCESS runtime requires installed dependencies. Run the install step before deploy, or remove --skip-install.",
        ));
    }
    output::status(
        json,
        "~",
        "Runtime artifact: CPython 3.14 project root",
        output::Phase::Deploy,
    );
    Ok(RuntimeArtifact {
        root_dir: project_dir.to_path_buf(),
        manifest,
        scan: match ownership {
            Some(ownership) => RuntimeArtifactScan::Relocated {
                base: Box::new(RuntimeArtifactScan::PythonRuntimeRoot),
                ownership,
            },
            None => RuntimeArtifactScan::PythonRuntimeRoot,
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
    if !matches!(
        detection.metadata.runtime.runtime_type,
        RuntimeType::Node | RuntimeType::Bun
    ) {
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
            "JavaScript PROCESS runtime artifact requires node_modules, but none was found in {} or {}. \
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
