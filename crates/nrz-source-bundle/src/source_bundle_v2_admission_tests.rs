use super::*;
use crate::source_bundle_v2::{
    encode_source_bundle_v2_manifest,
    tests::{budget, manifest},
};
use std::sync::Barrier;

fn encoded() -> Vec<u8> {
    encode_source_bundle_v2_manifest(manifest(), budget()).unwrap()
}

fn input(pool: &ManifestMemoryPool, bytes: &[u8]) -> ManifestInput {
    let mut input = pool.reserve_input(bytes.len()).unwrap();
    for chunk in bytes.chunks(17) {
        input.append(chunk).unwrap();
    }
    input
}

#[test]
fn input_capacity_is_acquired_before_allocation_and_released_on_cancel() {
    let pool = ManifestMemoryPool::new(32);
    assert!(matches!(
        pool.reserve_input(33),
        Err(AdmissionError::MemoryUnavailable { .. })
    ));
    assert_eq!(pool.reserved_bytes(), 0);
    let mut body = pool.reserve_input(32).unwrap();
    assert_eq!(pool.reserved_bytes(), 32);
    body.append(&[0; 16]).unwrap();
    assert!(matches!(
        body.append(&[0; 17]),
        Err(AdmissionError::InputLength)
    ));
    assert_eq!(body.bytes.len(), 16);
    drop(body);
    assert_eq!(pool.reserved_bytes(), 0);
    assert!(matches!(
        pool.reserve_input(SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES + 1),
        Err(AdmissionError::ManifestByteLimit)
    ));
    let body = pool.reserve_input(32).unwrap();
    assert!(matches!(
        body.verify(budget(), u64::MAX),
        Err(AdmissionError::InputLength)
    ));
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn every_reservation_stage_denies_before_starting_unbudgeted_work() {
    let bytes = encoded();
    let length = bytes.len() as u64;
    let profile = nrz_contract::artifact_json_v1::allocation_profile(&bytes);
    let scratch = memory::preflight_reservation(&profile).unwrap();
    let counts = preflight::inspect(&bytes, 20, profile).unwrap();
    let peak = memory::verification_reservation(&counts).unwrap();
    assert!(peak > scratch);
    for capacity in [
        length,
        length + scratch - 1,
        length + scratch,
        length + peak - 1,
    ] {
        let pool = ManifestMemoryPool::new(capacity);
        assert!(matches!(
            input(&pool, &bytes).verify(budget(), u64::MAX),
            Err(AdmissionError::MemoryUnavailable { .. })
        ));
        assert_eq!(pool.reserved_bytes(), 0);
    }
    let pool = ManifestMemoryPool::new(length + peak);
    let admitted = input(&pool, &bytes).verify(budget(), u64::MAX).unwrap();
    assert_eq!(pool.reserved_bytes(), peak);
    drop(admitted);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn proof_moves_with_its_reservation_and_only_borrows_verified_data() {
    let bytes = encoded();
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let admitted = input(&pool, &bytes).verify(budget(), u64::MAX).unwrap();
    assert_eq!(admitted.verified().summary().file_count, 1);
    assert_eq!(
        admitted.verified().manifest_blake3(),
        blake3::hash(&bytes).to_hex().as_str()
    );
    assert_eq!(admitted.reserved_bytes(), pool.reserved_bytes());
    let moved = std::thread::spawn(move || admitted).join().unwrap();
    assert!(pool.reserved_bytes() > bytes.len() as u64);
    drop(moved);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn failures_release_input_and_work_without_conflating_invalid_and_resource_limits() {
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    // The unparsed next element is malformed: the file limit must win first.
    assert!(matches!(
        input(&pool, br#"{"files":["\uZZZZ"]}"#).verify(
            SourceBundleV2ManifestBudget {
                max_file_count: 0,
                ..budget()
            },
            u64::MAX
        ),
        Err(AdmissionError::FileCountLimit)
    ));
    assert_eq!(pool.reserved_bytes(), 0);
    assert!(matches!(
        input(&pool, &encoded()).verify(budget(), 0),
        Err(AdmissionError::PathWorkLimit)
    ));
    assert_eq!(pool.reserved_bytes(), 0);
    let mut noncanonical = encoded();
    noncanonical.push(b'\n');
    assert!(matches!(
        input(&pool, &noncanonical).verify(budget(), u64::MAX),
        Err(AdmissionError::InvalidManifest(_))
    ));
    assert_eq!(pool.reserved_bytes(), 0);
    assert!(matches!(
        input(&pool, b"not json").verify(budget(), u64::MAX),
        Err(AdmissionError::InvalidManifest(_))
    ));
    assert_eq!(pool.reserved_bytes(), 0);
    let mut malformed = manifest();
    malformed["schemaVersion"] = serde_json::json!("\u{1}".repeat(4096));
    let bytes =
        nrz_contract::artifact_json_v1::encode(&malformed, crate::source_bundle_v2::limits())
            .unwrap();
    let Err(AdmissionError::InvalidManifest(message)) =
        input(&pool, &bytes).verify(budget(), u64::MAX)
    else {
        panic!("expected shape rejection")
    };
    assert!(message.len() <= 512);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn concurrent_owners_share_one_capacity_and_rollback_on_drop() {
    let pool = ManifestMemoryPool::new(8);
    let attempts_done = Arc::new(Barrier::new(17));
    let release = Arc::new(Barrier::new(17));
    let threads: Vec<_> = (0..16)
        .map(|_| {
            let pool = pool.clone();
            let attempts_done = attempts_done.clone();
            let release = release.clone();
            std::thread::spawn(move || {
                let reservation = pool.reserve_input(2);
                attempts_done.wait();
                release.wait();
                reservation.is_ok()
            })
        })
        .collect();
    attempts_done.wait();
    assert_eq!(pool.reserved_bytes(), 8);
    release.wait();
    assert_eq!(
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|accepted| *accepted)
            .count(),
        4
    );
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn checked_capacity_cannot_wrap_and_zero_capacity_is_a_real_limit() {
    let pool = ManifestMemoryPool::new(u64::MAX);
    let reservation = pool.reserve(u64::MAX).unwrap();
    assert!(matches!(
        pool.reserve(1),
        Err(AdmissionError::MemoryUnavailable { .. })
    ));
    drop(reservation);
    assert_eq!(pool.reserved_bytes(), 0);
    assert!(ManifestMemoryPool::new(0).reserve_input(1).is_err());
}

#[test]
fn large_plain_content_is_admitted_without_funding_input_sized_scratch() {
    let mut value = manifest();
    value["files"][0]["contentType"] = serde_json::Value::String("x".repeat(1024 * 1024));
    let bytes = encode_source_bundle_v2_manifest(value, budget()).unwrap();
    let pool = ManifestMemoryPool::new(4 * 1024 * 1024);
    let admitted = input(&pool, &bytes).verify(budget(), u64::MAX).unwrap();
    assert_eq!(
        admitted.verified().manifest_size_bytes(),
        bytes.len() as u64
    );
    assert_eq!(admitted.verified().wire().files.len(), 1);
    drop(admitted);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn numeric_fallback_is_funded_before_parsing_and_released_on_refusal() {
    let bytes = br#"{"unknown":1.0}"#;
    let profile = nrz_contract::artifact_json_v1::allocation_profile(bytes);
    assert!(profile.numeric_heap_candidate);
    let scratch = memory::preflight_reservation(&profile).unwrap();
    let pool = ManifestMemoryPool::new(bytes.len() as u64 + scratch - 1);
    assert!(matches!(
        input(&pool, bytes).verify(budget(), u64::MAX),
        Err(AdmissionError::MemoryUnavailable { requested_bytes }) if requested_bytes == scratch
    ));
    assert_eq!(pool.reserved_bytes(), 0);
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    assert!(matches!(
        input(&pool, bytes).verify(budget(), u64::MAX),
        Err(AdmissionError::InvalidManifest(_))
    ));
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn small_integer_manifest_still_fits_the_existing_four_mib_pool() {
    let bytes = encoded();
    let profile = nrz_contract::artifact_json_v1::allocation_profile(&bytes);
    assert!(!profile.numeric_heap_candidate);
    let pool = ManifestMemoryPool::new(4 * 1024 * 1024);
    let admitted = input(&pool, &bytes).verify(budget(), u64::MAX).unwrap();
    assert_eq!(admitted.verified().summary().file_count, 1);
    drop(admitted);
    assert_eq!(pool.reserved_bytes(), 0);
}
