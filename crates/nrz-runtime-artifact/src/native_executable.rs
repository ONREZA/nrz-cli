//! Native entry admission is independent of managed interpreter authorization.
use goblin::{
    container::{Container, Ctx, Endian},
    elf::{Elf, dynamic, header, program_header, section_header, symver},
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};

use crate::{ArtifactRoot, RuntimeArtifactError, invariant};

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
    if executable {
        verify_entry_interpreter(elf, bytes)?;
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

fn verify_entry_interpreter(elf: &Elf<'_>, bytes: &[u8]) -> Result<(), RuntimeArtifactError> {
    let mut segments = elf
        .program_headers
        .iter()
        .filter(|segment| segment.p_type == program_header::PT_INTERP);
    let Some(segment) = segments.next() else {
        // Static ELF and self-relocating static PIE have no dynamic dependencies.
        return if elf.libraries.is_empty() {
            Ok(())
        } else {
            invariant("dynamic native entry requires the qualified Linux glibc interpreter")
        };
    };
    // Linux uses the first PT_INTERP and requires a bounded, NUL-terminated
    // file-backed path. Goblin keeps only an optional decoded interpreter and
    // can conceal duplicate headers or a failed/truncated path read.
    let payload = segment
        .p_offset
        .checked_add(segment.p_filesz)
        .and_then(|end| Some(usize::try_from(segment.p_offset).ok()?..usize::try_from(end).ok()?))
        .and_then(|range| bytes.get(range));
    if segments.next().is_some()
        || !(2..=4096).contains(&segment.p_filesz)
        || payload.and_then(|bytes| bytes.last()) != Some(&0)
        || elf.interpreter != Some("/lib64/ld-linux-x86-64.so.2")
    {
        return invariant(
            "native entry requires one valid qualified Linux glibc interpreter segment",
        );
    }
    Ok(())
}

impl NativeExecutableRequirements {
    /// Exported version definitions of the qualified Compute system libraries.
    pub const QUALIFIED_SYSTEM_LIBRARY_VERSIONS_JSON: &'static str =
        include_str!("../assets/native-system-library-versions.json");

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
        // Compatibility acquisition accepts the same ambient root/cwd aliases as before.
        let root_path = std::fs::canonicalize(artifact_root).map_err(fs_error)?;
        let cwd_path = std::fs::canonicalize(launch_cwd).map_err(fs_error)?;
        if !cwd_path.starts_with(&root_path) || !cwd_path.is_dir() {
            return invariant("native launch cwd escapes the artifact");
        }
        let entry = executable_path.strip_prefix(artifact_root).map_err(|_| {
            RuntimeArtifactError::Invariant("native entry is outside the artifact".into())
        })?;
        let cwd = cwd_path.strip_prefix(&root_path).unwrap();
        let root = ArtifactRoot::open(artifact_root).map_err(fs_error)?;
        Self::verify_rooted_artifact_closure(&root, entry, cwd, |_, _| Ok(()))
    }

    /// Inspect native metadata using an already acquired artifact owner.
    /// Check every actual object/provider read against the caller's frozen byte identity.
    pub fn verify_rooted_artifact_closure<F>(
        root: &ArtifactRoot,
        executable_path: &Path,
        launch_cwd: &Path,
        mut check_bytes: F,
    ) -> Result<(Self, Vec<PathBuf>), RuntimeArtifactError>
    where
        F: FnMut(&Path, &[u8]) -> Result<(), RuntimeArtifactError>,
    {
        let cwd = root.canonicalize(launch_cwd).map_err(fs_error)?;
        if !root.file_type(&cwd, true).map_err(fs_error)?.is_dir() {
            return invariant("native launch cwd escapes the artifact");
        }
        let mut members = BTreeSet::new();
        resolve_artifact_path(root, executable_path, &mut members)?;
        let mut visited = BTreeSet::new();
        let mut reader = NativeArtifactReader {
            root,
            check_bytes: &mut check_bytes,
        };
        let requirements = inspect_closure(
            &mut reader,
            &cwd,
            executable_path,
            true,
            &[],
            &mut visited,
            &mut members,
        )?;
        Ok((requirements, members.into_iter().collect()))
    }
}

const MAX_VERSION_ENTRIES: usize = 4096;

fn version_error() -> RuntimeArtifactError {
    RuntimeArtifactError::Invariant(
        "native ELF has malformed or unsupported version metadata".into(),
    )
}

