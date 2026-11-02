//! Native entry admission is independent of managed interpreter authorization.
use goblin::{
    container::{Container, Ctx, Endian},
    elf::{Elf, dynamic, header, program_header, section_header, symver},
};
use scroll::Pread as _;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::File,
    io::{BufReader, Cursor, Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};

use crate::{ArtifactRoot, RuntimeArtifactError, invariant};

pub const NATIVE_EXECUTION_TARGET: &str = "native-linux-x86_64-glibc";
/// Metadata allowance for compatibility callers without an explicit owner budget.
pub const DEFAULT_NATIVE_METADATA_BYTES: u64 = 256 * 1024 * 1024;
/// Streamed file hashing and fixed parser scratch, excluding charged owned metadata.
pub const NATIVE_STREAM_WORKING_MEMORY_BYTES: u64 = 128 * 1024;

struct LoaderElf {
    header: header::Header,
    program_headers: Vec<program_header::ProgramHeader>,
    interpreter: Option<String>,
    dynamic: Option<dynamic::Dynamic>,
    libraries: Vec<String>,
    rpaths: Vec<String>,
    runpaths: Vec<String>,
    entry: u64,
}

struct NativeInput<R> {
    reader: BufReader<R>,
    length: u64,
    position: u64,
}
impl<R: Read + Seek> NativeInput<R> {
    fn new(reader: R, length: u64) -> Self {
        Self {
            reader: BufReader::with_capacity(64 * 1024, reader),
            length,
            position: 0,
        }
    }
    fn seek(&mut self, offset: u64) -> Result<(), RuntimeArtifactError> {
        let relative = i128::from(offset) - i128::from(self.position);
        if let Ok(relative) = i64::try_from(relative) {
            self.reader.seek_relative(relative).map_err(fs_error)?;
        } else {
            self.reader
                .seek(SeekFrom::Start(offset))
                .map_err(fs_error)?;
        }
        self.position = offset;
        Ok(())
    }
    fn record<const N: usize>(&mut self, offset: u64) -> Result<[u8; N], RuntimeArtifactError> {
        self.range(offset, N as u64)?;
        self.seek(offset)?;
        let mut bytes = [0; N];
        self.reader.read_exact(&mut bytes).map_err(fs_error)?;
        self.position += N as u64;
        Ok(bytes)
    }
    fn range(&self, offset: u64, size: u64) -> Result<(), RuntimeArtifactError> {
        if offset.checked_add(size).is_none_or(|end| end > self.length) {
            return Err(version_error());
        }
        Ok(())
    }
    fn string(
        &mut self,
        offset: u64,
        available: u64,
        maximum: usize,
        budget: &mut NativeClosureReadBudget,
    ) -> Result<String, RuntimeArtifactError> {
        self.range(offset, available)?;
        self.seek(offset)?;
        let mut remaining = available.min(maximum as u64 + 1);
        let mut result = Vec::new();
        while remaining > 0 {
            let mut chunk = [0; 256];
            let count = remaining.min(chunk.len() as u64) as usize;
            self.reader
                .read_exact(&mut chunk[..count])
                .map_err(fs_error)?;
            self.position += count as u64;
            let nul = chunk[..count].iter().position(|byte| *byte == 0);
            let used = nul.unwrap_or(count);
            // Own the string and simultaneous old/new Vec growth before allocating.
            budget.charge(used.checked_mul(4).ok_or_else(version_error)?)?;
            result.try_reserve(used).map_err(|_| version_error())?;
            result.extend_from_slice(&chunk[..used]);
            if nul.is_some() {
                return String::from_utf8(result).map_err(|_| version_error());
            }
            remaining -= count as u64;
        }
        Err(version_error())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExecutableRequirements {
    pub interpreter: Option<String>,
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
}

/// Callers reserve the raw file buffer and this metadata allowance separately
/// from retained manifests and any caller-owned path index/Python reachability.
/// Metadata charges are conservative and cumulative,
/// including allocations dropped before later closure objects are inspected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeClosureReadLimits {
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_metadata_bytes: u64,
}

/// Shared byte admission across one or more native closures.
#[derive(Debug)]
pub struct NativeClosureReadBudget {
    limits: NativeClosureReadLimits,
    used_bytes: u64,
    used_metadata_bytes: u64,
}

impl NativeClosureReadBudget {
    pub const fn new(limits: NativeClosureReadLimits) -> Self {
        Self {
            limits,
            used_bytes: 0,
            used_metadata_bytes: 0,
        }
    }

