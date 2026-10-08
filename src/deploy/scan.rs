use super::*;

#[cfg(test)]
#[path = "scan_python_tests.rs"]
mod python_tests;

// ── Output scan ──────────────────────────────────────────────

/// Read buffer for streaming SHA-256. Sized to match a single page-cache
/// readahead window — small enough to stay in L2 cache, large enough that the
/// per-file read overhead doesn't dominate hashing throughput on big assets.
pub(super) const SCAN_HASH_CHUNK_BYTES: usize = 64 * 1024;

/// Recursively scan `dir` and return a sorted list of `FileEntry { path, size, content_hash }`.
///
/// SHA-256 and size are computed **streaming**: the file is read in
/// `SCAN_HASH_CHUNK_BYTES` chunks and fed into the hasher, never buffered into
/// memory. Bytes are re-read from disk at upload time (page cache absorbs the
/// second read on any reasonable build host).
///
/// Safe relative symlinks are preserved as SOURCE_BUNDLE_V1 logical entries.
pub(crate) fn scan_dir(dir: &Path) -> anyhow::Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    let canonical_base = std::fs::canonicalize(dir)
        .with_context(|| format!("failed to canonicalize {}", dir.display()))?;
    let mut symlink_targets = Vec::new();
    scan_dir_recursive(
        dir,
        dir,
        dir,
        &canonical_base,
        &mut files,
        &mut symlink_targets,
    )?;
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

pub(super) fn scan_runtime_artifact(
    root_dir: &Path,
    scan: &RuntimeArtifactScan,
) -> anyhow::Result<Vec<FileEntry>> {
    match scan {
        RuntimeArtifactScan::All | RuntimeArtifactScan::NodeRuntimeRoot => scan_dir(root_dir),
        RuntimeArtifactScan::PythonRuntimeRoot(minor) => scan_python_root(root_dir, *minor, None),
        RuntimeArtifactScan::Relocated { base, ownership } => match base.as_ref() {
            RuntimeArtifactScan::PythonRuntimeRoot(minor) => scan_python_root(
                root_dir,
                *minor,
                Some(Path::new(&ownership.build_output_prefix)),
            ),
            _ => scan_runtime_artifact(root_dir, base),
        },
        RuntimeArtifactScan::Selected {
            roots,
            symlink_roots,
        } => scan_selected_runtime_roots(root_dir, roots, symlink_roots),
    }
}

fn scan_python_root(
    root: &Path,
    minor: nrz_source_bundle::PythonMinor,
    build_output: Option<&Path>,
) -> anyhow::Result<Vec<FileEntry>> {
    struct Pruning<'a> {
        project_package: bool,
        minor: nrz_source_bundle::PythonMinor,
        build_output: Option<&'a Path>,
    }

    fn visit(
        base: &Path,
        current: &Path,
        canonical: &Path,
        files: &mut Vec<FileEntry>,
        pruning: &Pruning<'_>,
        inherited_project_build_only: bool,
    ) -> anyhow::Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let ft = entry.file_type()?;
            let relative = path.strip_prefix(base)?;
            let staged_dependency = relative.starts_with(pruning.minor.site_packages_root());
            if is_python_installer_staging_path(base, &path) {
                continue;
            }
            // A previous build may leave another minor's incompatible wheel tree.
            if relative.parent() == Some(Path::new(".onreza/python"))
                && nrz_source_bundle::PythonMinor::from_version(&name)
                    .is_some_and(|other| other != pruning.minor)
            {
                continue;
            }
            let backend_output = pruning.project_package
                && (relative == Path::new("build/lib")
                    || relative == Path::new("build/bdist")
                    || relative.parent() == Some(Path::new("build"))
                        && (name.starts_with("bdist.") || ft.is_dir() && name.starts_with("lib.")));
            let project_build_only = !staged_dependency
                && (inherited_project_build_only
                    || name.ends_with(".egg-info")
                    || backend_output
                    || matches!(
                        name.as_ref(),
                        ".venv"
                            | "venv"
                            | "__pycache__"
                            | ".pytest_cache"
                            | ".mypy_cache"
                            | ".ruff_cache"
                            | ".tox"
                            | ".nox"
                            | "node_modules"
                    )
                    || (ft.is_dir() && path.join("pyvenv.cfg").is_file()));
            // The planner's selected output is authoritative, including when a
            // packaging backend uses the same directory name. Traverse its
            // ancestors, but keep their unrelated cache children excluded.
            let selected_output = pruning.build_output.is_some_and(|output| {
                relative.starts_with(output) || ft.is_dir() && output.starts_with(relative)
            });
            let project_dotenv =
                !staged_dependency && (name == ".env" || name.starts_with(".env."));
            if project_build_only && !selected_output
                || project_dotenv
                || is_vcs_internal_path(base, &path)
            {
                continue;
            }
            if ft.is_dir() {
                visit(base, &path, canonical, files, pruning, project_build_only)?;
            } else {
                scan_runtime_path_with_type(
                    base,
                    base,
                    &path,
                    ft,
                    canonical,
                    files,
                    &mut Vec::new(),
                )?;
            }
        }
        Ok(())
    }
    let canonical = std::fs::canonicalize(root)?;
    let mut files = Vec::new();
    let project_package =
        crate::detect::python::dependency_plan(&crate::detect::fs::LocalFs::new(root))?
            .is_some_and(|plan| plan.install_project);
    let pruning = Pruning {
        project_package,
        minor,
        // A project-root output retains the established local-cache exclusions.
        // Only a distinct selected subtree overrides project filename heuristics.
        build_output: build_output.filter(|path| *path != Path::new(".")),
    };
    visit(root, root, &canonical, &mut files, &pruning, false)?;
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

