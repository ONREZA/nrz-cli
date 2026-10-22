//! Manual requested-heap qualification; not discovered by cargo test or release gates.
//! cargo run -p nrz-source-bundle --example qualify_source_bundle_v2_admission
//! Add `--byte-ceiling` for the separate 512MiB wire-boundary run (2GiB lab pool).
//! Fixtures are caller-owned outside the interval; guarded input is allocated inside.
//! This checks the pinned allocator request envelope, not RSS/cgroup headroom.

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering::Relaxed};

use nrz_contract::artifact_json_v1;
use nrz_source_bundle::PathWorkBudget;
use nrz_source_bundle::source_bundle_v2::{
    SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES, SourceBundleV2ManifestBudget, SourceBundleV2ManifestError,
    SourceBundleV2ManifestSummary,
    admission::{AdmissionError, ManifestMemoryPool},
    decode_source_bundle_v2_manifest,
};
use serde_json::{Value, json};

struct Meter;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static BASELINE: AtomicUsize = AtomicUsize::new(0);
static RESERVED_BASELINE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COPY_PEAK: AtomicUsize = AtomicUsize::new(0);
static DEFICIT: AtomicUsize = AtomicUsize::new(0);
static RESERVED_PEAK: AtomicUsize = AtomicUsize::new(0);
static POOL: AtomicPtr<ManifestMemoryPool> = AtomicPtr::new(ptr::null_mut());
static STRICT_DROP: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

fn observe(live: usize, copying: bool) {
    let pool = POOL.load(Relaxed);
    if pool.is_null() {
        return;
    }
    // The single-threaded qualification owns this pool until after disarming.
    let reserved = (unsafe { &*pool }.reserved_bytes() as usize)
        .saturating_sub(RESERVED_BASELINE.load(Relaxed));
    let additional = live.saturating_sub(BASELINE.load(Relaxed));
    if copying {
        COPY_PEAK.fetch_max(additional, Relaxed);
    } else {
        PEAK.fetch_max(additional, Relaxed);
        COPY_PEAK.fetch_max(additional, Relaxed);
    }
    DEFICIT.fetch_max(additional.saturating_sub(reserved), Relaxed);
    RESERVED_PEAK.fetch_max(reserved, Relaxed);
}

unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        let result = unsafe { System.alloc(layout) };
        if !result.is_null() {
            observe(
                LIVE.fetch_add(layout.size(), Relaxed) + layout.size(),
                false,
            );
        }
        result
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if STRICT_DROP.load(Relaxed) {
            observe(LIVE.load(Relaxed), false);
        }
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        // Charge both blocks even if libc can grow in place. No allocation/logging
        // is allowed here: the atomics record failures for assertions after disarm.
        observe(LIVE.load(Relaxed).saturating_add(new_size), true);
        let result = unsafe { System.realloc(pointer, old, new_size) };
        if !result.is_null() {
            LIVE.fetch_sub(old.size(), Relaxed);
            observe(LIVE.fetch_add(new_size, Relaxed) + new_size, false);
        }
        result
    }
}

#[global_allocator]
static ALLOCATOR: Meter = Meter;

#[derive(Clone, Copy, Debug)]
enum Expected {
    Accepted(u64),
    Invalid,
    FileLimit,
    WorkLimit,
    MemoryLimit,
}

fn file(path: String) -> Value {
    json!({"path":path,"role":"static","entryType":"file",
        "blobDigest":{"algorithm":"BLAKE3","value":"0".repeat(64)},
        "size":0,"executable":false})
}

fn manifest(files: Vec<Value>) -> Value {
    json!({"schemaVersion":"SOURCE_BUNDLE_V2.0","capabilities":[],"layers":[],
        "routes":[],"entrypoints":[],"files":files})
}

fn canonical(value: Value) -> Vec<u8> {
    artifact_json_v1::encode(
        &value,
        artifact_json_v1::Limits {
            max_bytes: SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES,
            max_depth: artifact_json_v1::MAX_SUPPORTED_DEPTH,
        },
    )
    .unwrap()
}

