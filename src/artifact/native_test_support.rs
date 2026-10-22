// File-backed ELF fixtures exercise the SDK closure verifier without a host compiler.
pub(super) fn native_elf_fixture(runpath: Option<&str>) -> Vec<u8> {
    let mut bytes = vec![0u8; 1024];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&(if runpath.is_some() { 2u16 } else { 3u16 }).to_le_bytes());
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    if runpath.is_some() {
        bytes[24..32].copy_from_slice(&0x400180u64.to_le_bytes());
    }
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(if runpath.is_some() { 3u16 } else { 1u16 }).to_le_bytes());
    bytes[64..68].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
    bytes[68..72].copy_from_slice(&5u32.to_le_bytes());
    bytes[80..88].copy_from_slice(&0x400000u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&1024u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&1024u64.to_le_bytes());
    if let Some(runpath) = runpath {
        bytes[120..124].copy_from_slice(&2u32.to_le_bytes()); // PT_DYNAMIC
        bytes[128..136].copy_from_slice(&256u64.to_le_bytes());
        bytes[136..144].copy_from_slice(&0x400100u64.to_le_bytes());
        bytes[152..160].copy_from_slice(&80u64.to_le_bytes());
        bytes[160..168].copy_from_slice(&80u64.to_le_bytes());
        for (index, (tag, value)) in [(5u64, 0x400200u64), (10, 128), (1, 1), (29, 32)]
            .iter()
            .enumerate()
        {
            let offset = 256 + index * 16;
            bytes[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
            bytes[offset + 8..offset + 16].copy_from_slice(&value.to_le_bytes());
        }
        bytes[513..522].copy_from_slice(b"libfoo.so");
        bytes[544..544 + runpath.len()].copy_from_slice(runpath.as_bytes());
        let interpreter = b"/lib64/ld-linux-x86-64.so.2\0";
        bytes[176..180].copy_from_slice(&3u32.to_le_bytes()); // PT_INTERP
        bytes[184..192].copy_from_slice(&640u64.to_le_bytes());
        bytes[192..200].copy_from_slice(&0x400280u64.to_le_bytes());
        bytes[208..216].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[216..224].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[640..640 + interpreter.len()].copy_from_slice(interpreter);
    }
    bytes
}