pub(super) fn scan_selected_runtime_roots(
    root_dir: &Path,
    roots: &[crate::artifact::RuntimeArtifactScanRoot],
    symlink_roots: &[String],
) -> anyhow::Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    let canonical_base = std::fs::canonicalize(root_dir)
        .with_context(|| format!("failed to canonicalize {}", root_dir.display()))?;

    let mut queued_roots = roots
        .iter()
        .map(|root| normalize_runtime_artifact_path(&root.path))
        .collect::<anyhow::Result<VecDeque<_>>>()?;
    let mut scheduled_roots = queued_roots.iter().cloned().collect::<HashSet<_>>();
    let mut scanned_roots = HashSet::new();

    while let Some(root) = queued_roots.pop_front() {
        if !scanned_roots.insert(root.clone()) {
            continue;
        }
        let path = root_dir.join(&root);
        if !path.exists() {
            continue;
        }
        // Selected output can be project-relative beneath a workspace archive
        // root. Installer state belongs to that output root, while file paths
        // and symlink containment remain relative to the full archive root.
        let installer_root = if roots.iter().any(|candidate| {
            candidate.kind == crate::artifact::RuntimeArtifactScanRootKind::BuildOutput
                && normalize_runtime_artifact_path(&candidate.path)
                    .is_ok_and(|candidate| candidate == root)
        }) {
            path.as_path()
        } else {
            root_dir
        };
        let mut symlink_targets = Vec::new();
        scan_runtime_path(
            root_dir,
            installer_root,
            &path,
            &canonical_base,
            &mut files,
            &mut symlink_targets,
        )?;
        for target in symlink_targets {
            if runtime_scan_path_is_covered(&target, scheduled_roots.iter().map(String::as_str)) {
                continue;
            }
            if !symlink_roots.iter().any(|root| root == &target) {
                return Err(output::coded_error(
                    "INVALID_BUILD_OUTPUT",
                    format!(
                        "runtime dependency symlink does not resolve to a declared workspace package root: {target}"
                    ),
                ));
            }
            scheduled_roots.insert(target.clone());
            queued_roots.push_back(target);
        }
    }
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    files.dedup_by(|a, b| a.path == b.path);
    Ok(files)
}

pub(super) fn runtime_scan_path_is_covered<'a>(
    path: &str,
    mut roots: impl Iterator<Item = &'a str>,
) -> bool {
    roots.any(|root| root == "." || path == root || path.starts_with(&format!("{root}/")))
}

#[cfg(test)]
pub(super) fn prepare_deploy_files(
    manifest: &build_manifest::Manifest,
    files: Vec<FileEntry>,
    detection: &crate::detect::types::DetectionResult,
    json: bool,
) -> anyhow::Result<Vec<FileEntry>> {
    Ok(prepare_artifact_files(
        manifest,
        files,
        detection,
        ArtifactRootScope::ProjectRoot,
        &RuntimeArtifactScan::All,
        json,
    )
    .deployable_entries())
}

pub(super) fn prepare_artifact_files(
    manifest: &build_manifest::Manifest,
    files: Vec<FileEntry>,
    detection: &crate::detect::types::DetectionResult,
    root_scope: ArtifactRootScope,
    scan: &RuntimeArtifactScan,
    json: bool,
) -> crate::artifact::ArtifactFileCollection {
    let collection =
        crate::artifact::classify_artifact_files(manifest, files, detection, root_scope, scan);

    if collection.summary.pruned_files > 0 {
        output::status(
            json,
            "~",
            format!(
                "Pruned {pruned_count}/{original_count} build-only artifact(s) from SOURCE_BUNDLE_V1 ({pruned_bytes})",
                pruned_count = collection.summary.pruned_files,
                original_count = collection.summary.scanned_files,
                pruned_bytes = format_u64_bytes(collection.summary.pruned_bytes),
            ),
            output::Phase::Deploy,
        );
        tracing::info!(
            pruned_count = collection.summary.pruned_files,
            original_count = collection.summary.scanned_files,
            pruned_bytes = collection.summary.pruned_bytes,
            original_bytes = collection
                .summary
                .deployable_bytes
                .saturating_add(collection.summary.pruned_bytes),
            "pruned framework build-only artifacts before SOURCE_BUNDLE_V1 packaging"
        );
    }

    let deployable = collection.deployable_entries();
    warn_large_deploy_files(json, &deployable);
    collection
}