    /// Bytes admitted for reading, including failed reads or identity checks.
    pub const fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    pub const fn used_metadata_bytes(&self) -> u64 {
        self.used_metadata_bytes
    }

    fn charge(&mut self, bytes: usize) -> Result<(), RuntimeArtifactError> {
        self.used_metadata_bytes = self
            .used_metadata_bytes
            .checked_add(bytes as u64)
            .filter(|total| *total <= self.limits.max_metadata_bytes)
            .ok_or_else(|| {
                RuntimeArtifactError::Invariant(
                    "native closure exceeds its metadata byte limit".into(),
                )
            })?;
        Ok(())
    }

    fn entries<T>(&mut self, count: usize) -> Result<(), RuntimeArtifactError> {
        // Cover Vec growth, tree-node slack and allocator bookkeeping before allocation.
        self.charge(
            count
                .checked_mul(std::mem::size_of::<T>() + 256)
                .ok_or_else(version_error)?,
        )
    }

    fn text(&mut self, text: &str) -> Result<(), RuntimeArtifactError> {
        self.entries::<String>(1)?;
        self.charge(text.len().checked_mul(2).ok_or_else(version_error)?)
    }

    fn path(&mut self, path: &Path) -> Result<(), RuntimeArtifactError> {
        self.entries::<PathBuf>(1)?;
        self.charge(
            path.as_os_str()
                .len()
                .checked_mul(2)
                .ok_or_else(version_error)?,
        )
    }

