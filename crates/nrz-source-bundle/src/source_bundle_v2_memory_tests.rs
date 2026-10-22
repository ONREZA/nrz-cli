use super::*;

#[test]
fn maximum_content_type_has_a_bounded_useful_reservation() {
    let bytes = 512 * 1024 * 1024;
    let counts = StructuralCounts {
        objects: 3,
        object_members: 15,
        array_elements: 1,
        strings: 20,
        decoded_string_bytes: bytes,
        max_string_bytes: bytes,
        max_error_string_bytes: 16,
        max_path_bytes: 1,
        files: 1,
        unique_path_nodes: 1,
        ..StructuralCounts::default()
    };
    let reservation = verification_reservation(&counts).unwrap();
    assert!(reservation >= bytes);
    assert!(reservation + bytes < 3 * 1024 * 1024 * 1024 + 2 * 1024 * 1024);
}

#[test]
fn rejects_overflow_in_allocation_arithmetic() {
    let mut profile = nrz_contract::artifact_json_v1::allocation_profile(b"");
    profile.max_escaped_string_bytes = u64::MAX;
    assert!(preflight_reservation(&profile).is_none());
    profile.max_escaped_string_bytes = 0;
    profile.max_unquoted_run_bytes = u64::MAX;
    assert!(preflight_reservation(&profile).is_none());
    let fields: [fn(&mut StructuralCounts); 10] = [
        |counts: &mut StructuralCounts| {
            counts.allocation_profile.max_escaped_string_bytes = u64::MAX
        },
        |counts: &mut StructuralCounts| counts.allocation_profile.max_unquoted_run_bytes = u64::MAX,
        |counts: &mut StructuralCounts| counts.decoded_string_bytes = u64::MAX,
        |counts: &mut StructuralCounts| counts.owned_parser_string_bytes = u64::MAX,
        |counts: &mut StructuralCounts| counts.json_values = u64::MAX,
        |counts: &mut StructuralCounts| counts.object_members = u64::MAX,
        |counts: &mut StructuralCounts| counts.array_elements = u64::MAX,
        |counts: &mut StructuralCounts| counts.unique_path_nodes = u64::MAX,
        |counts: &mut StructuralCounts| counts.max_error_string_bytes = u64::MAX,
        |counts: &mut StructuralCounts| counts.metadata_decoded_string_bytes = u64::MAX,
    ];
    for field in fields {
        let mut counts = StructuralCounts::default();
        field(&mut counts);
        assert!(verification_reservation(&counts).is_none());
    }
}

#[test]
fn pinned_layouts_and_metadata_transients_are_covered() {
    assert!(supported_layout());
    assert_eq!(std::mem::size_of::<serde::__private229::de::Content>(), 32);
    let baseline = verification_reservation(&StructuralCounts::default()).unwrap();
    let counts = StructuralCounts {
        metadata_objects: 10,
        metadata_object_members: 100,
        metadata_array_elements: 100,
        metadata_strings: 100,
        metadata_decoded_string_bytes: 4 * 1024 * 1024,
        ..StructuralCounts::default()
    };
    assert!(verification_reservation(&counts).unwrap() > baseline + 12 * 1024 * 1024);
}

#[test]
fn scratch_funding_uses_shape_and_keeps_the_numeric_fallback() {
    use nrz_contract::artifact_json_v1::allocation_profile;
    let plain = format!("\"{}\"", "x".repeat(64 * 1024));
    let escaped = format!("\"{}\"", "\\u0001".repeat(1024));
    assert_eq!(
        preflight_reservation(&allocation_profile(plain.as_bytes())),
        Some(64 * 1024)
    );
    assert_eq!(
        preflight_reservation(&allocation_profile(escaped.as_bytes())),
        Some(5 * 6 * 1024 + 64 * 1024)
    );
    assert_eq!(
        preflight_reservation(&allocation_profile(b"19")),
        Some(6 + 64 * 1024)
    );
    assert_eq!(
        preflight_reservation(&allocation_profile(b"1.0")),
        Some(9 + 4 * 1024 * 1024)
    );
    assert_eq!(
        preflight_reservation(&allocation_profile(b"99999999999999999999")),
        Some(60 + 4 * 1024 * 1024)
    );
}
