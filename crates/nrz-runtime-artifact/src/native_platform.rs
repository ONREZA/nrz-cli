//! Header-only rejection of known native platform conflicts.
use std::io::Read;

use crate::{RuntimeArtifactError, invariant};
use goblin::{
    elf::{Elf, header},
    mach::{constants::cputype, fat},
};

enum NativeHeader {
    Elf(header::Header),
    Foreign,
    Archive,
    Other,
}

/// Recognize native payload headers without treating plain MZ text or Java
/// bytecode's CAFEBABE magic as native code. Unix archives count as link-time
/// native assets. Buffers a bounded header; streams to any declared PE signature
/// offset (e_lfanew) without buffering the intervening bytes.
/// Read from the current header position; the reader may be advanced beyond it.
pub fn has_native_payload_header<R: Read>(reader: &mut R) -> Result<bool, RuntimeArtifactError> {
    Ok(!matches!(inspect_header(reader)?, NativeHeader::Other))
}

/// Reject known native format or architecture conflicts with Linux x86_64.
///
/// Admits ELF executables, shared libraries (including entry=0), and relocatable
/// objects without executable-entry or DSO loader rules. Plain resources and Unix
/// archive containers are allowed. Success is not ABI, importability, library
/// closure, or archive-member compatibility proof. No complete file is buffered.
/// Read from the current header position; the reader may be advanced beyond it.
pub fn verify_linux_x86_64_native_platform<R: Read>(
    reader: &mut R,
) -> Result<(), RuntimeArtifactError> {
    match inspect_header(reader)? {
        NativeHeader::Elf(elf) => {
            if elf.e_ident[header::EI_CLASS] != header::ELFCLASS64
                || elf.e_ident[header::EI_DATA] != header::ELFDATA2LSB
                || elf.e_machine != header::EM_X86_64
                || !matches!(
                    elf.e_ident[header::EI_OSABI],
                    header::ELFOSABI_NONE | header::ELFOSABI_LINUX
                )
                || !matches!(
                    elf.e_type,
                    header::ET_REL | header::ET_EXEC | header::ET_DYN
                )
            {
                return invariant("native platform requires Linux x86_64 little-endian ELF");
            }
        }
        NativeHeader::Foreign => {
            return invariant("native platform rejects Mach-O and PE payloads for Linux x86_64");
        }
        NativeHeader::Archive | NativeHeader::Other => {}
    }
    Ok(())
}

fn inspect_header<R: Read>(reader: &mut R) -> Result<NativeHeader, RuntimeArtifactError> {
    let mut prefix = Vec::with_capacity(64);
    reader.take(64).read_to_end(&mut prefix).map_err(io_error)?;
    if prefix.starts_with(b"\x7fELF") {
        if prefix.len() < header::SIZEOF_IDENT {
            return invariant("native platform requires a complete ELF header");
        }
        return Elf::parse_header(&prefix)
            .map(NativeHeader::Elf)
            .map_err(|error| {
                RuntimeArtifactError::Invariant(format!(
                    "invalid native platform ELF header: {error}"
                ))
            });
    }
    if prefix.starts_with(b"!<arch>\n") {
        return Ok(NativeHeader::Archive);
    }
    if prefix.starts_with(b"MZ") && prefix.len() >= 64 {
        // Only the DOS signature and e_lfanew locate a PE header. The unused
        // legacy DOS fields cannot distinguish a native payload from text.
        let offset = goblin::pe::header::PE_POINTER_OFFSET as usize;
        let pointer = u32::from_le_bytes(prefix[offset..offset + 4].try_into().unwrap());
        let mut signature = Vec::with_capacity(4);
        let pointer = pointer as usize;
        if let Some(remaining) = prefix.get(pointer..) {
            signature.extend_from_slice(&remaining[..remaining.len().min(4)]);
        } else {
            // Advance to the declared signature without buffering an arbitrary
            // DOS stub. A truncated resource does not supply a PE witness.
            let gap = (pointer - prefix.len()) as u64;
            if std::io::copy(&mut reader.take(gap), &mut std::io::sink()).map_err(io_error)? != gap
            {
                return Ok(NativeHeader::Other);
            }
        }
        reader
            .take((4 - signature.len()) as u64)
            .read_to_end(&mut signature)
            .map_err(io_error)?;
        if signature == b"PE\0\0" {
            return Ok(NativeHeader::Foreign);
        }
    }
    if prefix.len() >= 4 {
        let magic = u32::from_be_bytes(prefix[..4].try_into().unwrap());
        if matches!(magic, 0xfeedface | 0xcefaedfe | 0xfeedfacf | 0xcffaedfe) {
            return Ok(NativeHeader::Foreign);
        }
        if matches!(magic, 0xcafebabe | 0xbebafeca | 0xcafebabf | 0xbfbafeca)
            && is_macho_fat_header(&prefix, matches!(magic, 0xbebafeca | 0xbfbafeca))
        {
            return Ok(NativeHeader::Foreign);
        }
    }
    Ok(NativeHeader::Other)
}

fn is_macho_fat_header(prefix: &[u8], little_endian: bool) -> bool {
    // FAT32 and FAT64 share the count and first CPU fields. Require an actual
    // architecture witness: CAFEBABE alone also names Java class resources.
    if prefix.len() < fat::SIZEOF_FAT_HEADER + fat::SIZEOF_FAT_ARCH {
        return false;
    }
    let mut normalized = prefix[..fat::SIZEOF_FAT_HEADER + fat::SIZEOF_FAT_ARCH].to_vec();
    if little_endian {
        for field in normalized.as_chunks_mut::<4>().0 {
            field.reverse();
        }
    }
    match (
        fat::FatHeader::parse(&normalized),
        fat::FatArch::parse(&normalized, fat::SIZEOF_FAT_HEADER),
    ) {
        (Ok(header), Ok(arch)) => {
            header.nfat_arch > 0
                && matches!(
                    arch.cputype,
                    cputype::CPU_TYPE_VAX
                        | cputype::CPU_TYPE_MC680X0
                        | cputype::CPU_TYPE_X86
                        | cputype::CPU_TYPE_X86_64
                        | cputype::CPU_TYPE_MIPS
                        | cputype::CPU_TYPE_MC98000
                        | cputype::CPU_TYPE_HPPA
                        | cputype::CPU_TYPE_ARM
                        | cputype::CPU_TYPE_ARM64
                        | cputype::CPU_TYPE_ARM64_32
                        | cputype::CPU_TYPE_MC88000
                        | cputype::CPU_TYPE_SPARC
                        | cputype::CPU_TYPE_I860
                        | cputype::CPU_TYPE_ALPHA
                        | cputype::CPU_TYPE_POWERPC
                        | cputype::CPU_TYPE_POWERPC64
                )
        }
        _ => false,
    }
}

fn io_error(error: std::io::Error) -> RuntimeArtifactError {
    RuntimeArtifactError::Invariant(format!("cannot inspect native platform header: {error}"))
}

#[cfg(test)]
#[path = "native_platform_tests.rs"]
mod tests;