    fn path_read(&mut self, path: &Path) -> Result<(), RuntimeArtifactError> {
        self.path(path)?;
        // The rooted filesystem adapter may expand a Linux symlink/realpath result.
        self.charge(4 * 4096)
    }
}

/// Validate the supported executable format, returning its library requirements.
/// Presence in a Builder image does not prove that a runtime can satisfy them.
pub fn verify_native_executable(
    bytes: &[u8],
) -> Result<NativeExecutableRequirements, RuntimeArtifactError> {
    // The standalone path uses the same admission and parser. Existing semantic
    // maxima bound its owned output; bytes are already owned by its caller.
    let mut budget = standalone_budget(bytes.len());
    let mut input = NativeInput {
        reader: BufReader::with_capacity(0, Cursor::new(bytes)),
        length: bytes.len() as u64,
        position: 0,
    };
    let elf = parse_loader_elf(&mut input, &mut budget)?;
    inspect_elf(&elf, input.length, true, &mut budget)
}

fn standalone_budget(bytes: usize) -> NativeClosureReadBudget {
    NativeClosureReadBudget::new(NativeClosureReadLimits {
        max_file_bytes: bytes as u64,
        max_total_bytes: bytes as u64,
        max_metadata_bytes: DEFAULT_NATIVE_METADATA_BYTES,
    })
}

fn parse_loader_elf<R: Read + Seek>(
    input: &mut NativeInput<R>,
    budget: &mut NativeClosureReadBudget,
) -> Result<LoaderElf, RuntimeArtifactError> {
    let header = input.record::<64>(0)?;
    let parsed = (|| -> Result<Elf<'_>, goblin::error::Error> {
        Elf::lazy_parse(Elf::parse_header(&header)?)
    })()
    .map_err(|error| {
        RuntimeArtifactError::Invariant(format!("invalid native ELF entry: {error}"))
    })?;
    let mut elf = LoaderElf {
        entry: parsed.entry,
        header: parsed.header,
        program_headers: Vec::new(),
        interpreter: None,
        dynamic: None,
        libraries: Vec::new(),
        rpaths: Vec::new(),
        runpaths: Vec::new(),
    };
    let ctx = Ctx::new(Container::Big, Endian::Little);
    // Reject other layouts before interpreting any program or dynamic records.
    if elf.header.e_ident[header::EI_CLASS] != header::ELFCLASS64
        || elf.header.e_ident[header::EI_DATA] != header::ELFDATA2LSB
    {
        return invariant("native entry requires a runnable Linux x86_64 ELF executable");
    }
    let phoff = usize::try_from(elf.header.e_phoff).map_err(|_| version_error())?;
    let count = usize::from(elf.header.e_phnum);
    let end = count
        .checked_mul(program_header::ProgramHeader::size(ctx))
        .and_then(|size| phoff.checked_add(size))
        .ok_or_else(version_error)?;
    if end as u64 > input.length {
        return Err(version_error());
    }
    budget.entries::<program_header::ProgramHeader>(count)?;
    elf.program_headers
        .try_reserve_exact(count)
        .map_err(|_| version_error())?;
    for index in 0..count {
        let bytes = input.record::<56>((phoff + index * 56) as u64)?;
        elf.program_headers.push(
            program_header::ProgramHeader::parse(&bytes, 0, 1, ctx)
                .map_err(|_| version_error())?
                .pop()
                .ok_or_else(version_error)?,
        );
    }
    for segment in &elf.program_headers {
        if segment.p_type == program_header::PT_INTERP {
            if !(2..=4096).contains(&segment.p_filesz) {
                return invariant(
                    "native entry requires one valid qualified Linux glibc interpreter segment",
                );
            }
            let path = input
                .string(segment.p_offset, segment.p_filesz, 4095, budget)
                .map_err(|_| {
                    RuntimeArtifactError::Invariant(
                        "native entry requires one valid qualified Linux glibc interpreter segment"
                            .into(),
                    )
                })?;
            if path.len() as u64 + 1 != segment.p_filesz {
                return invariant(
                    "native entry requires one valid qualified Linux glibc interpreter segment",
                );
            }
            elf.interpreter = Some(path);
        }
    }
    let mut segments = elf
        .program_headers
        .iter()
        .filter(|segment| segment.p_type == program_header::PT_DYNAMIC);
    if let Some(segment) = segments.next() {
        if segments.next().is_some() {
            return Err(version_error());
        }
        input.range(segment.p_offset, segment.p_filesz)?;
        let mut dyns = Vec::new();
        let mut info = dynamic::DynamicInfo::default();
        let mut terminated = false;
        let mut needed = 0;
        let mut paths = 0;
        for offset in (0..segment.p_filesz / 16).map(|index| segment.p_offset + index * 16) {
            let record = input.record::<16>(offset)?;
            let value: dynamic::Dyn = record.pread_with(0, ctx).map_err(|_| version_error())?;
            if value.d_tag == dynamic::DT_NULL {
                terminated = true;
                break;
            }
            match value.d_tag {
                dynamic::DT_NEEDED => {
                    needed += 1;
                    if needed > 128 {
                        return Err(version_error());
                    }
                }
                dynamic::DT_RPATH | dynamic::DT_RUNPATH => {
                    paths += 1;
                    if paths > 128 {
                        return Err(version_error());
                    }
                }
                dynamic::DT_FLAGS_1 => {
                    info.flags_1 = value.d_val;
                    continue;
                }
                dynamic::DT_AUDIT | dynamic::DT_DEPAUDIT | 0x7fff_ffff | 0x7fff_fffd => {
                    return invariant(
                        "native ELF uses unsupported audit/filter/nodefaultlib loading",
                    );
                }
                dynamic::DT_STRTAB
                | dynamic::DT_STRSZ
                | dynamic::DT_VERNEED
                | dynamic::DT_VERNEEDNUM
                | dynamic::DT_VERDEF
                | dynamic::DT_VERDEFNUM => {
                    if dyns
                        .iter()
                        .any(|item: &dynamic::Dyn| item.d_tag == value.d_tag)
                    {
                        return Err(version_error());
                    }
                }
                _ => continue,
            }
            budget.entries::<dynamic::Dyn>(1)?;
            dyns.try_reserve_exact(1).map_err(|_| version_error())?;
            dyns.push(value);
        }
        if !terminated {
            return Err(version_error());
        }
        if info.flags_1 & dynamic::DF_1_NODEFLIB != 0 {
            return invariant("native ELF uses unsupported audit/filter/nodefaultlib loading");
        }
        let pointer = dyns
            .iter()
            .find(|value| value.d_tag == dynamic::DT_STRTAB)
            .map(|value| value.d_val);
        let length = dyns
            .iter()
            .find(|value| value.d_tag == dynamic::DT_STRSZ)
            .map(|value| value.d_val);
        match (pointer, length) {
            (Some(pointer), Some(length)) => {
                info.strtab = virtual_offset(&elf.program_headers, pointer, length, input.length)?;
                info.strsz = usize::try_from(length).map_err(|_| version_error())?;
            }
            (None, None) if needed == 0 && paths == 0 => (),
            _ => return Err(version_error()),
        }
        elf.dynamic = Some(dynamic::Dynamic { dyns, info });
        for value in &elf.dynamic.as_ref().unwrap().dyns {
            let destination = match value.d_tag {
                dynamic::DT_NEEDED => &mut elf.libraries,
                dynamic::DT_RPATH => &mut elf.rpaths,
                dynamic::DT_RUNPATH => &mut elf.runpaths,
                _ => continue,
            };
            let maximum = if value.d_tag == dynamic::DT_NEEDED {
                256
            } else {
                128 * 4097
            };
            let name = dynamic_string(
                input,
                &elf.dynamic.as_ref().unwrap().info,
                value.d_val,
                maximum,
                budget,
            )?;
            budget.entries::<&str>(1)?;
            destination
                .try_reserve_exact(1)
                .map_err(|_| version_error())?;
            destination.push(name);
        }
    }
    Ok(elf)
}