fn dynamic_tag(elf: &Elf<'_>, tag: u64) -> Result<Option<u64>, RuntimeArtifactError> {
    let mut values = elf
        .dynamic
        .iter()
        .flat_map(|value| &value.dyns)
        .filter(|value| value.d_tag == tag);
    let first = values.next().map(|value| value.d_val);
    if values.next().is_some() {
        return Err(version_error());
    }
    Ok(first)
}

/// The loader uses dynamic tags even when an ELF has no section headers. Give
/// goblin a bounded view of that file-backed segment instead of trusting SHT_*.
fn version_section(
    elf: &Elf<'_>,
    bytes: &[u8],
    pointer_tag: u64,
    count_tag: u64,
    section_type: u32,
) -> Result<Option<section_header::SectionHeader>, RuntimeArtifactError> {
    let pair = (dynamic_tag(elf, pointer_tag)?, dynamic_tag(elf, count_tag)?);
    let (address, count) = match pair {
        (None, None) => return Ok(None),
        (Some(address), Some(count)) if count > 0 && count <= MAX_VERSION_ENTRIES as u64 => {
            (address, count)
        }
        _ => return Err(version_error()),
    };
    let mut segments = elf.program_headers.iter().filter(|segment| {
        segment.p_type == program_header::PT_LOAD
            && address >= segment.p_vaddr
            && segment
                .p_vaddr
                .checked_add(segment.p_filesz)
                .is_some_and(|end| address < end)
    });
    let segment = segments.next().ok_or_else(version_error)?;
    if segments.next().is_some() {
        return Err(version_error());
    }
    let offset = segment
        .p_offset
        .checked_add(address - segment.p_vaddr)
        .ok_or_else(version_error)?;
    let end = segment
        .p_offset
        .checked_add(segment.p_filesz)
        .filter(|end| *end <= bytes.len() as u64)
        .ok_or_else(version_error)?;
    Ok(Some(section_header::SectionHeader {
        sh_type: section_type,
        sh_offset: offset,
        sh_size: end - offset,
        sh_info: count as u32,
        ..Default::default()
    }))
}

fn version_name(elf: &Elf<'_>, offset: usize) -> Result<String, RuntimeArtifactError> {
    elf.dynstrtab
        .get_at(offset)
        .filter(|name| !name.is_empty() && name.len() <= 256)
        .map(str::to_owned)
        .ok_or_else(version_error)
}

// GNU version nodes use the SysV ELF name hash, not the GNU symbol-table hash.
fn version_name_hash(name: &str) -> u32 {
    let mut hash = 0_u32;
    for byte in name.bytes() {
        hash = (hash << 4).wrapping_add(u32::from(byte));
        let high = hash & 0xf000_0000;
        hash ^= high >> 24;
        hash &= !high;
    }
    hash
}

fn version_needs(
    elf: &Elf<'_>,
    bytes: &[u8],
) -> Result<BTreeMap<String, BTreeSet<String>>, RuntimeArtifactError> {
    let Some(section) = version_section(
        elf,
        bytes,
        dynamic::DT_VERNEED,
        dynamic::DT_VERNEEDNUM,
        section_header::SHT_GNU_VERNEED,
    )?
    else {
        return Ok(BTreeMap::new());
    };
    let count = section.sh_info as usize;
    let sections = [section];
    let parsed =
        symver::VerneedSection::parse(bytes, &sections, Ctx::new(Container::Big, Endian::Little))
            .map_err(|_| version_error())?
            .ok_or_else(version_error)?;
    let needs = parsed.iter().collect::<Vec<_>>();
    if needs.len() != count {
        return Err(version_error());
    }
    let mut result = BTreeMap::new();
    let mut total = 0;
    for (index, need) in needs.iter().enumerate() {
        total += usize::from(need.vn_cnt);
        if need.vn_version != 1
            || need.vn_cnt == 0
            || need.vn_aux < 16
            || total > MAX_VERSION_ENTRIES
            || (index + 1 == count && need.vn_next != 0)
            || (index + 1 < count && need.vn_next < 16)
        {
            return Err(version_error());
        }
        let library = version_name(elf, need.vn_file)?;
        if !elf.libraries.contains(&library.as_str()) || result.contains_key(&library) {
            return Err(version_error());
        }
        let auxiliary = need.iter().collect::<Vec<_>>();
        if auxiliary.len() != usize::from(need.vn_cnt) {
            return Err(version_error());
        }
        let mut versions = BTreeSet::new();
        for (index, version) in auxiliary.iter().enumerate() {
            if (index + 1 == auxiliary.len() && version.vna_next != 0)
                || (index + 1 < auxiliary.len() && version.vna_next < 16)
                || version.vna_flags & !symver::VER_FLG_WEAK != 0
            {
                return Err(version_error());
            }
            let name = version_name(elf, version.vna_name)?;
            if version.vna_flags & symver::VER_FLG_WEAK == 0 {
                if version.vna_hash != version_name_hash(&name) {
                    return Err(version_error());
                }
                versions.insert(name);
            }
        }
        result.insert(library, versions);
    }
    Ok(result)
}

