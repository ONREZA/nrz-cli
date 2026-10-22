use super::*;

fn image(size: u64) -> File {
    let file = tempfile::tempfile().unwrap();
    file.set_len(size).unwrap();
    file
}

fn overwrite(file: &mut File, offset: u64, byte: u8) {
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(&[byte]).unwrap();
}

#[test]
fn package_roots_match_independent_veritysetup_2_8_8_vectors() {
    // Offline veritysetup --format=1 --no-superblock --hash=sha256, zero data/salt.
    for (blocks, tree_blocks, expected_root, expected_tree_digest) in [
        (
            1,
            0,
            "2e534ef0107d592d1706ac38ad58596b85ef3375d360a6d669f974792df5cb5a",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            2,
            1,
            "05927d36681daf2129a7ab821c194d61c7943aba7a8a38532b2e075ca1e56736",
            "77a8442bb10d67573d2ef5af3b8092820d04d0f3e71c4becfac69c4ab3933c2d",
        ),
        (
            128,
            1,
            "f66c183e247d6f5fff0f2e9250cd1a05a4bb083fbdad91767d9e396540ff8c13",
            "9594eb85962bd014da3e763ef9a1f6cb533ae09507c6632ff5bb9ca43e4df7c8",
        ),
        (
            129,
            3,
            "35d7502c55320932d5e1f52c7d38699f8ba2df3409bb52d5b4ad5b6dce18d18c",
            "669e7ea596b2315865161a05e02116ef3f53a2a3952bcfe9f80eb1e1e8564293",
        ),
    ] {
        let size = blocks * 4096;
        let mut file = image(size);
        let result = append_application_verity(&mut file, size, [0; 32]).unwrap();
        assert_eq!(result.verity.root_hash, expected_root);
        assert_eq!(result.verity.tree_offset, size);
        assert_eq!(result.verity.tree_length, tree_blocks * 4096);
        assert_eq!(result.blob_size, size + tree_blocks * 4096);
        let verified = verify_application_verity(&mut file, size, &result.verity).unwrap();
        assert_eq!(verified.blob_digest, result.blob_digest);
        file.rewind().unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(result.blob_digest, blake3::hash(&bytes).to_hex().as_str());
        assert_eq!(
            hex::encode(Sha256::digest(&bytes[size as usize..])),
            expected_tree_digest
        );
    }
}

#[test]
fn canonical_padding_is_zero_and_authenticated_with_nonzero_salt() {
    let mut file = image(4097);
    overwrite(&mut file, 0, 31);
    overwrite(&mut file, 4096, 19);
    let result = append_application_verity(&mut file, 4097, [7; 32]).unwrap();
    assert_eq!(result.verity.salt, "07".repeat(32));
    assert_eq!(
        result.verity.root_hash,
        "c6e64614c692f2a51e4c54ba93d346c5a80ac99406b67e32b28d716495eb4dfb"
    );
    let mut padding = [1; 4095];
    file.seek(SeekFrom::Start(4097)).unwrap();
    file.read_exact(&mut padding).unwrap();
    assert_eq!(padding, [0; 4095]);
    let mut first_block = [0; BLOCK_BYTES];
    read_block(&mut file, 0, &mut first_block).unwrap();
    let mut hasher = Sha256::new();
    hasher.update([7; 32]);
    hasher.update(first_block);
    let expected_first_hash: [u8; 32] = hasher.finalize().into();
    let mut stored = [0; 32];
    file.seek(SeekFrom::Start(result.verity.tree_offset))
        .unwrap();
    file.read_exact(&mut stored).unwrap();
    assert_eq!(stored, expected_first_hash);
    overwrite(&mut file, 4097, 1);
    assert!(verify_application_verity(&mut file, 4097, &result.verity).is_err());
}