fn qualify(name: &str, bytes: Vec<u8>, expected: Expected) {
    let capacity = if matches!(expected, Expected::MemoryLimit) {
        bytes.len() as u64
    } else {
        512 * 1024 * 1024
    };
    qualify_bytes(name, &bytes, expected, capacity);
}

fn qualify_bytes(name: &str, bytes: &[u8], expected: Expected, capacity: u64) {
    let expected_digest = if matches!(expected, Expected::Accepted(_)) {
        Some(blake3::hash(bytes))
    } else {
        None
    };
    let pool = ManifestMemoryPool::new(capacity);
    let baseline = LIVE.load(Relaxed);
    BASELINE.store(baseline, Relaxed);
    RESERVED_BASELINE.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    COPY_PEAK.store(0, Relaxed);
    DEFICIT.store(0, Relaxed);
    RESERVED_PEAK.store(0, Relaxed);
    POOL.store(ptr::from_ref(&pool).cast_mut(), Relaxed);
    let result = pool.reserve_input(bytes.len()).and_then(|mut input| {
        input.append(bytes)?;
        input.verify(
            SourceBundleV2ManifestBudget {
                max_file_count: if matches!(expected, Expected::FileLimit) {
                    0
                } else {
                    100_000
                },
                max_logical_bytes: 0,
                max_static_file_bytes: 0,
            },
            if matches!(expected, Expected::WorkLimit) {
                0
            } else {
                u64::MAX
            },
        )
    });
    let retained = LIVE.load(Relaxed).saturating_sub(baseline);
    let retained_reservation = pool.reserved_bytes();
    let correct = match (&result, expected) {
        (Ok(admitted), Expected::Accepted(file_count)) => {
            admitted.reserved_bytes() == retained_reservation
                && retained as u64 <= retained_reservation
                && admitted.verified().summary()
                    == SourceBundleV2ManifestSummary {
                        file_count,
                        logical_static_bytes: 0,
                        artifact_size_bytes: 0,
                        max_static_file_size_bytes: 0,
                    }
                && admitted.verified().manifest_size_bytes() == bytes.len() as u64
                && admitted.verified().manifest_blake3()
                    == expected_digest.unwrap().to_hex().as_str()
        }
        (Err(AdmissionError::InvalidManifest(message)), Expected::Invalid) => {
            retained_reservation == 0 && message.len() <= 512 && retained <= 512
        }
        (Err(AdmissionError::FileCountLimit), Expected::FileLimit)
        | (Err(AdmissionError::PathWorkLimit), Expected::WorkLimit)
        | (Err(AdmissionError::MemoryUnavailable { .. }), Expected::MemoryLimit) => {
            retained_reservation == 0 && retained == 0
        }
        _ => false,
    };
    drop(result);
    let released = pool.reserved_bytes() == 0 && LIVE.load(Relaxed) == baseline;
    POOL.store(ptr::null_mut(), Relaxed);
    assert!(correct, "{name}: unexpected outcome for {expected:?}");
    assert!(
        released,
        "{name}: result/input did not release allocation and capacity"
    );
    assert_eq!(
        DEFICIT.load(Relaxed),
        0,
        "{name}: allocation exceeded its phase reservation"
    );
    println!(
        "{name}: input={} requestedPeak={} copyPeak={} reservedPeak={} retained={} retainedReservation={} outcome={expected:?}",
        bytes.len(),
        PEAK.load(Relaxed),
        COPY_PEAK.load(Relaxed),
        RESERVED_PEAK.load(Relaxed),
        retained,
        retained_reservation
    );
    if let Some(digest) = expected_digest {
        println!("{name}: manifestBlake3={digest}");
    }
}