fn version_definitions(
    elf: &Elf<'_>,
    bytes: &[u8],
) -> Result<BTreeSet<String>, RuntimeArtifactError> {
    let Some(section) = version_section(
        elf,
        bytes,
        dynamic::DT_VERDEF,
        dynamic::DT_VERDEFNUM,
        section_header::SHT_GNU_VERDEF,
    )?
    else {
        return Ok(BTreeSet::new());
    };
    let count = section.sh_info as usize;
    let sections = [section];
    let parsed =
        symver::VerdefSection::parse(bytes, &sections, Ctx::new(Container::Big, Endian::Little))
            .map_err(|_| version_error())?
            .ok_or_else(version_error)?;
    let definitions = parsed.iter().collect::<Vec<_>>();
    if definitions.len() != count {
        return Err(version_error());
    }
    let mut result = BTreeSet::new();
    let mut total = 0;
    for (index, definition) in definitions.iter().enumerate() {
        total += usize::from(definition.vd_cnt);
        if definition.vd_version != 1
            || definition.vd_cnt == 0
            || definition.vd_aux < 20
            || total > MAX_VERSION_ENTRIES
            || (index + 1 == count && definition.vd_next != 0)
            || (index + 1 < count && definition.vd_next < 20)
        {
            return Err(version_error());
        }
        let auxiliary = definition.iter().collect::<Vec<_>>();
        if auxiliary.len() != usize::from(definition.vd_cnt) {
            return Err(version_error());
        }
        for (index, name) in auxiliary.iter().enumerate() {
            if (index + 1 == auxiliary.len() && name.vda_next != 0)
                || (index + 1 < auxiliary.len() && name.vda_next < 8)
            {
                return Err(version_error());
            }
            let name = version_name(elf, name.vda_name)?;
            if index == 0 {
                if definition.vd_hash != version_name_hash(&name) {
                    return Err(version_error());
                }
                // Later verdaux names describe parents, not additional exports.
                result.insert(name);
            }
        }
    }
    Ok(result)
}

fn qualified_versions() -> &'static BTreeMap<String, BTreeSet<String>> {
    static VERSIONS: OnceLock<BTreeMap<String, BTreeSet<String>>> = OnceLock::new();
    VERSIONS.get_or_init(|| {
        let baseline: serde_json::Value = serde_json::from_str(
            NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARY_VERSIONS_JSON,
        )
        .expect("qualified native version baseline must be valid JSON");
        serde_json::from_value(baseline["libraries"].clone())
            .expect("qualified native version baseline must contain library exports")
    })
}

fn verify_versions(
    library: &str,
    required: &BTreeSet<String>,
    provided: &BTreeSet<String>,
) -> Result<(), RuntimeArtifactError> {
    if let Some(missing) = required.difference(provided).next() {
        return invariant(format!(
            "native library '{library}' does not provide required version '{missing}'"
        ));
    }
    Ok(())
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
    root: &ArtifactRoot,
    path: &Path,
    members: &mut BTreeSet<PathBuf>,
) -> Result<PathBuf, RuntimeArtifactError> {
    resolved_library_path(root, path, members)?
        .ok_or_else(|| fs_error(std::io::ErrorKind::NotFound.into()))
}

