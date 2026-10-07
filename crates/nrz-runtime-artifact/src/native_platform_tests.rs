use super::{has_native_payload_header, verify_linux_x86_64_native_platform};
use goblin::elf::header;
use std::io::{Cursor, Read};

fn elf_header(kind: u16) -> Vec<u8> {
    let mut bytes = vec![0; 64];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&kind.to_le_bytes());
    bytes[18..20].copy_from_slice(&header::EM_X86_64.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

// Entry-only admission rejected genuine SO entry=0 and linker ET_REL assets.
#[test]
fn linux_native_platform_accepts_shared_objects_and_link_time_assets() {
    for kind in [header::ET_DYN, header::ET_REL, header::ET_EXEC] {
        let bytes = elf_header(kind);
        verify_linux_x86_64_native_platform(&mut Cursor::new(&bytes)).unwrap();
        let mut linux = bytes;
        linux[header::EI_OSABI] = header::ELFOSABI_LINUX;
        verify_linux_x86_64_native_platform(&mut Cursor::new(&linux)).unwrap();
    }
    for bytes in [
        b"!<arch>\n".as_slice(),
        b"python/data/resource".as_slice(),
        b"",
        b"M",
        b"\x7f",
    ] {
        verify_linux_x86_64_native_platform(&mut Cursor::new(bytes)).unwrap();
    }
}

// The installer state cannot attest these actual header-platform conflicts.
#[test]
fn linux_native_platform_rejects_foreign_and_malformed_elf_headers() {
    let base = elf_header(header::ET_DYN);
    for (offset, value) in [
        (header::EI_CLASS, header::ELFCLASS32),
        (header::EI_DATA, header::ELFDATA2MSB),
        (header::EI_OSABI, header::ELFOSABI_FREEBSD),
    ] {
        let mut bytes = base.clone();
        bytes[offset] = value;
        assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(&bytes)).is_err());
    }
    for machine in [header::EM_AARCH64, header::EM_386, header::EM_ARM] {
        let mut bytes = base.clone();
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(&bytes)).is_err());
    }
    for kind in [header::ET_NONE, header::ET_CORE] {
        assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(elf_header(kind))).is_err());
    }
    // The classifier recognizes an ELF32 header even though the target guard
    // refuses its architecture/class; it must not demand an ELF64-sized buffer.
    let mut elf32 = base[..52].to_vec();
    elf32[header::EI_CLASS] = header::ELFCLASS32;
    assert!(has_native_payload_header(&mut Cursor::new(&elf32)).unwrap());
    assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(elf32)).is_err());
    for len in 4..64 {
        assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(&base[..len])).is_err());
    }
}

#[test]
fn linux_native_platform_requires_pe_and_fat_header_witnesses() {
    // A real Python identifier and Java bytecode are not foreign native formats.
    for bytes in [
        b"MZ = 42\nprint(MZ)\n".as_slice(),
        b"\xca\xfe\xba\xbe\0\0\0\x3d\0\x1b\x0a\0\x02\0\x03\x07".as_slice(),
    ] {
        let mut resource = bytes.to_vec();
        resource.resize(64, 0);
        assert!(!has_native_payload_header(&mut Cursor::new(&resource)).unwrap());
        verify_linux_x86_64_native_platform(&mut Cursor::new(resource)).unwrap();
    }
    // PE signature location is declared, not limited to a fixed header window.
    for offset in [48_usize, 64, 0x78, 8192] {
        let mut bytes = vec![0; (offset + 4).max(64)];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&(offset as u32).to_le_bytes());
        bytes[offset..offset + 4].copy_from_slice(b"PE\0\0");
        assert!(has_native_payload_header(&mut Cursor::new(&bytes)).unwrap());
        assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(&bytes)).is_err());
    }
    for (magic, little_endian) in [
        (0xcafebabe_u32, false),
        (0xbebafeca, true),
        (0xcafebabf, false),
        (0xbfbafeca, true),
    ] {
        let mut bytes = vec![0; 64];
        bytes[..4].copy_from_slice(&magic.to_be_bytes());
        for (offset, value) in [(4, 1_u32), (8, 0x01000007)] {
            bytes[offset..offset + 4].copy_from_slice(&if little_endian {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            });
        }
        assert!(has_native_payload_header(&mut Cursor::new(&bytes)).unwrap());
        assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(bytes)).is_err());
    }
    for bytes in [
        b"\xfe\xed\xfa\xce".as_slice(),
        b"\xce\xfa\xed\xfe".as_slice(),
        b"\xfe\xed\xfa\xcf".as_slice(),
        b"\xcf\xfa\xed\xfe".as_slice(),
    ] {
        assert!(verify_linux_x86_64_native_platform(&mut Cursor::new(bytes)).is_err());
    }
}

#[test]
fn native_header_inspection_propagates_io_failures() {
    struct Failing(Cursor<Vec<u8>>);
    impl Read for Failing {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if self.0.position() as usize == self.0.get_ref().len() {
                Err(std::io::Error::other("read failure"))
            } else {
                self.0.read(buffer)
            }
        }
    }
    let mut distant_pe = vec![0; 64];
    distant_pe[..2].copy_from_slice(b"MZ");
    for pointer in [64_u32, 8192] {
        distant_pe[60..64].copy_from_slice(&pointer.to_le_bytes());
        assert!(has_native_payload_header(&mut Failing(Cursor::new(distant_pe.clone()))).is_err());
        assert!(
            verify_linux_x86_64_native_platform(&mut Failing(Cursor::new(distant_pe.clone())))
                .is_err()
        );
    }
    assert!(has_native_payload_header(&mut Failing(Cursor::new(vec![]))).is_err());
    assert!(verify_linux_x86_64_native_platform(&mut Failing(Cursor::new(vec![]))).is_err());
}
