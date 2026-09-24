use crate::manifest::OnrezaBuildOutputManifestLayersItemEntry;

#[test]
fn manifest_compute_entry_obeys_platform_path_limit() {
    let longest_valid = "a".repeat(512);
    assert!(
        longest_valid
            .parse::<OnrezaBuildOutputManifestLayersItemEntry>()
            .is_ok()
    );

    let too_long = "a".repeat(513);
    assert!(
        too_long
            .parse::<OnrezaBuildOutputManifestLayersItemEntry>()
            .is_err()
    );
}
