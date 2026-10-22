use nrz_contract::application_package_v1::{
    BlobReference, PackageDescriptor, Role, SchemaVersion, VerityGeometry, VerityHashAlgorithm,
};
use nrz_contract::source_bundle_v2::{BlobDigest, DigestAlgorithm};
use serde_json::{Value, json};

use super::*;

fn policy() -> ApplicationPackagePolicy {
    ApplicationPackagePolicy {
        canonicalization_profile: "test-erofs-verity-v1".into(),
        generator_identity: "test-toolchain@sha256:pinned".into(),
    }
}

fn descriptor(data_blocks: u64, tree_blocks: u64) -> PackageDescriptor {
    let policy = policy();
    PackageDescriptor {
        schema_version: SchemaVersion::ApplicationPackageV1,
        role: Role::Static,
        blob: BlobReference {
            media_type: APPLICATION_PACKAGE_MEDIA_TYPE.into(),
            digest: BlobDigest {
                algorithm: DigestAlgorithm::Blake3,
                value: "ab".repeat(32),
            },
            size: (data_blocks + tree_blocks) * 4096,
        },
        canonicalization_profile: policy.canonicalization_profile,
        generator_identity: policy.generator_identity,
        erofs_size: data_blocks * 4096,
        verity: VerityGeometry {
            version: 1,
            hash_algorithm: VerityHashAlgorithm::Sha256,
            root_hash: "cd".repeat(32),
            salt: "ef".repeat(32),
            data_block_size: 4096,
            hash_block_size: 4096,
            data_block_count: data_blocks,
            data_offset: 0,
            tree_offset: data_blocks * 4096,
            tree_length: tree_blocks * 4096,
        },
    }
}

#[test]
fn verity_geometry_matches_no_superblock_sha256_format_vectors() {
    // Independent veritysetup 2.8.8 outputs, including the single-data-block root.
    for (data_blocks, tree_blocks) in [
        (1, 0),
        (2, 1),
        (127, 1),
        (128, 1),
        (129, 3),
        (16384, 129),
        (16385, 132),
    ] {
        let wire = descriptor(data_blocks, tree_blocks);
        let verified = verify_application_package_descriptor(wire.clone(), &policy()).unwrap();
        assert_eq!(verified.wire(), &wire);
        let mut unpadded = wire;
        unpadded.erofs_size -= 4095;
        validate_application_package_descriptor(&unpadded, &policy()).unwrap();
        unpadded.erofs_size -= 1;
        assert!(validate_application_package_descriptor(&unpadded, &policy()).is_err());
    }
}

#[test]
fn package_layout_rejects_inconsistent_geometry_and_unsafe_integers() {
    let wire = serde_json::to_value(descriptor(129, 3)).unwrap();
    for (pointer, replacement) in [
        ("/blob/mediaType", json!("application/octet-stream")),
        ("/blob/size", json!(129 * 4096)),
        ("/blob/size", json!((129 + 3) * 4096 + 1)),
        ("/blob/size", json!(MAX_SAFE_INTEGER + 1)),
        ("/erofsSize", json!(0)),
        ("/erofsSize", json!(MAX_SAFE_INTEGER + 1)),
        ("/erofsSize", json!(128 * 4096)),
        ("/verity/version", json!(0)),
        ("/verity/version", json!(2)),
        ("/verity/dataBlockSize", json!(0)),
        ("/verity/dataBlockSize", json!(512)),
        ("/verity/hashBlockSize", json!(8192)),
        ("/verity/dataBlockCount", json!(0)),
        ("/verity/dataBlockCount", json!(128)),
        ("/verity/dataBlockCount", json!(MAX_SAFE_INTEGER)),
        ("/verity/dataBlockCount", json!(u64::MAX)),
        ("/verity/dataOffset", json!(4096)),
        ("/verity/dataOffset", json!(u64::MAX)),
        ("/verity/treeOffset", json!(129 * 4096 + 1)),
        ("/verity/treeOffset", json!(u64::MAX)),
        ("/verity/treeLength", json!(2 * 4096)),
        ("/verity/treeLength", json!(4 * 4096)),
        ("/verity/treeLength", json!(u64::MAX)),
    ] {
        let mut invalid = wire.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        let descriptor = serde_json::from_value(invalid).unwrap();
        assert!(
            validate_application_package_descriptor(&descriptor, &policy()).is_err(),
            "accepted invalid {pointer}"
        );
    }
}

