//! Native entry admission is independent of managed interpreter authorization.
use goblin::elf::{Elf, header, program_header};

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
    if elf.header.e_ident[header::EI_CLASS] != header::ELFCLASS64
        || elf.header.e_ident[header::EI_DATA] != header::ELFDATA2LSB
        || elf.header.e_machine != header::EM_X86_64
        || !matches!(elf.header.e_type, header::ET_EXEC | header::ET_DYN)
        || elf.entry == 0
        || !matches!(
            elf.header.e_ident[header::EI_OSABI],
            header::ELFOSABI_NONE | header::ELFOSABI_LINUX
        )
    {
        return invariant("native entry requires a runnable Linux x86_64 ELF executable");
    }
    if !elf.program_headers.iter().any(|segment| {
        segment.p_type == program_header::PT_LOAD
            && segment.p_flags & program_header::PF_X != 0
            && elf.entry >= segment.p_vaddr
            && segment
                .p_vaddr
                .checked_add(segment.p_filesz)
                .is_some_and(|end| elf.entry < end)
            && segment
                .p_offset
                .checked_add(segment.p_filesz)
                .is_some_and(|end| end <= bytes.len() as u64)
    }) {
        return invariant("native entry is outside an executable file-backed ELF segment");
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
                || name.contains(['/', '\\', '\0'])
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
            .any(|path| path.is_empty() || path.len() > 4096 || path.contains('\0'))
    {
        return invariant("native entry has invalid dynamic library paths");
    }
    Ok(NativeExecutableRequirements {
        interpreter: elf.interpreter.map(str::to_owned),
        libraries: elf.libraries.into_iter().map(str::to_owned).collect(),
        library_paths,
    })
}