fn virtual_offset(
    headers: &[program_header::ProgramHeader],
    address: u64,
    size: u64,
    file_bytes: u64,
) -> Result<usize, RuntimeArtifactError> {
    let end = address.checked_add(size).ok_or_else(version_error)?;
    let mut candidates = headers.iter().filter(|segment| {
        segment.p_type == program_header::PT_LOAD
            && address >= segment.p_vaddr
            && segment
                .p_vaddr
                .checked_add(segment.p_filesz)
                .is_some_and(|limit| end <= limit)
    });
    let segment = candidates.next().ok_or_else(version_error)?;
    if candidates.next().is_some() {
        return Err(version_error());
    }
    let offset = segment
        .p_offset
        .checked_add(address - segment.p_vaddr)
        .ok_or_else(version_error)?;
    if offset.checked_add(size).is_none_or(|end| end > file_bytes) {
        return Err(version_error());
    }
    usize::try_from(offset).map_err(|_| version_error())
}

fn dynamic_string<R: Read + Seek>(
    input: &mut NativeInput<R>,
    info: &dynamic::DynamicInfo,
    offset: u64,
    maximum: usize,
    budget: &mut NativeClosureReadBudget,
) -> Result<String, RuntimeArtifactError> {
    input.range(info.strtab as u64, info.strsz as u64)?;
    let available = (info.strsz as u64)
        .checked_sub(offset)
        .ok_or_else(version_error)?;
    input.string(
        (info.strtab as u64)
            .checked_add(offset)
            .ok_or_else(version_error)?,
        available,
        maximum,
        budget,
    )
}

fn inspect_elf(
    elf: &LoaderElf,
    length: u64,
    executable: bool,
    budget: &mut NativeClosureReadBudget,
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
                .is_some_and(|end| end <= length)
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
        verify_entry_interpreter(elf)?;
    }
    if elf
        .interpreter
        .as_deref()
        .is_some_and(|path| path != "/lib64/ld-linux-x86-64.so.2")
    {
        return invariant("native entry requires the qualified Linux glibc interpreter");
    }
    if elf.libraries.len() > 128
        || elf.libraries.iter().any(|name| {
            name.is_empty()
                || name.len() > 256
                || name.contains(['/', '\\', '\0', '$'])
                || name == "."
                || name == ".."
        })
    {
        return invariant("native entry has invalid dynamic library requirements");
    }
    let mut library_paths = Vec::new();
    for path in elf
        .rpaths
        .iter()
        .chain(&elf.runpaths)
        .flat_map(|path| path.split(':'))
    {
        if library_paths.len() >= 128
            || path.is_empty()
            || path.len() > 4096
            || path.contains(['\0', '\\'])
        {
            return invariant("native entry has invalid dynamic library paths");
        }
        budget.text(path)?;
        library_paths.push(path.to_owned());
    }
    for name in &elf.libraries {
        budget.text(name)?;
    }
    if let Some(interpreter) = &elf.interpreter {
        budget.text(interpreter)?;
    }
    budget.entries::<String>(elf.libraries.len())?;
    Ok(NativeExecutableRequirements {
        interpreter: elf.interpreter.clone(),
        libraries: elf.libraries.clone(),
        library_paths,
    })
}

