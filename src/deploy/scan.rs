use super::*;
use nrz_runtime_artifact::{ArtifactFileType, ArtifactRoot};

#[cfg(test)]
#[path = "scan_python_tests.rs"]
mod python_tests;

// ── Output scan ──────────────────────────────────────────────

/// Read buffer for streaming SHA-256. Sized to match a single page-cache
/// readahead window — small enough to stay in L2 cache, large enough that the
/// per-file read overhead doesn't dominate hashing throughput on big assets.
pub(super) const SCAN_HASH_CHUNK_BYTES: usize = 65_536;

/// Recursively scan `dir` and return a sorted list of `FileEntry { path, size, content_hash }`.
///
/// SHA-256 and size are computed **streaming**: the file is read in
/// `SCAN_HASH_CHUNK_BYTES` chunks and fed into the hasher, never buffered into
/// memory. Bytes are re-read from disk at upload time (page cache absorbs the
/// second read on any reasonable build host).
///
/// Safe relative symlinks are preserved as SOURCE_BUNDLE_V1 logical entries.
#[cfg(test)]
pub(crate) fn scan_dir(dir: &Path) -> anyhow::Result<Vec<FileEntry>> {
    scan_runtime_artifact_rooted(&ArtifactRoot::open(dir)?, &RuntimeArtifactScan::All)
}

#[cfg(test)]
pub(super) fn scan_runtime_artifact(
    root_dir: &Path,
    scan: &RuntimeArtifactScan,
) -> anyhow::Result<Vec<FileEntry>> {
    scan_runtime_artifact_rooted(&ArtifactRoot::open(root_dir)?, scan)
}

pub(super) fn scan_runtime_artifact_rooted(
    owner: &ArtifactRoot,
    scan: &RuntimeArtifactScan,
) -> anyhow::Result<Vec<FileEntry>> {
    match scan {
        RuntimeArtifactScan::All | RuntimeArtifactScan::NodeRuntimeRoot => {
            let mut files = Vec::new();
            scan_dir_recursive(
                Path::new(""),
                Path::new(""),
                Path::new(""),
                owner,
                &mut files,
                &mut Vec::new(),
            )?;
            files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
            Ok(files)
        }
        RuntimeArtifactScan::PythonRuntimeRoot(minor) => scan_python_root(owner, *minor, None),
        RuntimeArtifactScan::Relocated { base, ownership } => match base.as_ref() {
            RuntimeArtifactScan::PythonRuntimeRoot(minor) => scan_python_root(
                owner,
                *minor,
                Some(Path::new(&ownership.build_output_prefix)),
            ),
            _ => scan_runtime_artifact_rooted(owner, base),
        },
        RuntimeArtifactScan::Selected {
            roots,
            symlink_roots,
        } => scan_selected_runtime_roots_rooted(owner, roots, symlink_roots),
    }
}

