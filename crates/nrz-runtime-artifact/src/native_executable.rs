//! Native entry admission is independent of managed interpreter authorization.
use goblin::elf::{Elf, dynamic, header, program_header};
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Component, Path, PathBuf},
};

use crate::{RuntimeArtifactError, invariant};

pub const NATIVE_EXECUTION_TARGET: &str = "native-linux-x86_64-glibc";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExecutableRequirements {
    pub interpreter: Option<String>,
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
}

/// Validate the supported executable format, returning its library requirements.
/// Presence in a Builder image does not prove that a runtime can satisfy them.
pub fn verify_native_executable(
    bytes: &[u8],
) -> Result<NativeExecutableRequirements, RuntimeArtifactError> {
    let elf = Elf::parse(bytes).map_err(|error| {
        RuntimeArtifactError::Invariant(format!("invalid native ELF entry: {error}"))
    })?;
    inspect_elf(&elf, bytes, true)
}

fn inspect_elf(
    elf: &Elf<'_>,
    bytes: &[u8],
    executable: bool,
) -> Result<NativeExecutableRequirements, RuntimeArtifactError> {
    if elf.header.e_ident[header::EI_CLASS] != header::ELFCLASS64
        || elf.header.e_ident[header::EI_DATA] != header::ELFDATA2LSB
        || elf.header.e_machine != header::EM_X86_64
        || !matches!(elf.header.e_type, header::ET_EXEC | header::ET_DYN)
        || (executable && elf.entry == 0)
        || (!executable && elf.header.e_type != header::ET_DYN)
        || !matches!(
            elf.header.e_ident[header::EI_OSABI],
            header::ELFOSABI_NONE | header::ELFOSABI_LINUX
        )
    {
        return invariant("native entry requires a runnable Linux x86_64 ELF executable");
    }
    if !elf.program_headers.iter().any(|segment| {
        segment.p_type == program_header::PT_LOAD
            && segment.p_filesz > 0
            && segment
                .p_offset
                .checked_add(segment.p_filesz)
                .is_some_and(|end| end <= bytes.len() as u64)
            && (!executable
                || (segment.p_flags & program_header::PF_X != 0
                    && elf.entry >= segment.p_vaddr
                    && segment
                        .p_vaddr
                        .checked_add(segment.p_filesz)
                        .is_some_and(|end| elf.entry < end)))
    }) {
        return invariant(if executable {
            "native entry is outside an executable file-backed ELF segment"
        } else {
            "native shared library requires a file-backed loadable ELF segment"
        });
    }
    if elf
        .interpreter
        .is_some_and(|path| path != "/lib64/ld-linux-x86-64.so.2")
    {
        return invariant("native entry requires the qualified Linux glibc interpreter");
    }
    if elf.libraries.len() > 128
        || elf.libraries.iter().any(|name| {
            name.is_empty()
                || name.len() > 256
                || name.contains(['/', '\\', '\0', '$'])
                || *name == "."
                || *name == ".."
        })
    {
        return invariant("native entry has invalid dynamic library requirements");
    }
    let library_paths: Vec<String> = elf
        .rpaths
        .iter()
        .chain(&elf.runpaths)
        .flat_map(|path| path.split(':'))
        .map(str::to_owned)
        .collect();
    if library_paths.len() > 128
        || library_paths
            .iter()
            .any(|path| path.is_empty() || path.len() > 4096 || path.contains(['\0', '\\']))
    {
        return invariant("native entry has invalid dynamic library paths");
    }
    Ok(NativeExecutableRequirements {
        interpreter: elf.interpreter.map(str::to_owned),
        libraries: elf
            .libraries
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        library_paths,
    })
}