fn verify_entry_interpreter(elf: &LoaderElf) -> Result<(), RuntimeArtifactError> {
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
    if segments.next().is_some()
        || !(2..=4096).contains(&segment.p_filesz)
        || elf.interpreter.as_deref() != Some("/lib64/ld-linux-x86-64.so.2")
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
        check_bytes: F,
    ) -> Result<(Self, Vec<PathBuf>), RuntimeArtifactError>
    where
        F: FnMut(&Path, &[u8]) -> Result<(), RuntimeArtifactError>,
    {
        Self::verify_rooted_artifact_closure_bounded(
            root,
            executable_path,
            launch_cwd,
            &mut NativeClosureReadBudget::new(NativeClosureReadLimits {
                max_file_bytes: u64::MAX,
                max_total_bytes: u64::MAX,
                max_metadata_bytes: DEFAULT_NATIVE_METADATA_BYTES,
            }),
            check_bytes,
        )
    }

    /// Compatibility callers retain the complete byte callback and fund its buffer.
    pub fn verify_rooted_artifact_closure_bounded<F>(
        root: &ArtifactRoot,
        executable_path: &Path,
        launch_cwd: &Path,
        budget: &mut NativeClosureReadBudget,
        mut check_bytes: F,
    ) -> Result<(Self, Vec<PathBuf>), RuntimeArtifactError>
    where
        F: FnMut(&Path, &[u8]) -> Result<(), RuntimeArtifactError>,
    {
        verify_closure(
            root,
            executable_path,
            launch_cwd,
            budget,
            |path, file, length| {
                let length = usize::try_from(length).map_err(|_| version_error())?;
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(length)
                    .map_err(|_| version_error())?;
                bytes.resize(length, 0);
                file.read_exact(&mut bytes).map_err(fs_error)?;
                check_bytes(path, &bytes)
            },
        )
    }

    /// Verify each entire rooted file with bounded hashing scratch before parsing
    /// its loader metadata. Identity stays pinned across hashing, callback and parsing.
    pub fn verify_rooted_artifact_closure_streamed<F>(
        root: &ArtifactRoot,
        executable_path: &Path,
        launch_cwd: &Path,
        budget: &mut NativeClosureReadBudget,
        mut check_digest: F,
    ) -> Result<(Self, Vec<PathBuf>), RuntimeArtifactError>
    where
        F: FnMut(&Path, u64, blake3::Hash) -> Result<(), RuntimeArtifactError>,
    {
        verify_closure(
            root,
            executable_path,
            launch_cwd,
            budget,
            |path, file, length| {
                let mut hash = blake3::Hasher::new();
                let mut scratch = vec![0; 64 * 1024].into_boxed_slice();
                let mut remaining = length;
                while remaining > 0 {
                    let count = remaining.min(scratch.len() as u64) as usize;
                    file.read_exact(&mut scratch[..count]).map_err(fs_error)?;
                    hash.update(&scratch[..count]);
                    remaining -= count as u64;
                }
                check_digest(path, length, hash.finalize())
            },
        )
    }
}