#[test]
fn readback_rejects_every_data_and_tree_block_and_tree_padding_tamper() {
    let size = 129 * 4096;
    let mut file = image(size);
    let result = append_application_verity(&mut file, size, [0; 32]).unwrap();
    for block in 0..result.blob_size / 4096 {
        let offset = block * 4096;
        file.seek(SeekFrom::Start(offset)).unwrap();
        let mut original = [0];
        file.read_exact(&mut original).unwrap();
        overwrite(&mut file, offset, original[0] ^ 1);
        assert!(
            verify_application_verity(&mut file, size, &result.verity).is_err(),
            "accepted modified block {block}"
        );
        overwrite(&mut file, offset, original[0]);
    }
    // Root block has two child hashes, so this slot must remain canonical zero padding.
    overwrite(&mut file, result.verity.tree_offset + 64, 1);
    assert!(verify_application_verity(&mut file, size, &result.verity).is_err());
}

#[test]
fn padding_rejected_even_with_a_valid_hash_tree_and_root() {
    let mut file = image(8192);
    overwrite(&mut file, 4097, 1);
    let result = append_application_verity(&mut file, 8192, [0; 32]).unwrap();
    assert!(verify_application_verity(&mut file, 4097, &result.verity).is_err());
}

#[test]
fn readback_rejects_truncation_trailing_bytes_wrong_root_salt_and_geometry() {
    for size in [1, 8192, 129 * 4096] {
        let mut file = image(size);
        let result = append_application_verity(&mut file, size, [0; 32]).unwrap();
        let mut wrong_root = result.verity.clone();
        wrong_root.root_hash = "ff".repeat(32);
        assert!(verify_application_verity(&mut file, size, &wrong_root).is_err());
        let mut wrong_salt = result.verity.clone();
        wrong_salt.salt = "01".repeat(32);
        assert!(verify_application_verity(&mut file, size, &wrong_salt).is_err());
        for invalid in ["AB".repeat(32), "a".repeat(63), "gg".repeat(32)] {
            wrong_salt.salt = invalid;
            assert!(verify_application_verity(&mut file, size, &wrong_salt).is_err());
        }
        let mut geometry = result.verity.clone();
        geometry.data_offset = 4096;
        assert!(verify_application_verity(&mut file, size, &geometry).is_err());
        geometry = result.verity.clone();
        geometry.tree_offset += 4096;
        assert!(verify_application_verity(&mut file, size, &geometry).is_err());
        file.set_len(result.blob_size - 1).unwrap();
        assert!(verify_application_verity(&mut file, size, &result.verity).is_err());
        file.set_len(result.blob_size + 1).unwrap();
        assert!(verify_application_verity(&mut file, size, &result.verity).is_err());
    }
}

#[test]
fn append_rejects_changed_initial_length_invalid_sizes_and_nonregular_inodes() {
    for requested in [0, 4095, 4097, u64::MAX] {
        let mut file = image(4096);
        assert!(append_application_verity(&mut file, requested, [0; 32]).is_err());
        assert_eq!(file.metadata().unwrap().len(), 4096);
    }
    let mut device = File::open("/dev/null").unwrap();
    assert!(append_application_verity(&mut device, 1, [0; 32]).is_err());
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(4096).unwrap();
    let mut readonly = File::open(file.path()).unwrap();
    assert!(append_application_verity(&mut readonly, 4096, [0; 32]).is_err());
}

#[test]
fn layer_offsets_are_root_first_and_checked_without_allocating_image_bytes() {
    let layout = application_package_layout(16385 * 4096).unwrap();
    let layout_layers = layers(layout.data_block_count, layout.tree_offset).unwrap();
    assert_eq!(
        layout_layers
            .iter()
            .map(|layer| layer.blocks)
            .collect::<Vec<_>>(),
        [129, 2, 1]
    );
    assert_eq!(layout_layers[2].offset, layout.tree_offset);
    assert_eq!(layout_layers[1].offset, layout.tree_offset + 4096);
    assert_eq!(layout_layers[0].offset, layout.tree_offset + 3 * 4096);
    assert!(layers(129, u64::MAX - 4095).is_err());
    assert!(block_offset(u64::MAX, 1).is_err());
}