fn qualify_projection(name: &str, bytes: Vec<u8>) {
    let pool = ManifestMemoryPool::new(512 * 1024 * 1024);
    // This caller-owned holder is outside the measured metadata allocations.
    let mut old = Vec::with_capacity(3);
    let baseline = LIVE.load(Relaxed);
    BASELINE.store(baseline, Relaxed);
    RESERVED_BASELINE.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    COPY_PEAK.store(0, Relaxed);
    DEFICIT.store(0, Relaxed);
    RESERVED_PEAK.store(0, Relaxed);
    STRICT_DROP.store(true, Relaxed);
    POOL.store(ptr::from_ref(&pool).cast_mut(), Relaxed);
    let mut body = pool.reserve_input(bytes.len()).unwrap();
    body.append(&bytes).unwrap();
    let admitted = body
        .verify(
            SourceBundleV2ManifestBudget {
                max_file_count: 100_000,
                max_logical_bytes: 0,
                max_static_file_bytes: 0,
            },
            u64::MAX,
        )
        .unwrap();
    for _ in 0..3 {
        // Existing manifest/old-generation slack cannot cover this preparation's allocations.
        BASELINE.store(LIVE.load(Relaxed), Relaxed);
        RESERVED_BASELINE.store(pool.reserved_bytes() as usize, Relaxed);
        old.push(admitted.prepare_static_projection(u64::MAX).unwrap());
        observe(LIVE.load(Relaxed), false);
    }
    let index_reservations = old.iter().map(|index| index.reserved_bytes()).sum::<u64>();
    assert_eq!(
        pool.reserved_bytes(),
        admitted.reserved_bytes() + index_reservations
    );
    drop(admitted);
    assert_eq!(pool.reserved_bytes(), index_reservations);
    BASELINE.store(baseline, Relaxed);
    RESERVED_BASELINE.store(0, Relaxed);
    observe(LIVE.load(Relaxed), false);
    let before_lookup = ALLOCATIONS.load(Relaxed);
    for index in &old {
        assert!(
            index
                .lookup("a", &mut PathWorkBudget::new(100_000))
                .unwrap()
                .is_some()
        );
        for path in ["a", "missing/deep/suffix", "a/../bad", "a//bad", ""] {
            let _ = index.lookup(path, &mut PathWorkBudget::new(100_000));
            let _ = index.lookup(path, &mut PathWorkBudget::new(0));
        }
    }
    let lookup_allocations = ALLOCATIONS.load(Relaxed) - before_lookup;
    old.clear();
    let released = pool.reserved_bytes() == 0 && LIVE.load(Relaxed) == baseline;
    POOL.store(ptr::null_mut(), Relaxed);
    STRICT_DROP.store(false, Relaxed);
    assert!(
        released,
        "{name}: projection did not release data and permits"
    );
    assert_eq!(
        DEFICIT.load(Relaxed),
        0,
        "{name}: allocation/drop exceeded its phase allowance"
    );
    assert_eq!(
        lookup_allocations, 0,
        "{name}: lookup allocated on hit/miss/invalid/refusal"
    );
    println!(
        "{name}: requestedPeak={} copyPeak={} reservedPeak={} oldIndices=3 retainedReservation={index_reservations} lookupAllocations=0",
        PEAK.load(Relaxed),
        COPY_PEAK.load(Relaxed),
        RESERVED_PEAK.load(Relaxed)
    );
}

fn byte_ceiling_file(index: usize) -> Vec<u8> {
    let mut entry = file(format!("shared/assets/f{index:04}"));
    entry["contentType"] = Value::String(String::new());
    canonical(entry)
}