pub(super) fn ensure_no_unresolved_lfs_pointers(
    root_dir: &Path,
    files: &[FileEntry],
    git_lfs_enabled: bool,
) -> anyhow::Result<()> {
    for file in files {
        if file.size == 0 || file.size > GIT_LFS_POINTER_MAX_BYTES {
            continue;
        }
        let path = root_dir.join(&file.path);
        if is_git_lfs_pointer_file(&path)? {
            let (code, message) = if git_lfs_enabled {
                (
                    "GIT_LFS_UNRESOLVED",
                    format!(
                        "file \"{}\" is still an unresolved Git LFS pointer even though Git LFS is enabled for this project. \
                         Run `git lfs pull` before local deploy, or check the builder Git LFS fetch step for server-side deploys.",
                        file.path
                    ),
                )
            } else {
                (
                    "GIT_LFS_REQUIRED",
                    format!(
                        "file \"{}\" is an unresolved Git LFS pointer, but Git LFS is disabled for this project. \
                     Enable Git LFS in project settings or commit the real file bytes before deploying.",
                        file.path
                    ),
                )
            };
            return Err(output::coded_error(code, message));
        }
    }

    Ok(())
}

pub(super) const GIT_LFS_POINTER_MAX_BYTES: u64 = 1024;

pub(super) fn is_git_lfs_pointer_file(path: &Path) -> anyhow::Result<bool> {
    let mut file = std::fs::File::open(path).with_context(|| {
        format!(
            "failed to open {} while checking Git LFS pointer",
            path.display()
        )
    })?;
    let mut buf = Vec::new();
    file.by_ref()
        .take(GIT_LFS_POINTER_MAX_BYTES)
        .read_to_end(&mut buf)
        .with_context(|| {
            format!(
                "failed to read {} while checking Git LFS pointer",
                path.display()
            )
        })?;
    let content = String::from_utf8_lossy(&buf);

    Ok(
        content.starts_with("version https://git-lfs.github.com/spec/v1\n")
            && content.contains("\noid sha256:")
            && content.contains("\nsize "),
    )
}

pub(super) fn warn_large_deploy_files(json: bool, files: &[FileEntry]) {
    const LARGE_DEPLOY_FILE_WARNING_BYTES: u64 = 25 * 1024 * 1024;
    let mut large = files
        .iter()
        .filter(|file| file.size > LARGE_DEPLOY_FILE_WARNING_BYTES)
        .collect::<Vec<_>>();
    large.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    if large.is_empty() {
        return;
    }

    let display = large
        .iter()
        .take(5)
        .map(|file| format!("{} ({})", file.path, format_u64_bytes(file.size)))
        .collect::<Vec<_>>()
        .join(", ");
    output::warn(
        json,
        format!(
            "Large deployment files detected before upload: {display}. \
             Server-side plan limits will be checked during upload preparation."
        ),
        output::Phase::Deploy,
    );
}

pub(super) fn scan_dir_recursive(
    base: &Path,
    installer_root: &Path,
    current: &Path,
    canonical_base: &Path,
    files: &mut Vec<FileEntry>,
    symlink_targets: &mut Vec<String>,
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

        scan_runtime_path_with_type(
            base,
            installer_root,
            &path,
            ft,
            canonical_base,
            files,
            symlink_targets,
        )?;
    }

    Ok(())
}

pub(super) fn scan_runtime_path(
    base: &Path,
    installer_root: &Path,
    path: &Path,
    canonical_base: &Path,
    files: &mut Vec<FileEntry>,
    symlink_targets: &mut Vec<String>,
) -> anyhow::Result<()> {
    let ft = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to stat {}", path.display()))?
        .file_type();
    scan_runtime_path_with_type(
        base,
        installer_root,
        path,
        ft,
        canonical_base,
        files,
        symlink_targets,
    )
}

