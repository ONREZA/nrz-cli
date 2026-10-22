use super::*;
use std::io::Write;

const ROOT: u64 = 40;
const DIR: u64 = 50;
const FILE: u64 = 60;
const LINK: u64 = 70;
const DASH: u64 = 80;

fn set16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn set32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn set64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn inode(bytes: &mut [u8], nid: u64, mode: u16, links: u16, data: &[u8]) {
    let offset = nid as usize * 32;
    set16(bytes, offset, 4);
    set16(bytes, offset + 4, mode);
    set16(bytes, offset + 6, links);
    set32(bytes, offset + 8, data.len() as u32);
    set32(bytes, offset + 16, u32::MAX);
    bytes[offset + 32..offset + 32 + data.len()].copy_from_slice(data);
}
fn directory(entries: &[(u64, u8, &str)]) -> Vec<u8> {
    let mut bytes = vec![0; entries.len() * 12];
    for (index, (nid, kind, name)) in entries.iter().enumerate() {
        set64(&mut bytes, index * 12, *nid);
        let offset = bytes.len();
        set16(&mut bytes, index * 12 + 8, offset as u16);
        bytes[index * 12 + 10] = *kind;
        bytes.extend_from_slice(name.as_bytes());
    }
    bytes
}
fn fixture() -> Vec<u8> {
    let mut bytes = vec![0; BLOCK];
    set32(&mut bytes, 1024, 0xe0f5e1e2);
    set32(&mut bytes, 1032, 3);
    bytes[1036] = 12;
    set16(&mut bytes, 1038, ROOT as u16);
    set64(&mut bytes, 1040, 5);
    set32(&mut bytes, 1060, 1);
    set32(&mut bytes, 1104, 3);
    set16(&mut bytes, 1108, 8);
    bytes[1152..1160].copy_from_slice(&[6, 0, 0, 5, 0, 0, 0, 0]);
    inode(
        &mut bytes,
        ROOT,
        0o40775,
        3,
        &directory(&[
            (ROOT, 2, "."),
            (ROOT, 2, ".."),
            (DIR, 2, "a"),
            (DASH, 1, "a-"),
            (LINK, 7, "z"),
        ]),
    );
    inode(
        &mut bytes,
        DIR,
        0o40775,
        2,
        &directory(&[(DIR, 2, "."), (ROOT, 2, ".."), (FILE, 1, "f")]),
    );
    inode(&mut bytes, FILE, 0o100775, 1, b"abc");
    inode(&mut bytes, LINK, 0o120777, 1, b"a/f");
    inode(&mut bytes, DASH, 0o100775, 1, b"dash");
    bytes
}
fn expected() -> Vec<FileEntry> {
    serde_json::from_value(serde_json::json!([
        {"entryType":"file","path":"a-","role":"compute","blobDigest":{"algorithm":"BLAKE3","value":"0000000000000000000000000000000000000000000000000000000000000000"},"size":4,"executable":false},
        {"entryType":"file","path":"a/f","role":"compute","blobDigest":{"algorithm":"BLAKE3","value":"0000000000000000000000000000000000000000000000000000000000000000"},"size":3,"executable":true},
        {"entryType":"symlink","path":"z","role":"compute","linkTarget":"a/f","size":0}
    ])).unwrap()
}
fn limits() -> MetadataLimits {
    MetadataLimits {
        max_nodes: 5,
        max_path_bytes: 3,
        max_memory_bytes: 64 * 1024,
    }
}
fn verify(bytes: &[u8], policy: MetadataLimits) -> Result<u64> {
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(bytes).unwrap();
    let entries = expected();
    verify_erofs_metadata(
        &mut file,
        &entries.iter().collect::<Vec<_>>(),
        [0; 16],
        policy,
    )
}

#[test]
fn exact_inventory_accepts_implicit_directory_and_dash_sibling() {
    assert_eq!(verify(&fixture(), limits()).unwrap(), BLOCK as u64);
    let mut appended = fixture();
    appended.extend_from_slice(&[0; BLOCK]);
    assert!(verify(&appended, limits()).is_err());
}

#[test]
fn canonical_metadata_and_profile_corruptions_are_rejected() {
    let base = fixture();
    for offset in [
        1032,
        1036,
        1037,
        1048,
        1056,
        1068,
        1072,
        1088,
        1104,
        1108,
        1110,
        1152,
        1154,
        1156,
        FILE as usize * 32,
        FILE as usize * 32 + 2,
        FILE as usize * 32 + 4,
        FILE as usize * 32 + 12,
        FILE as usize * 32 + 24,
        FILE as usize * 32 + 26,
    ] {
        let mut bytes = base.clone();
        bytes[offset] ^= 0x80;
        assert!(
            verify(&bytes, limits()).is_err(),
            "accepted corruption at {offset}"
        );
    }
    let mut special = base.clone();
    set16(&mut special, FILE as usize * 32 + 4, 0o104775);
    assert!(verify(&special, limits()).is_err());
}