fn qualify_byte_ceiling() {
    const FILES: usize = 4096;
    const LAB_POOL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
    const PREFIX: &[u8] = br#"{"capabilities":[],"entrypoints":[],"files":["#;
    const SUFFIX: &[u8] = br#"],"layers":[],"routes":[],"schemaVersion":"SOURCE_BUNDLE_V2.0"}"#;
    const EMPTY_CONTENT: &[u8] = b"\"contentType\":\"\"";
    let template = byte_ceiling_file(0);
    let split = template
        .windows(EMPTY_CONTENT.len())
        .position(|window| window == EMPTY_CONTENT)
        .unwrap()
        + EMPTY_CONTENT.len()
        - 1;
    let overhead = PREFIX.len() + SUFFIX.len() + FILES * template.len() + FILES - 1;
    let payload = SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES
        .checked_sub(overhead)
        .unwrap();
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES + 1)
        .unwrap();
    let capacity = bytes.capacity();
    bytes.extend_from_slice(PREFIX);
    for index in 0..FILES {
        if index != 0 {
            bytes.push(b',');
        }
        let entry = byte_ceiling_file(index);
        assert_eq!(entry.len(), template.len());
        bytes.extend_from_slice(&entry[..split]);
        let content_bytes = payload / FILES + usize::from(index < payload % FILES);
        bytes.resize(bytes.len() + content_bytes, b'x');
        bytes.extend_from_slice(&entry[split..]);
    }
    bytes.extend_from_slice(SUFFIX);
    assert_eq!(bytes.len(), SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES);
    assert_eq!(
        bytes.capacity(),
        capacity,
        "fixture construction reallocated"
    );
    println!(
        "byte-ceiling fixture: input={} capacity={} valueSize={} expectedFiles={FILES} labPoolBytes={LAB_POOL_BYTES}",
        bytes.len(),
        bytes.capacity(),
        std::mem::size_of::<Value>()
    );
    STRICT_DROP.store(true, Relaxed);
    qualify_bytes(
        "byte-ceiling",
        &bytes,
        Expected::Accepted(FILES as u64),
        LAB_POOL_BYTES,
    );
    STRICT_DROP.store(false, Relaxed);

    let pool = ManifestMemoryPool::new(LAB_POOL_BYTES);
    let before = ALLOCATIONS.load(Relaxed);
    let refused = pool.reserve_input(SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES + 1);
    assert_eq!(
        ALLOCATIONS.load(Relaxed),
        before,
        "oversized admission allocated"
    );
    assert!(matches!(refused, Err(AdmissionError::ManifestByteLimit)));
    assert_eq!(pool.reserved_bytes(), 0);
    bytes.push(b'\n');
    assert_eq!(bytes.capacity(), capacity, "MAX+1 fixture reallocated");
    let before = ALLOCATIONS.load(Relaxed);
    let decoded = decode_source_bundle_v2_manifest(
        &bytes,
        SourceBundleV2ManifestBudget {
            max_file_count: FILES as u64,
            max_logical_bytes: 0,
            max_static_file_bytes: 0,
        },
    );
    assert_eq!(
        ALLOCATIONS.load(Relaxed),
        before,
        "oversized decoder allocated"
    );
    assert!(matches!(
        decoded,
        Err(SourceBundleV2ManifestError::Canonical(
            artifact_json_v1::Error::ByteLimit
        ))
    ));
    println!(
        "byte-ceiling: files={FILES} exactBytes={} reserveMaxPlusOne=ManifestByteLimit decodeMaxPlusOne=ByteLimit",
        SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES
    );
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("--byte-ceiling") => {
            assert!(
                arguments.next().is_none(),
                "usage: qualify_source_bundle_v2_admission [--byte-ceiling]"
            );
            qualify_byte_ceiling();
            return;
        }
        None => {}
        Some(_) => panic!("usage: qualify_source_bundle_v2_admission [--byte-ceiling]"),
    }
    let broad = || manifest((0..10_000).map(|i| file(format!("f{i:08}"))).collect());
    qualify("broad-10k", canonical(broad()), Expected::Accepted(10_000));
    let shared = format!("{}/", vec!["d"; 100].join("/"));
    qualify(
        "shared-10k",
        canonical(manifest(
            (0..10_000)
                .map(|i| file(format!("{shared}f{i:08}")))
                .collect(),
        )),
        Expected::Accepted(10_000),
    );
    qualify(
        "deep-100k",
        canonical(manifest(vec![file(vec!["d"; 100_000].join("/"))])),
        Expected::Accepted(1),
    );
    let mut content = manifest(vec![file("a".into())]);
    content["files"][0]["contentType"] = Value::String("x".repeat(32 * 1024 * 1024));
    qualify("content-32MiB", canonical(content), Expected::Accepted(1));

    let mut fan_in = (0..500).map(|i| json!({"path":format!("a{i:08}"),"role":"static","entryType":"symlink","linkTarget":"m","size":0})).collect::<Vec<_>>();
    fan_in.push(json!({"path":"m","role":"static","entryType":"symlink","linkTarget":format!("{shared}z"),"size":0}));
    fan_in.push(file(format!("{shared}z")));
    fan_in.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    qualify(
        "fan-in",
        canonical(manifest(fan_in)),
        Expected::Accepted(502),
    );

    let mut metadata = manifest(Vec::new());
    metadata["layers"] =
        json!([{"name":"s","target":"STATIC","rootPath":"d".repeat(2 * 1024 * 1024)}]);
    qualify("metadata-2MiB", canonical(metadata), Expected::Accepted(0));
    let mut escaped = manifest(vec![file("a".into())]);
    escaped["files"][0]["contentType"] = Value::String("\u{1}".repeat(1024 * 1024));
    qualify(
        "escaped-content-1MiB",
        canonical(escaped),
        Expected::Accepted(1),
    );
    let mut tiny = manifest(vec![file("a".into())]);
    tiny["files"][0]["unknown"] = json!((0..40_000).map(|_| vec![Value::Null]).collect::<Vec<_>>());
    qualify("tiny-array-40k", canonical(tiny), Expected::Invalid);
    let mut wide = manifest(vec![file("a".into())]);
    wide["files"][0]["unknown"] = Value::Object(
        (0..40_000)
            .map(|i| (format!("k{i:08}"), Value::Null))
            .collect(),
    );
    qualify("wide-object-40k", canonical(wide), Expected::Invalid);
    // AP numbers retain independently grown strings until the full DOM is rejected.
    let mut numbers = br#"{"unknown":["#.to_vec();
    for index in 0..128 {
        if index != 0 {
            numbers.push(b',');
        }
        numbers.extend(std::iter::repeat_n(b'1', 131_073));
    }
    numbers.extend_from_slice(b"]}");
    qualify("many-long-numbers", numbers, Expected::Invalid);
    for (name, number) in [
        (
            "rounding-hard-mantissa",
            format!("0.{}5", "123456789".repeat(128)),
        ),
        (
            "rounding-subnormal",
            format!("4.{}e-324", "9406564584124654".repeat(128)),
        ),
    ] {
        qualify(
            name,
            format!("{{\"unknown\":{number}}}").into_bytes(),
            Expected::Invalid,
        );
    }
    let mut invalid = manifest(vec![file("a".into())]);
    invalid["files"][0]["size"] = Value::String("\u{1}".repeat(256 * 1024));
    qualify("large-error", canonical(invalid), Expected::Invalid);
    qualify(
        "wrong-root-string",
        canonical(Value::String("x".repeat(1024 * 1024))),
        Expected::Invalid,
    );
    qualify("file-denial", canonical(broad()), Expected::FileLimit);
    qualify(
        "work-denial",
        canonical(manifest(vec![file("a".into())])),
        Expected::WorkLimit,
    );
    qualify(
        "memory-denial",
        canonical(manifest(vec![file("a".into())])),
        Expected::MemoryLimit,
    );
    let mut projection_broad = (0..10_000)
        .map(|i| file(format!("f{i:08}")))
        .collect::<Vec<_>>();
    projection_broad.push(file("a".into()));
    projection_broad.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    qualify_projection(
        "projection-broad-10k",
        canonical(manifest(projection_broad)),
    );
    qualify_projection(
        "projection-deep-10k",
        canonical(manifest(vec![
            file("a".into()),
            file(format!("{}a", "d/".repeat(10_000))),
        ])),
    );
    let mut shared_targets = (0..1001).map(|i| json!({"path":format!("alias-{i:04}"),"role":"static","entryType":"symlink","linkTarget":"shared","size":0})).collect::<Vec<_>>();
    shared_targets.push(file("a".into()));
    let target = format!("{}z", "long/".repeat(600));
    shared_targets.push(file(target.clone()));
    shared_targets.push(
        json!({"path":"shared","role":"static","entryType":"symlink","linkTarget":target,"size":0}),
    );
    shared_targets.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    qualify_projection(
        "projection-shared-long-target",
        canonical(manifest(shared_targets)),
    );
    let mut authored = manifest(vec![file("a".into())]);
    authored["files"][0]["contentType"] = Value::String("x".repeat(8 * 1024 * 1024));
    qualify_projection("projection-content-8MiB", canonical(authored));
}
