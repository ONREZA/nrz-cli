use nrz_runtime_artifact::verify_native_executable;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct MeasuredAllocator;
thread_local! { static ALLOCATED: Cell<Option<usize>> = const { Cell::new(None) }; }
fn allocated(bytes: usize) {
    let _ = ALLOCATED.try_with(|value| {
        if let Some(previous) = value.get() {
            value.set(Some(previous + bytes));
        }
    });
}
// This integration binary measures actual requested heap bytes on its current
// thread, excluding fixture construction and concurrent test threads.
unsafe impl GlobalAlloc for MeasuredAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size());
        // SAFETY: delegate the unchanged allocator contract to System.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size());
        // SAFETY: delegate the unchanged allocator contract to System.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: every allocation in this binary is delegated to System.
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, bytes: usize) -> *mut u8 {
        allocated(bytes);
        // SAFETY: delegate the unchanged allocator contract to System.
        unsafe { System.realloc(pointer, layout, bytes) }
    }
}
#[global_allocator]
static ALLOCATOR: MeasuredAllocator = MeasuredAllocator;

fn measure<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    ALLOCATED.with(|value| value.set(Some(0)));
    let result = operation();
    let bytes = ALLOCATED.with(|value| value.replace(None).unwrap());
    (result, bytes)
}

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

#[test]
fn loader_metadata_does_not_index_unused_or_zero_padded_string_tables() {
    let strings = 2 * 1024 * 1024;
    let sections = 1024 + strings;
    let mut bytes = elf(62, None);
    bytes.resize(sections + 3 * 64, 0);
    let file_bytes = bytes.len() as u64;
    bytes[96..104].copy_from_slice(&file_bytes.to_le_bytes());
    bytes[104..112].copy_from_slice(&file_bytes.to_le_bytes());
    bytes[56..58].copy_from_slice(&2_u16.to_le_bytes());
    bytes[120..124].copy_from_slice(&goblin::elf::program_header::PT_DYNAMIC.to_le_bytes());
    bytes[128..136].copy_from_slice(&512_u64.to_le_bytes());
    // The first DT_NULL terminates the table, even when its segment has large padding.
    bytes[152..160].copy_from_slice(&(strings as u64).to_le_bytes());
    for (index, (tag, value)) in [
        (goblin::elf::dynamic::DT_STRTAB, 0x400400),
        (goblin::elf::dynamic::DT_STRSZ, strings as u64),
        (goblin::elf::dynamic::DT_NULL, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let offset = 512 + index * 16;
        bytes[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
        bytes[offset + 8..offset + 16].copy_from_slice(&value.to_le_bytes());
    }
    bytes[40..48].copy_from_slice(&(sections as u64).to_le_bytes());
    bytes[58..60].copy_from_slice(&64_u16.to_le_bytes());
    bytes[60..62].copy_from_slice(&3_u16.to_le_bytes());
    bytes[62..64].copy_from_slice(&1_u16.to_le_bytes());
    let string_header = sections + 64;
    bytes[string_header + 4..string_header + 8]
        .copy_from_slice(&goblin::elf::section_header::SHT_STRTAB.to_le_bytes());
    bytes[string_header + 24..string_header + 32].copy_from_slice(&1024_u64.to_le_bytes());
    bytes[string_header + 32..string_header + 40].copy_from_slice(&(strings as u64).to_le_bytes());
    let symbol_header = sections + 128;
    bytes[symbol_header + 4..symbol_header + 8]
        .copy_from_slice(&goblin::elf::section_header::SHT_SYMTAB.to_le_bytes());
    bytes[symbol_header + 40..symbol_header + 44].copy_from_slice(&1_u32.to_le_bytes());
    let (baseline, eager_bytes) = measure(|| goblin::elf::Elf::parse(&bytes));
    drop(baseline.unwrap());
    let (result, loader_bytes) = measure(|| verify_native_executable(&bytes));
    result.unwrap();
    assert!(
        eager_bytes > strings * 24,
        "baseline must reproduce actual parser expansion"
    );
    assert!(
        loader_bytes < 64 * 1024,
        "loader metadata allocated {loader_bytes} bytes for a {strings}-byte table"
    );
    eprintln!("actual cumulative allocations: eager={eager_bytes}, loader={loader_bytes}");
}
