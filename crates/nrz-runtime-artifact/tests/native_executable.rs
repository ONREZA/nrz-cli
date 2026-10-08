use nrz_runtime_artifact::verify_native_executable;

fn elf(machine: u16, interpreter: Option<&str>) -> Vec<u8> {
    let mut bytes = vec![0u8; 512];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(if interpreter.is_some() { 2u16 } else { 1u16 }).to_le_bytes());
    bytes[64..68].copy_from_slice(&1u32.to_le_bytes());
    bytes[68..72].copy_from_slice(&5u32.to_le_bytes());
    bytes[80..88].copy_from_slice(&0x400000u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&512u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&512u64.to_le_bytes());
    if let Some(interpreter) = interpreter {
        bytes[120..124].copy_from_slice(&3u32.to_le_bytes());
        bytes[128..136].copy_from_slice(&192u64.to_le_bytes());
        bytes[152..160].copy_from_slice(&((interpreter.len() + 1) as u64).to_le_bytes());
        bytes[192..192 + interpreter.len()].copy_from_slice(interpreter.as_bytes());
    }
    bytes
}

#[test]
fn native_entry_rejects_wrong_machine_interpreter_and_format() {
    assert!(verify_native_executable(&elf(62, None)).is_ok());
    let glibc = verify_native_executable(&elf(62, Some("/lib64/ld-linux-x86-64.so.2"))).unwrap();
    assert_eq!(
        glibc.interpreter.as_deref(),
        Some("/lib64/ld-linux-x86-64.so.2")
    );
    for bytes in [
        elf(183, None),
        elf(62, Some("/lib/ld-musl-x86_64.so.1")),
        b"#!/bin/sh\n".to_vec(),
        elf(62, Some("/bin/sh")),
    ] {
        assert!(verify_native_executable(&bytes).is_err());
    }
    let mut missing_segment = elf(62, None);
    missing_segment[68..72].fill(0);
    assert!(verify_native_executable(&missing_segment).is_err());
    let mut shared_library = elf(62, None);
    shared_library[24..32].fill(0);
    assert!(verify_native_executable(&shared_library).is_err());
}