fn verify_closure<F>(
    root: &ArtifactRoot,
    executable_path: &Path,
    launch_cwd: &Path,
    budget: &mut NativeClosureReadBudget,
    mut check_file: F,
) -> Result<(NativeExecutableRequirements, Vec<PathBuf>), RuntimeArtifactError>
where
    F: FnMut(&Path, &mut File, u64) -> Result<(), RuntimeArtifactError>,
{
    budget.path_read(launch_cwd)?;
    let cwd = root.canonicalize(launch_cwd).map_err(fs_error)?;
    if !root.file_type(&cwd, true).map_err(fs_error)?.is_dir() {
        return invariant("native launch cwd escapes the artifact");
    }
    let mut members = BTreeSet::new();
    resolve_artifact_path(root, executable_path, &mut members, budget)?;
    let mut visited = BTreeSet::new();
    let mut reader = NativeArtifactReader {
        root,
        check_file: &mut check_file,
        budget,
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

const MAX_VERSION_ENTRIES: usize = 4096;

fn version_error() -> RuntimeArtifactError {
    RuntimeArtifactError::Invariant(
        "native ELF has malformed or unsupported version metadata".into(),
    )
}

fn dynamic_tag(elf: &LoaderElf, tag: u64) -> Result<Option<u64>, RuntimeArtifactError> {
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

/// The loader uses dynamic tags even when an ELF has no section headers.
fn version_section(
    elf: &LoaderElf,
    length: u64,
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
        .filter(|end| *end <= length)
        .ok_or_else(version_error)?;
    Ok(Some(section_header::SectionHeader {
        sh_type: section_type,
        sh_offset: offset,
        sh_size: end - offset,
        sh_info: count as u32,
        ..Default::default()
    }))
}

fn version_name<R: Read + Seek>(
    elf: &LoaderElf,
    input: &mut NativeInput<R>,
    offset: usize,
    budget: &mut NativeClosureReadBudget,
) -> Result<String, RuntimeArtifactError> {
    let info = &elf.dynamic.as_ref().ok_or_else(version_error)?.info;
    let name = dynamic_string(input, info, offset as u64, 256, budget)?;
    if name.is_empty() || name.len() > 256 {
        return Err(version_error());
    }
    budget.text(&name)?;
    Ok(name)
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

fn version_record<R: Read + Seek, const N: usize>(
    input: &mut NativeInput<R>,
    section: &section_header::SectionHeader,
    relative: u64,
) -> Result<[u8; N], RuntimeArtifactError> {
    if relative
        .checked_add(N as u64)
        .is_none_or(|end| end > section.sh_size)
    {
        return Err(version_error());
    }
    input.record(
        section
            .sh_offset
            .checked_add(relative)
            .ok_or_else(version_error)?,
    )
}
fn elf_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn elf_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn version_needs<R: Read + Seek>(
    elf: &LoaderElf,
    input: &mut NativeInput<R>,
    budget: &mut NativeClosureReadBudget,
) -> Result<BTreeMap<String, BTreeSet<String>>, RuntimeArtifactError> {
    let Some(section) = version_section(
        elf,
        input.length,
        dynamic::DT_VERNEED,
        dynamic::DT_VERNEEDNUM,
        section_header::SHT_GNU_VERNEED,
    )?
    else {
        return Ok(BTreeMap::new());
    };
    let count = section.sh_info as usize;
    budget.entries::<symver::Verneed<'static>>(count)?;
    let mut result = BTreeMap::new();
    let mut total = 0usize;
    let mut cursor = 0u64;
    for index in 0..count {
        let record = version_record::<_, 16>(input, &section, cursor)?;
        let (version, entries, file, aux, next) = (
            elf_u16(&record, 0),
            elf_u16(&record, 2) as usize,
            elf_u32(&record, 4),
            elf_u32(&record, 8),
            elf_u32(&record, 12),
        );
        total += entries;
        if version != 1
            || entries == 0
            || aux < 16
            || total > MAX_VERSION_ENTRIES
            || (index + 1 == count && next != 0)
            || (index + 1 < count && next < 16)
        {
            return Err(version_error());
        }
        let library = version_name(elf, input, file as usize, budget)?;
        if !elf.libraries.contains(&library) || result.contains_key(&library) {
            return Err(version_error());
        }
        budget.entries::<symver::Vernaux>(entries)?;
        let mut auxiliary = cursor
            .checked_add(u64::from(aux))
            .ok_or_else(version_error)?;
        let mut versions = BTreeSet::new();
        for item in 0..entries {
            let record = version_record::<_, 16>(input, &section, auxiliary)?;
            let (hash, flags, name, next) = (
                elf_u32(&record, 0),
                elf_u16(&record, 4),
                elf_u32(&record, 8),
                elf_u32(&record, 12),
            );
            if (item + 1 == entries && next != 0)
                || (item + 1 < entries && next < 16)
                || flags & !symver::VER_FLG_WEAK != 0
            {
                return Err(version_error());
            }
            let name = version_name(elf, input, name as usize, budget)?;
            if flags & symver::VER_FLG_WEAK == 0 {
                if hash != version_name_hash(&name) {
                    return Err(version_error());
                }
                budget.entries::<String>(1)?;
                versions.insert(name);
            }
            auxiliary = auxiliary
                .checked_add(u64::from(next))
                .ok_or_else(version_error)?;
        }
        budget.entries::<(String, BTreeSet<String>)>(1)?;
        result.insert(library, versions);
        cursor = cursor
            .checked_add(u64::from(next))
            .ok_or_else(version_error)?;
    }
    Ok(result)
}

fn version_definitions<R: Read + Seek>(
    elf: &LoaderElf,
    input: &mut NativeInput<R>,
    budget: &mut NativeClosureReadBudget,
) -> Result<BTreeSet<String>, RuntimeArtifactError> {
    let Some(section) = version_section(
        elf,
        input.length,
        dynamic::DT_VERDEF,
        dynamic::DT_VERDEFNUM,
        section_header::SHT_GNU_VERDEF,
    )?
    else {
        return Ok(BTreeSet::new());
    };
    let count = section.sh_info as usize;
    budget.entries::<symver::Verdef<'static>>(count)?;
    let mut result = BTreeSet::new();
    let mut total = 0usize;
    let mut cursor = 0u64;
    for index in 0..count {
        let record = version_record::<_, 20>(input, &section, cursor)?;
        let (version, entries, hash, aux, next) = (
            elf_u16(&record, 0),
            elf_u16(&record, 6) as usize,
            elf_u32(&record, 8),
            elf_u32(&record, 12),
            elf_u32(&record, 16),
        );
        total += entries;
        if version != 1
            || entries == 0
            || aux < 20
            || total > MAX_VERSION_ENTRIES
            || (index + 1 == count && next != 0)
            || (index + 1 < count && next < 20)
        {
            return Err(version_error());
        }
        budget.entries::<symver::Verdaux>(entries)?;
        let mut auxiliary = cursor
            .checked_add(u64::from(aux))
            .ok_or_else(version_error)?;
        for item in 0..entries {
            let record = version_record::<_, 8>(input, &section, auxiliary)?;
            let (name, next) = (elf_u32(&record, 0), elf_u32(&record, 4));
            if (item + 1 == entries && next != 0) || (item + 1 < entries && next < 8) {
                return Err(version_error());
            }
            let name = version_name(elf, input, name as usize, budget)?;
            if item == 0 {
                if hash != version_name_hash(&name) {
                    return Err(version_error());
                }
                budget.entries::<String>(1)?;
                result.insert(name);
            }
            auxiliary = auxiliary
                .checked_add(u64::from(next))
                .ok_or_else(version_error)?;
        }
        cursor = cursor
            .checked_add(u64::from(next))
            .ok_or_else(version_error)?;
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
    budget: &mut NativeClosureReadBudget,
) -> Result<PathBuf, RuntimeArtifactError> {
    resolved_library_path(root, path, members, budget)?
        .ok_or_else(|| fs_error(std::io::ErrorKind::NotFound.into()))
}

// Search directories may be absent, but lexical bounds and every existing
// symlink ancestor still have to be validated before the loader ignores them.
fn resolved_library_path(
    root: &ArtifactRoot,
    path: &Path,
    members: &mut BTreeSet<PathBuf>,
    budget: &mut NativeClosureReadBudget,
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
    budget.entries::<std::ffi::OsString>(relative.components().count())?;
    budget.charge(
        relative
            .as_os_str()
            .len()
            .checked_mul(2)
            .ok_or_else(version_error)?,
    )?;
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
        budget.path(&prefix)?;
        budget.charge(part.len().checked_mul(2).ok_or_else(version_error)?)?;
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
            budget.path(&prefix)?;
            aliases.insert(prefix.clone());
            budget.path_read(&prefix)?;
            let target = root.read_link(&prefix).map_err(fs_error)?;
            if target.is_absolute() || has_intermediate_parent(&target) {
                return invariant(
                    "native artifact library symlink requires a relative target without intermediate parent traversal",
                );
            }
            prefix.pop();
            for part in target.components().rev() {
                budget.entries::<std::ffi::OsString>(1)?;
                budget.charge(
                    part.as_os_str()
                        .len()
                        .checked_mul(2)
                        .ok_or_else(version_error)?,
                )?;
                pending.push_front(part.as_os_str().to_owned());
            }
        }
    }
    // Check the original path: normalizing before resolving symlinks changes OS
    // semantics and can conceal an escape through a link followed by `..`.
    budget.path_read(path)?;
    let canonical = root.canonicalize(path).map_err(fs_error)?;
    if prefix != canonical {
        return invariant("native artifact library path changed while resolving");
    }
    budget.entries::<PathBuf>(aliases.len())?;
    members.extend(aliases);
    Ok(Some(prefix))
}

fn loader_paths(
    root: &ArtifactRoot,
    cwd: &Path,
    object: &Path,
    paths: &[String],
    members: &mut BTreeSet<PathBuf>,
    budget: &mut NativeClosureReadBudget,
) -> Result<Vec<PathBuf>, RuntimeArtifactError> {
    let mut result = Vec::new();
    for path in paths.iter().flat_map(|path| path.split(':')) {
        budget.path(object)?;
        budget.path(cwd)?;
        budget.text(path)?;
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
        if resolved_library_path(root, &candidate, members, budget)?.is_none() {
            continue;
        }
        if !root.file_type(&candidate, true).map_err(fs_error)?.is_dir() {
            return invariant("native artifact library path is not a directory");
        }
        budget.entries::<PathBuf>(1)?;
        result.push(candidate);
    }
    Ok(result)
}

fn same_file(before: &std::fs::Metadata, after: &std::fs::Metadata) -> bool {
    let common = before.len() == after.len() && before.modified().ok() == after.modified().ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        common
            && (
                before.dev(),
                before.ino(),
                before.mode(),
                before.nlink(),
                before.mtime_nsec(),
                before.ctime(),
                before.ctime_nsec(),
            ) == (
                after.dev(),
                after.ino(),
                after.mode(),
                after.nlink(),
                after.mtime_nsec(),
                after.ctime(),
                after.ctime_nsec(),
            )
    }
    #[cfg(not(unix))]
    {
        common
    }
}

struct NativeArtifactReader<'a, F> {
    root: &'a ArtifactRoot,
    check_file: &'a mut F,
    budget: &'a mut NativeClosureReadBudget,
}
impl<F> NativeArtifactReader<'_, F>
where
    F: FnMut(&Path, &mut File, u64) -> Result<(), RuntimeArtifactError>,
{
    fn with_file<T>(
        &mut self,
        path: &Path,
        inspect: impl FnOnce(
            &mut NativeInput<&mut File>,
            &mut NativeClosureReadBudget,
        ) -> Result<T, RuntimeArtifactError>,
    ) -> Result<T, RuntimeArtifactError> {
        let mut file = self.root.open_file(path).map_err(fs_error)?;
        let before = file.metadata().map_err(fs_error)?;
        let length = before.len();
        if length > self.budget.limits.max_file_bytes {
            return invariant("native closure file exceeds its byte limit");
        }
        self.budget.used_bytes = self
            .budget
            .used_bytes
            .checked_add(length)
            .filter(|total| *total <= self.budget.limits.max_total_bytes)
            .ok_or_else(|| {
                RuntimeArtifactError::Invariant(
                    "native closure exceeds its aggregate byte limit".into(),
                )
            })?;
        (self.check_file)(path, &mut file, length)?;
        if file.read(&mut [0]).map_err(fs_error)? != 0
            || !same_file(&before, &file.metadata().map_err(fs_error)?)
        {
            return invariant("native closure file changed during identity verification");
        }
        file.seek(SeekFrom::Start(0)).map_err(fs_error)?;
        let result = inspect(&mut NativeInput::new(&mut file, length), self.budget);
        let named = self
            .root
            .open_file(path)
            .map_err(fs_error)?
            .metadata()
            .map_err(fs_error)?;
        if !same_file(&before, &file.metadata().map_err(fs_error)?) || !same_file(&before, &named) {
            return invariant("native closure file changed during metadata inspection");
        }
        result
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
    F: FnMut(&Path, &mut File, u64) -> Result<(), RuntimeArtifactError>,
{
    let root = reader.root;
    if visited.len() >= 128 {
        return invariant("native library closure exceeds its object bound");
    }
    let canonical = resolve_artifact_path(root, object, members, reader.budget)?;
    let metadata = root.file_type(&canonical, true).map_err(fs_error)?;
    if !metadata.is_file() {
        return invariant("native library must be a regular file");
    }
    let (elf, requirements, version_needs) = reader.with_file(&canonical, |input, budget| {
        let elf = parse_loader_elf(input, budget)?;
        let requirements = inspect_elf(&elf, input.length, executable, budget)?;
        let needs = version_needs(&elf, input, budget)?;
        version_definitions(&elf, input, budget)?;
        Ok((elf, requirements, needs))
    })?;
    reader.budget.path(&canonical)?;
    members.insert(canonical.clone());
    reader.budget.entries::<PathBuf>(1)?;
    visited.insert(canonical);
    let rpaths = loader_paths(root, cwd, object, &elf.rpaths, members, reader.budget)?;
    let runpaths = loader_paths(root, cwd, object, &elf.runpaths, members, reader.budget)?;
    let mut inherited = if elf.runpaths.is_empty() {
        rpaths
    } else {
        Vec::new()
    };
    for path in inherited_rpaths {
        reader.budget.path(path)?;
    }
    inherited.extend_from_slice(inherited_rpaths);
    let search = if elf.runpaths.is_empty() {
        for path in &inherited {
            reader.budget.path(path)?;
        }
        inherited.clone()
    } else {
        runpaths
    };
    // Retain only owned requirements while descending, not each ancestor's file buffer.
    drop(elf);
    for library in &requirements.libraries {
        let mut resolved = None;
        for directory in &search {
            reader.budget.path(directory)?;
            reader.budget.text(library)?;
            let candidate = directory.join(library);
            match root.file_type(&candidate, false) {
                Ok(_) => {
                    let canonical =
                        resolve_artifact_path(root, &candidate, members, reader.budget)?;
                    resolved = Some((candidate, canonical));
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(fs_error(error)),
            }
        }
        if let Some((loader_path, canonical)) = resolved {
            if let Some(required) = version_needs.get(library) {
                let provided = reader.with_file(&canonical, |input, budget| {
                    let provider = parse_loader_elf(input, budget)?;
                    version_definitions(&provider, input, budget)
                })?;
                verify_versions(library, required, &provided)?;
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