pub(super) fn scan_runtime_path_with_type(
    base: &Path,
    installer_root: &Path,
    path: &Path,
    ft: std::fs::FileType,
    canonical_base: &Path,
    files: &mut Vec<FileEntry>,
    symlink_targets: &mut Vec<String>,
) -> anyhow::Result<()> {
    if is_vcs_internal_path(base, path) || is_python_installer_staging_path(installer_root, path) {
        return Ok(());
    }

    if ft.is_symlink() {
        let rel = path
            .strip_prefix(base)
            .context("failed to compute relative path")?
            .to_string_lossy()
            .replace('\\', "/");
        let symlink = read_deploy_symlink_target(path, &rel, canonical_base)?;
        files.push(FileEntry {
            path: rel,
            size: 0,
            content_hash: sha256_hex(symlink.link_target.as_bytes()),
            kind: crate::artifact::ArtifactFileKind::Symlink,
            symlink_resolved_path: Some(symlink.resolved_path.clone()),
        });
        symlink_targets.push(symlink.resolved_path);
        return Ok(());
    }

    if ft.is_dir() {
        scan_dir_recursive(
            base,
            installer_root,
            path,
            canonical_base,
            files,
            symlink_targets,
        )?;
    } else if ft.is_file() {
        let rel = path
            .strip_prefix(base)
            .context("failed to compute relative path")?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let (size, content_hash) =
            hash_file_streaming(path).with_context(|| format!("failed to hash {}", rel_str))?;
        files.push(FileEntry {
            path: rel_str,
            size,
            content_hash,
            kind: crate::artifact::ArtifactFileKind::File,
            symlink_resolved_path: None,
        });
    }

    Ok(())
}

fn is_python_installer_staging_path(base: &Path, path: &Path) -> bool {
    path.strip_prefix(base)
        .is_ok_and(|relative| relative.starts_with(".onreza/python/build"))
}

pub(super) fn is_vcs_internal_path(base: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(base) else {
        return false;
    };
    rel.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if matches!(name.to_str(), Some(".git" | ".hg" | ".svn"))
        )
    })
}

pub(super) fn read_deploy_symlink_target(
    path: &Path,
    rel: &str,
    canonical_base: &Path,
) -> anyhow::Result<DeploySymlinkTarget> {
    let target = std::fs::read_link(path)
        .with_context(|| format!("failed to read SOURCE_BUNDLE_V1 symlink {}", path.display()))?;
    let target = target.to_str().ok_or_else(|| {
        output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "SOURCE_BUNDLE_V1 symlink target is not UTF-8: {}",
                path.display()
            ),
        )
    })?;
    let resolved_path = resolve_deploy_symlink_target(rel, target)?;
    match std::fs::canonicalize(path) {
        Ok(canonical) if canonical.starts_with(canonical_base) => Ok(DeploySymlinkTarget {
            link_target: target.to_string(),
            resolved_path,
        }),
        Ok(canonical) => Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "SOURCE_BUNDLE_V1 symlink escapes build output: {rel} -> {target} resolved to {}",
                canonical.display()
            ),
        )),
        Err(error) => Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("SOURCE_BUNDLE_V1 broken symlink in build output: {rel} -> {target} ({error})"),
        )),
    }
}

pub(super) fn resolve_deploy_symlink_target(rel: &str, target: &str) -> anyhow::Result<String> {
    if target.is_empty() || target.contains('\\') || target.contains('\0') {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("unsafe SOURCE_BUNDLE_V1 symlink target: {rel} -> {target}"),
        ));
    }
    if source_bundle_contract_characters(target) > SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "SOURCE_BUNDLE_V1 symlink target too long: {rel} -> {target} (max {SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS} characters)"
            ),
        ));
    }
    let target_path = Path::new(target);
    if target_path.is_absolute() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("SOURCE_BUNDLE_V1 symlink has absolute target: {rel} -> {target}"),
        ));
    }

    let mut resolved = PathBuf::new();
    if let Some(parent) = Path::new(rel).parent()
        && !parent.as_os_str().is_empty()
    {
        resolved.push(parent);
    }
    for component in target_path.components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(output::coded_error(
                        "INVALID_BUILD_OUTPUT",
                        format!("SOURCE_BUNDLE_V1 symlink escapes build output: {rel} -> {target}"),
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(output::coded_error(
                    "INVALID_BUILD_OUTPUT",
                    format!("unsafe SOURCE_BUNDLE_V1 symlink target: {rel} -> {target}"),
                ));
            }
        }
    }
    if resolved.as_os_str().is_empty() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("unsafe SOURCE_BUNDLE_V1 symlink target: {rel} -> {target}"),
        ));
    }
    path_to_runtime_artifact_string(&resolved)
}

/// Streaming SHA-256 + size for a single file. Returns `(size, lowercase_hex_sha256)`.
pub(crate) fn hash_file_streaming(path: &Path) -> anyhow::Result<(u64, String)> {
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
    Ok((size, sha256_finalize_hex(hasher)))
}