#[test]
fn exact_profile_and_generator_policy_is_required() {
    let wire = descriptor(1, 0);
    for invalid_policy in [
        ApplicationPackagePolicy {
            canonicalization_profile: "other-profile".into(),
            ..policy()
        },
        ApplicationPackagePolicy {
            generator_identity: "other-toolchain".into(),
            ..policy()
        },
    ] {
        assert!(validate_application_package_descriptor(&wire, &invalid_policy).is_err());
    }
    for invalid in [String::new(), "x".repeat(257), "before\0after".into()] {
        for generator in [false, true] {
            let mut wire = descriptor(1, 0);
            let mut policy = policy();
            if generator {
                wire.generator_identity = invalid.clone();
                policy.generator_identity = invalid.clone();
            } else {
                wire.canonicalization_profile = invalid.clone();
                policy.canonicalization_profile = invalid.clone();
            }
            assert!(validate_application_package_descriptor(&wire, &policy).is_err());
        }
    }
    let mut wire = wire;
    wire.canonicalization_profile = "界".repeat(256);
    wire.generator_identity = "x".repeat(256);
    let policy = ApplicationPackagePolicy {
        canonicalization_profile: wire.canonicalization_profile.clone(),
        generator_identity: wire.generator_identity.clone(),
    };
    validate_application_package_descriptor(&wire, &policy).unwrap();
}

#[test]
fn digest_root_and_salt_reject_noncanonical_encodings() {
    let wire = serde_json::to_value(descriptor(2, 1)).unwrap();
    for pointer in ["/blob/digest/value", "/verity/rootHash", "/verity/salt"] {
        for invalid in [
            "a".repeat(63),
            "a".repeat(65),
            "AB".repeat(32),
            "gg".repeat(32),
        ] {
            let mut wire = wire.clone();
            *wire.pointer_mut(pointer).unwrap() = json!(invalid);
            let descriptor = serde_json::from_value(wire).unwrap();
            assert!(validate_application_package_descriptor(&descriptor, &policy()).is_err());
        }
    }
}

#[test]
fn closed_wire_shapes_reject_unknown_fields_nulls_and_digest_domain_swaps() {
    let wire = serde_json::to_value(descriptor(2, 1)).unwrap();
    for pointer in ["", "/blob", "/blob/digest", "/verity"] {
        let mut invalid = wire.clone();
        invalid
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(true));
        assert!(serde_json::from_value::<PackageDescriptor>(invalid).is_err());
    }
    for (pointer, replacement) in [
        ("/schemaVersion", json!("APPLICATION_PACKAGE_V2.0")),
        ("/role", json!("static")),
        ("/blob/digest/algorithm", json!("SHA256")),
        ("/verity/hashAlgorithm", json!("BLAKE3")),
        ("/erofsSize", json!(-1)),
        ("/erofsSize", json!(1.5)),
        ("/verity/dataBlockSize", json!(u64::MAX)),
    ] {
        let mut invalid = wire.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            serde_json::from_value::<PackageDescriptor>(invalid).is_err(),
            "accepted {pointer}"
        );
    }
    for field in wire.as_object().unwrap().keys() {
        let mut invalid = wire.clone();
        invalid[field] = Value::Null;
        assert!(serde_json::from_value::<PackageDescriptor>(invalid).is_err());
    }
    assert!(Role::Static < Role::Compute);
}

#[test]
fn layout_helper_checks_safe_integer_boundaries_without_image_allocation() {
    for size in [0, MAX_SAFE_INTEGER, MAX_SAFE_INTEGER + 1, u64::MAX] {
        assert!(application_package_layout(size).is_err());
    }
    let layout = application_package_layout(MAX_SAFE_INTEGER / 2).unwrap();
    assert!(layout.package_size <= MAX_SAFE_INTEGER);
    assert_eq!(layout.tree_offset % APPLICATION_VERITY_BLOCK_SIZE, 0);
    assert_eq!(layout.tree_length % APPLICATION_VERITY_BLOCK_SIZE, 0);
    assert_eq!(layout.package_size, layout.tree_offset + layout.tree_length);
    for (size, blocks, tree_blocks) in [(1, 1, 0), (4096, 1, 0), (4097, 2, 1), (129 * 4096, 129, 3)]
    {
        let layout = application_package_layout(size).unwrap();
        assert_eq!(layout.data_block_count, blocks);
        assert_eq!(
            layout.tree_length,
            tree_blocks * APPLICATION_VERITY_BLOCK_SIZE
        );
    }
}