#[test]
fn leaf_sizes_types_and_link_text_must_match_manifest() {
    let mut bytes = fixture();
    set32(&mut bytes, FILE as usize * 32 + 8, u32::MAX);
    assert!(verify(&bytes, limits()).is_err());
    let mut bytes = fixture();
    bytes[LINK as usize * 32 + 32] = b'x';
    assert!(verify(&bytes, limits()).is_err());
    let mut bytes = fixture();
    bytes[ROOT as usize * 32 + 32 + 4 * 12 + 10] = 1;
    assert!(verify(&bytes, limits()).is_err());
}

#[test]
fn repeated_inodes_cycles_parent_corruption_and_order_are_rejected() {
    let root_table = ROOT as usize * 32 + 32;
    for (offset, value) in [
        (root_table + 2 * 12, ROOT),
        (root_table + 3 * 12, FILE),
        (root_table + 12, DIR),
    ] {
        let mut bytes = fixture();
        set64(&mut bytes, offset, value);
        assert!(verify(&bytes, limits()).is_err());
    }
    let mut bytes = fixture();
    let names = root_table + 5 * 12;
    bytes[names + 3] = b'z';
    assert!(verify(&bytes, limits()).is_err());
    let mut bytes = fixture();
    set16(&mut bytes, root_table + 8, u16::MAX);
    assert!(verify(&bytes, limits()).is_err());
}

#[test]
fn allocation_node_and_path_limits_are_checked_before_image_read() {
    for policy in [
        MetadataLimits {
            max_nodes: 4,
            ..limits()
        },
        MetadataLimits {
            max_path_bytes: 2,
            ..limits()
        },
        MetadataLimits {
            max_memory_bytes: 1,
            ..limits()
        },
    ] {
        assert!(matches!(verify(&[], policy), Err(MetadataError::Limit(_))));
    }
}

#[test]
fn oversized_directory_and_offset_overflow_fail_before_extraction() {
    let mut bytes = fixture();
    set32(&mut bytes, DIR as usize * 32 + 8, u32::MAX);
    assert!(verify(&bytes, limits()).is_err());
    let mut bytes = fixture();
    set32(&mut bytes, 1064, u32::MAX);
    assert!(verify(&bytes, limits()).is_err());
    let mut bytes = fixture();
    set16(&mut bytes, ROOT as usize * 32, 0);
    set32(&mut bytes, ROOT as usize * 32 + 16, u32::MAX);
    assert!(verify(&bytes, limits()).is_err());
}

#[test]
fn extended_inode_metadata_is_checked() {
    let mut bytes = fixture();
    let offset = DASH as usize * 32;
    bytes[offset..offset + 96].fill(0);
    set16(&mut bytes, offset, 5);
    set16(&mut bytes, offset + 4, 0o100775);
    set64(&mut bytes, offset + 8, 4);
    set32(&mut bytes, offset + 44, 1);
    bytes[offset + 64..offset + 68].copy_from_slice(b"dash");
    assert!(verify(&bytes, limits()).is_ok());
    set32(&mut bytes, offset + 28, 1);
    assert!(verify(&bytes, limits()).is_err());
}

#[test]
fn multiblock_directory_accepts_zero_padding_and_preserves_global_sort() {
    let mut bytes = fixture();
    bytes.resize(3 * BLOCK, 0);
    set32(&mut bytes, 1060, 3);
    set16(&mut bytes, ROOT as usize * 32, 0);
    let first = directory(&[
        (ROOT, 2, "."),
        (ROOT, 2, ".."),
        (DIR, 2, "a"),
        (DASH, 1, "a-"),
    ]);
    let second = directory(&[(LINK, 7, "z")]);
    set32(
        &mut bytes,
        ROOT as usize * 32 + 8,
        (BLOCK + second.len()) as u32,
    );
    set32(&mut bytes, ROOT as usize * 32 + 16, 1);
    bytes[BLOCK..BLOCK + first.len()].copy_from_slice(&first);
    bytes[2 * BLOCK..2 * BLOCK + second.len()].copy_from_slice(&second);
    assert_eq!(verify(&bytes, limits()).unwrap(), (3 * BLOCK) as u64);
    bytes[BLOCK + first.len() + 1] = b'x';
    assert!(verify(&bytes, limits()).is_err());
}