// Search directories may be absent, but lexical bounds and every existing
// symlink ancestor still have to be validated before the loader ignores them.
fn resolved_library_path(
    root: &ArtifactRoot,
    path: &Path,
    members: &mut BTreeSet<PathBuf>,
) -> Result<Option<PathBuf>, RuntimeArtifactError> {
    let relative = path;
    let mut depth = 0;
    for component in relative.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::ParentDir if depth == 0 => {
                return invariant("native library path escapes the artifact");
            }
            Component::ParentDir => depth -= 1,
            Component::CurDir => {}
            _ => return invariant("native library path escapes the artifact"),
        }
    }
    let mut pending = relative
        .components()
        .map(|part| part.as_os_str().to_owned())
        .collect::<VecDeque<_>>();
    let mut prefix = PathBuf::new();
    let mut links = 0;
    let mut aliases = BTreeSet::new();
    while let Some(part) = pending.pop_front() {
        if part == "." {
            continue;
        }
        if part == ".." {
            if prefix.as_os_str().is_empty() {
                return invariant("native library path escapes the artifact");
            }
            prefix.pop();
            continue;
        }
        prefix.push(part);
        let metadata = match root.file_type(&prefix, false) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(fs_error(error)),
        };
        if metadata.is_symlink() {
            links += 1;
            if links > 128 {
                return invariant("native artifact library symlink chain exceeds its bound");
            }
            aliases.insert(prefix.clone());
            let target = root.read_link(&prefix).map_err(fs_error)?;
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
    // Check the original path: normalizing before resolving symlinks changes OS
    // semantics and can conceal an escape through a link followed by `..`.
    let canonical = root.canonicalize(path).map_err(fs_error)?;
    if prefix != canonical {
        return invariant("native artifact library path changed while resolving");
    }
    members.extend(aliases);
    Ok(Some(prefix))
}

fn loader_paths(
    root: &ArtifactRoot,
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
        if resolved_library_path(root, &candidate, members)?.is_none() {
            continue;
        }
        if !root.file_type(&candidate, true).map_err(fs_error)?.is_dir() {
            return invariant("native artifact library path is not a directory");
        }
        result.push(candidate);
    }
    Ok(result)
}

struct NativeArtifactReader<'a, F> {
    root: &'a ArtifactRoot,
    check_bytes: &'a mut F,
}

impl<F> NativeArtifactReader<'_, F>
where
    F: FnMut(&Path, &[u8]) -> Result<(), RuntimeArtifactError>,
{
    fn read(&mut self, path: &Path) -> Result<Vec<u8>, RuntimeArtifactError> {
        let bytes = self.root.read(path).map_err(fs_error)?;
        (self.check_bytes)(path, &bytes)?;
        Ok(bytes)
    }
}

fn inspect_closure<F>(
    reader: &mut NativeArtifactReader<'_, F>,
    cwd: &Path,
    object: &Path,
    executable: bool,
    inherited_rpaths: &[PathBuf],
    visited: &mut BTreeSet<PathBuf>,
    members: &mut BTreeSet<PathBuf>,
) -> Result<NativeExecutableRequirements, RuntimeArtifactError>
where
    F: FnMut(&Path, &[u8]) -> Result<(), RuntimeArtifactError>,
{
    let root = reader.root;
    if visited.len() >= 128 {
        return invariant("native library closure exceeds its object bound");
    }
    let canonical = resolve_artifact_path(root, object, members)?;
    let metadata = root.file_type(&canonical, true).map_err(fs_error)?;
    if !metadata.is_file() {
        return invariant("native library must be a regular file");
    }
    let bytes = reader.read(&canonical)?;
    let elf = Elf::parse(&bytes).map_err(|error| {
        RuntimeArtifactError::Invariant(format!("invalid native artifact ELF: {error}"))
    })?;
    let requirements = inspect_elf(&elf, &bytes, executable)?;
    let version_needs = version_needs(&elf, &bytes)?;
    version_definitions(&elf, &bytes)?;
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
    members.insert(canonical.clone());
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
            match root.file_type(&candidate, false) {
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
            if let Some(required) = version_needs.get(library) {
                let provider_bytes = reader.read(&canonical)?;
                let provider = Elf::parse(&provider_bytes).map_err(|error| {
                    RuntimeArtifactError::Invariant(format!("invalid native library ELF: {error}"))
                })?;
                verify_versions(
                    library,
                    required,
                    &version_definitions(&provider, &provider_bytes)?,
                )?;
            }
            if !visited.contains(&canonical) {
                inspect_closure(
                    reader,
                    cwd,
                    &loader_path,
                    false,
                    &inherited,
                    visited,
                    members,
                )?;
            }
        } else if !NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARIES
            .contains(&library.as_str())
        {
            return invariant(format!(
                "native artifact lacks required library '{library}' requested by {}",
                object.display()
            ));
        } else if let Some(required) = version_needs.get(library) {
            verify_versions(library, required, &qualified_versions()[library])?;
        }
    }
    Ok(requirements)
}

#[cfg(test)]
#[path = "native_executable_tests.rs"]
mod tests;