impl NativeExecutableRequirements {
    /// Qualified minimal Ubuntu 26.04 Compute SONAMEs, not the Builder/host cache.
    pub const QUALIFIED_SYSTEM_LIBRARIES: &'static [&'static str] = &[
        "libc.so.6",
        "libm.so.6",
        "libpthread.so.0",
        "libdl.so.2",
        "librt.so.1",
        "libgcc_s.so.1",
        "libstdc++.so.6",
        "ld-linux-x86-64.so.2",
    ];

    /// Verify declared dynamic dependencies without executing the ELF or loading
    /// host libraries. Returned relative paths must also survive archive scanning.
    pub fn verify_artifact_closure(
        artifact_root: &Path,
        executable_path: &Path,
        launch_cwd: &Path,
    ) -> Result<(Self, Vec<PathBuf>), RuntimeArtifactError> {
        let root = std::fs::canonicalize(artifact_root).map_err(fs_error)?;
        let cwd = std::fs::canonicalize(launch_cwd).map_err(fs_error)?;
        if !cwd.starts_with(&root) || !cwd.is_dir() {
            return invariant("native launch cwd escapes the artifact");
        }
        let mut members = BTreeSet::new();
        let entry_path = root.join(executable_path.strip_prefix(artifact_root).map_err(|_| {
            RuntimeArtifactError::Invariant("native entry is outside the artifact".into())
        })?);
        resolve_artifact_path(&root, &entry_path, &mut members)?;
        let mut visited = BTreeSet::new();
        let requirements = inspect_closure(
            &root,
            &cwd,
            &entry_path,
            true,
            &[],
            &mut visited,
            &mut members,
        )?;
        Ok((requirements, members.into_iter().collect()))
    }
}

fn fs_error(error: std::io::Error) -> RuntimeArtifactError {
    RuntimeArtifactError::Invariant(format!("native artifact closure is incomplete: {error}"))
}

fn qualified_system_path(path: &str) -> bool {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    matches!(
        parts.as_slice(),
        ["lib" | "lib64", ..] | ["usr", "lib" | "lib64", ..]
    )
}

// Only leading parents are supported in loader suffixes/link targets. Intermediate
// directories traversed and then discarded would not survive a file-only archive.
fn has_intermediate_parent(path: &Path) -> bool {
    let mut normal = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => normal = true,
            Component::ParentDir if normal => return true,
            _ => {}
        }
    }
    false
}

fn resolve_artifact_path(
    root: &Path,
    path: &Path,
    members: &mut BTreeSet<PathBuf>,
) -> Result<PathBuf, RuntimeArtifactError> {
    // Check the original path: normalizing before resolving symlinks changes OS
    // semantics and can conceal an escape through a link followed by `..`.
    let canonical = std::fs::canonicalize(path).map_err(fs_error)?;
    if !canonical.starts_with(root) {
        return invariant("native library path escapes the artifact");
    }
    let relative = path.strip_prefix(root).map_err(|_| {
        RuntimeArtifactError::Invariant("native library path escapes the artifact".into())
    })?;
    let mut pending = relative
        .components()
        .map(|part| part.as_os_str().to_owned())
        .collect::<VecDeque<_>>();
    let mut prefix = root.to_path_buf();
    let mut links = 0;
    while let Some(part) = pending.pop_front() {
        if part == "." {
            continue;
        }
        if part == ".." {
            if prefix == root {
                return invariant("native library path escapes the artifact");
            }
            prefix.pop();
            continue;
        }
        prefix.push(part);
        let metadata = std::fs::symlink_metadata(&prefix).map_err(fs_error)?;
        if metadata.file_type().is_symlink() {
            links += 1;
            if links > 128 {
                return invariant("native artifact library symlink chain exceeds its bound");
            }
            members.insert(prefix.strip_prefix(root).unwrap().to_path_buf());
            let target = std::fs::read_link(&prefix).map_err(fs_error)?;
            if target.is_absolute() || has_intermediate_parent(&target) {
                return invariant(
                    "native artifact library symlink requires a relative target without intermediate parent traversal",
                );
            }
            prefix.pop();
            for part in target.components().rev() {
                pending.push_front(part.as_os_str().to_owned());
            }
        }
    }
    if prefix != canonical {
        return invariant("native artifact library path changed while resolving");
    }
    Ok(prefix)
}