fn scan_python_root(
    owner: &ArtifactRoot,
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
        owner: &ArtifactRoot,
        files: &mut Vec<FileEntry>,
        pruning: &Pruning<'_>,
        inherited_project_build_only: bool,
    ) -> anyhow::Result<()> {
        for name in owner.read_dir(current)? {
            let path = current.join(&name);
            let name = name.to_string_lossy();
            let ft = owner.file_type(&path, false)?;
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
                    || (ft.is_dir()
                        && owner
                            .file_type(&path.join("pyvenv.cfg"), true)
                            .is_ok_and(|kind| kind.is_file())));
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
                visit(base, &path, owner, files, pruning, project_build_only)?;
            } else {
                scan_runtime_path_with_type(base, base, &path, ft, owner, files, &mut Vec::new())?;
            }
        }
        Ok(())
    }
    let root = Path::new("");
    let mut files = Vec::new();
    let project_package = crate::detect::python::dependency_plan(&RootedPythonFs(owner))?
        .is_some_and(|plan| plan.install_project);
    let pruning = Pruning {
        project_package,
        minor,
        // A project-root output retains the established local-cache exclusions.
        // Only a distinct selected subtree overrides project filename heuristics.
        build_output: build_output.filter(|path| *path != Path::new(".")),
    };
    visit(root, Path::new(""), owner, &mut files, &pruning, false)?;
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn scan_selected_runtime_roots_rooted(
    owner: &ArtifactRoot,
    roots: &[crate::artifact::RuntimeArtifactScanRoot],
    symlink_roots: &[String],
) -> anyhow::Result<Vec<FileEntry>> {
    let root_dir = Path::new("");
    let mut files = Vec::new();

    let mut queued_roots = roots
        .iter()
        .map(|root| normalize_runtime_artifact_path(&root.path))
        .collect::<anyhow::Result<VecDeque<_>>>()?;
    let mut scheduled_roots = queued_roots.iter().cloned().collect::<HashSet<_>>();
    let mut scanned_roots = HashSet::new();
    let mut discovered_targets = Vec::new();

    while let Some(root) = queued_roots.pop_front() {
        if !scanned_roots.insert(root.clone()) {
            continue;
        }
        let path = root_dir.join(&root);
        match owner.file_type(&path, false) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
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
            owner,
            &mut files,
            &mut symlink_targets,
        )?;
        for target in symlink_targets {
            if symlink_roots.contains(&target) && scheduled_roots.insert(target.clone()) {
                queued_roots.push_back(target.clone());
            }
            discovered_targets.push(target);
        }
    }
    // A .bin link can precede the package alias that establishes its workspace root.
    for target in discovered_targets {
        if !runtime_scan_path_is_covered(&target, scheduled_roots.iter().map(String::as_str)) {
            return Err(output::coded_error(
                "INVALID_BUILD_OUTPUT",
                format!(
                    "runtime dependency symlink does not resolve to a declared workspace package root: {target}"
                ),
            ));
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

#[cfg(test)]
pub(super) fn ensure_no_unresolved_lfs_pointers(
    root_dir: &Path,
    files: &[FileEntry],
    git_lfs_enabled: bool,
) -> anyhow::Result<()> {
    ensure_no_unresolved_lfs_pointers_rooted(&ArtifactRoot::open(root_dir)?, files, git_lfs_enabled)
}

pub(super) fn ensure_no_unresolved_lfs_pointers_rooted(
    owner: &ArtifactRoot,
    files: &[FileEntry],
    git_lfs_enabled: bool,
) -> anyhow::Result<()> {
    for file in files {
        if file.size == 0 || file.size > GIT_LFS_POINTER_MAX_BYTES {
            continue;
        }
        let is_pointer = qualify_scanned_artifact_file(
            owner.open_file(Path::new(&file.path))?,
            &file.path,
            file.size,
            &file.content_hash,
            |reader| is_git_lfs_pointer_reader(reader),
        )?;
        if is_pointer {
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

#[cfg(test)]
pub(super) fn is_git_lfs_pointer_file(path: &Path) -> anyhow::Result<bool> {
    is_git_lfs_pointer_reader(std::fs::File::open(path)?)
}

fn is_git_lfs_pointer_reader(mut file: impl Read) -> anyhow::Result<bool> {
    let mut buf = Vec::new();
    file.by_ref()
        .take(GIT_LFS_POINTER_MAX_BYTES)
        .read_to_end(&mut buf)
        .context("failed to read opened artifact while checking Git LFS pointer")?;
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
    owner: &ArtifactRoot,
    files: &mut Vec<FileEntry>,
    symlink_targets: &mut Vec<String>,
) -> anyhow::Result<()> {
    let entries = owner
        .read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?;
    for name in entries {
        let path = current.join(name);
        let ft = owner
            .file_type(&path, false)
            .with_context(|| format!("failed to stat {}", path.display()))?;

        scan_runtime_path_with_type(
            base,
            installer_root,
            &path,
            ft,
            owner,
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
    owner: &ArtifactRoot,
    files: &mut Vec<FileEntry>,
    symlink_targets: &mut Vec<String>,
) -> anyhow::Result<()> {
    let ft = owner
        .file_type(path, false)
        .with_context(|| format!("failed to stat {}", path.display()))?;
    scan_runtime_path_with_type(
        base,
        installer_root,
        path,
        ft,
        owner,
        files,
        symlink_targets,
    )
}

pub(super) fn scan_runtime_path_with_type(
    base: &Path,
    installer_root: &Path,
    path: &Path,
    ft: ArtifactFileType,
    owner: &ArtifactRoot,
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
        let symlink = read_deploy_symlink_target(owner, path, &rel)?;
        files.push(FileEntry {
            path: rel,
            size: 0,
            content_hash: sha256_hex(symlink.link_target.as_bytes()),
            kind: crate::artifact::ArtifactFileKind::Symlink,
            symlink_resolved_path: Some(symlink.resolved_path.clone()),
            symlink_target: Some(symlink.link_target),
        });
        symlink_targets.push(symlink.resolved_path);
        return Ok(());
    }

    if ft.is_dir() {
        scan_dir_recursive(base, installer_root, path, owner, files, symlink_targets)?;
    } else if ft.is_file() {
        let rel = path
            .strip_prefix(base)
            .context("failed to compute relative path")?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let (size, content_hash) = hash_open_file_streaming(owner.open_file(path)?)
            .with_context(|| format!("failed to hash {}", rel_str))?;
        files.push(FileEntry {
            path: rel_str,
            size,
            content_hash,
            kind: crate::artifact::ArtifactFileKind::File,
            symlink_resolved_path: None,
            symlink_target: None,
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
    owner: &ArtifactRoot,
    path: &Path,
    rel: &str,
) -> anyhow::Result<DeploySymlinkTarget> {
    let target = owner
        .read_link(path)
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
    validate_deploy_symlink_target(rel, target)?;
    let relative = owner.canonicalize(path).map_err(|error| {
        let reason = if error.kind() == std::io::ErrorKind::PermissionDenied {
            "symlink escapes build output"
        } else {
            "broken symlink in build output"
        };
        output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("SOURCE_BUNDLE_V1 {reason}: {rel} -> {target} ({error})"),
        )
    })?;
    if relative.as_os_str().is_empty() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!("unsafe SOURCE_BUNDLE_V1 symlink target: {rel} -> {target}"),
        ));
    }
    Ok(DeploySymlinkTarget {
        link_target: target.to_string(),
        resolved_path: path_to_runtime_artifact_string(&relative)?,
    })
}

fn validate_deploy_symlink_target(rel: &str, target: &str) -> anyhow::Result<()> {
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

    Ok(())
}

/// Streaming size and SHA-256 from the same opened artifact file.
pub(crate) fn hash_open_file_streaming(file: std::fs::File) -> anyhow::Result<(u64, String)> {
    struct HashWriter(Sha256);

    impl std::io::Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut input = std::io::BufReader::with_capacity(SCAN_HASH_CHUNK_BYTES, file);
    let mut hasher = HashWriter(Sha256::new());
    let size =
        std::io::copy(&mut input, &mut hasher).context("failed to read opened artifact file")?;
    Ok((size, sha256_finalize_hex(hasher.0)))
}

pub(super) struct ArtifactQualificationReader {
    input: std::io::BufReader<std::io::Take<std::fs::File>>,
    hasher: Sha256,
    bytes: u64,
}

impl ArtifactQualificationReader {
    fn new(file: std::fs::File, limit: u64) -> Self {
        Self {
            input: std::io::BufReader::with_capacity(
                limit.min(SCAN_HASH_CHUNK_BYTES as u64) as usize,
                file.take(limit),
            ),
            hasher: Sha256::new(),
            bytes: 0,
        }
    }

    fn finish(self) -> (u64, String) {
        (self.bytes, sha256_finalize_hex(self.hasher))
    }
}

impl Read for ArtifactQualificationReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.input.read(buffer)?;
        self.hasher.update(&buffer[..read]);
        self.bytes += read as u64;
        Ok(read)
    }
}

/// Attest the exact bytes consumed by qualification, then stream the remaining bytes once.
pub(super) fn qualify_scanned_artifact_file<T>(
    file: std::fs::File,
    path: &str,
    expected_size: u64,
    expected_hash: &str,
    qualify: impl FnOnce(&mut ArtifactQualificationReader) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let mut reader = ArtifactQualificationReader::new(file, expected_size.saturating_add(1));
    let result = qualify(&mut reader)?;
    std::io::copy(
        &mut reader.by_ref().take(expected_size.saturating_add(1)),
        &mut std::io::sink(),
    )
    .with_context(|| format!("failed to read artifact qualification bytes: {path}"))?;
    let (size, hash) = reader.finish();
    anyhow::ensure!(
        size == expected_size && hash == expected_hash,
        "artifact changed after scanning: {path}"
    );
    Ok(result)
}

// Detection reads used by Python pruning share the same artifact owner as scanning.
struct RootedPythonFs<'a>(&'a ArtifactRoot);

impl crate::detect::fs::Fs for RootedPythonFs<'_> {
    fn exists(&self, path: &str) -> bool {
        self.0.file_type(Path::new(path), true).is_ok()
    }

    fn is_dir(&self, path: &str) -> bool {
        self.0
            .file_type(Path::new(path), true)
            .is_ok_and(|kind| kind.is_dir())
    }

    fn is_file(&self, path: &str) -> bool {
        self.0
            .file_type(Path::new(path), true)
            .is_ok_and(|kind| kind.is_file())
    }

    fn read_file(&self, path: &str) -> Option<String> {
        const MAX_BYTES: u64 = 512 * 1024;
        let relative = self.0.canonicalize(Path::new(path)).ok()?;
        let file = self.0.open_file(&relative).ok()?;
        let mut content = String::new();
        file.take(MAX_BYTES + 1).read_to_string(&mut content).ok()?;
        (content.len() as u64 <= MAX_BYTES).then_some(content)
    }

    fn list_dir(&self, path: &str) -> Vec<String> {
        let mut names = self
            .0
            .read_dir(Path::new(if path.is_empty() { "." } else { path }))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|name| name.to_str().map(str::to_owned))
            .collect::<Vec<_>>();
        names.sort();
        names
    }
}