fn loader_paths(
    root: &Path,
    cwd: &Path,
    object: &Path,
    paths: &[&str],
    members: &mut BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, RuntimeArtifactError> {
    let mut result = Vec::new();
    for path in paths.iter().flat_map(|path| path.split(':')) {
        let suffix = path
            .strip_prefix("$ORIGIN")
            .or_else(|| path.strip_prefix("${ORIGIN}"));
        let candidate = if let Some(suffix) = suffix {
            if (!suffix.is_empty() && !suffix.starts_with('/')) || suffix.contains('$') {
                return invariant("native library path contains an unsupported loader token");
            }
            let suffix = Path::new(suffix.trim_start_matches('/'));
            if has_intermediate_parent(suffix) {
                return invariant(
                    "native library path has unsupported intermediate parent traversal",
                );
            }
            object.parent().unwrap().join(suffix)
        } else {
            if path.contains('$') {
                return invariant("native library path contains an unsupported loader token");
            }
            // ELF paths are Unix paths even when a prebuilt bundle is published
            // on a different host OS.
            if path.starts_with('/') {
                if qualified_system_path(path) {
                    // Only the qualified SONAME baseline can satisfy system paths.
                    continue;
                }
                return invariant(
                    "native library path points outside the artifact and qualified system directories",
                );
            }
            let path = Path::new(path);
            if has_intermediate_parent(path) {
                return invariant(
                    "native library path has unsupported intermediate parent traversal",
                );
            }
            cwd.join(path)
        };
        resolve_artifact_path(root, &candidate, members)?;
        if !candidate.is_dir() {
            return invariant("native artifact library path is not a directory");
        }
        result.push(candidate);
    }
    Ok(result)
}

fn inspect_closure(
    root: &Path,
    cwd: &Path,
    object: &Path,
    executable: bool,
    inherited_rpaths: &[PathBuf],
    visited: &mut BTreeSet<PathBuf>,
    members: &mut BTreeSet<PathBuf>,
) -> Result<NativeExecutableRequirements, RuntimeArtifactError> {
    if visited.len() >= 128 {
        return invariant("native library closure exceeds its object bound");
    }
    let canonical = resolve_artifact_path(root, object, members)?;
    let metadata = std::fs::metadata(&canonical).map_err(fs_error)?;
    if !metadata.is_file() {
        return invariant("native library must be a regular file");
    }
    let bytes = std::fs::read(&canonical).map_err(fs_error)?;
    let elf = Elf::parse(&bytes).map_err(|error| {
        RuntimeArtifactError::Invariant(format!("invalid native artifact ELF: {error}"))
    })?;
    let requirements = inspect_elf(&elf, &bytes, executable)?;
    if elf.dynamic.as_ref().is_some_and(|value| {
        value.info.flags_1 & dynamic::DF_1_NODEFLIB != 0
            || value.dyns.iter().any(|value| {
                matches!(
                    value.d_tag,
                    dynamic::DT_AUDIT | dynamic::DT_DEPAUDIT | 0x7fff_ffff | 0x7fff_fffd
                )
            })
    }) {
        return invariant("native ELF uses unsupported audit/filter/nodefaultlib loading");
    }
    members.insert(canonical.strip_prefix(root).unwrap().to_path_buf());
    visited.insert(canonical);
    let rpaths = loader_paths(root, cwd, object, &elf.rpaths, members)?;
    let runpaths = loader_paths(root, cwd, object, &elf.runpaths, members)?;
    let mut inherited = if elf.runpaths.is_empty() {
        rpaths
    } else {
        Vec::new()
    };
    inherited.extend_from_slice(inherited_rpaths);
    let search = if elf.runpaths.is_empty() {
        inherited.clone()
    } else {
        runpaths
    };
    for library in &requirements.libraries {
        let mut resolved = None;
        for directory in &search {
            let candidate = directory.join(library);
            match std::fs::symlink_metadata(&candidate) {
                Ok(_) => {
                    let canonical = resolve_artifact_path(root, &candidate, members)?;
                    resolved = Some((candidate, canonical));
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(fs_error(error)),
            }
        }
        if let Some((loader_path, canonical)) = resolved {
            if !visited.contains(&canonical) {
                inspect_closure(root, cwd, &loader_path, false, &inherited, visited, members)?;
            }
        } else if !NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARIES
            .contains(&library.as_str())
        {
            return invariant(format!(
                "native artifact lacks required library '{library}' requested by {}",
                object.strip_prefix(root).unwrap().display()
            ));
        }
    }
    Ok(requirements)
}

#[cfg(test)]
#[path = "native_executable_tests.rs"]
mod tests;
