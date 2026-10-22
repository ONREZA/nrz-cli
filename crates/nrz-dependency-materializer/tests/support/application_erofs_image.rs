//! One canonical minimal EROFS inventory for application verification fixtures.
pub(super) fn metadata_only_image(uuid: [u8; 16]) -> Vec<u8> {
    let mut bytes = vec![0; 4096];
    for (offset, value) in [(1024, 0xe0f5e1e2_u32), (1032, 3), (1060, 1), (1104, 3)] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[1036] = 12;
    bytes[1038..1040].copy_from_slice(&40_u16.to_le_bytes());
    bytes[1040..1048].copy_from_slice(&2_u64.to_le_bytes());
    bytes[1072..1088].copy_from_slice(&uuid);
    bytes[1108..1110].copy_from_slice(&8_u16.to_le_bytes());
    bytes[1152..1160].copy_from_slice(&[6, 0, 0, 5, 0, 0, 0, 0]);
    let mut directory = vec![0; 36];
    for (index, (nid, kind, name)) in [(40_u64, 2_u8, "."), (40, 2, ".."), (50, 1, "file")]
        .into_iter()
        .enumerate()
    {
        directory[index * 12..index * 12 + 8].copy_from_slice(&nid.to_le_bytes());
        let offset = directory.len() as u16;
        directory[index * 12 + 8..index * 12 + 10].copy_from_slice(&offset.to_le_bytes());
        directory[index * 12 + 10] = kind;
        directory.extend_from_slice(name.as_bytes());
    }
    for (offset, mode, links, content) in [
        (1280, 0o40775_u16, 2_u16, directory.as_slice()),
        (1600, 0o100775, 1, b"abc".as_slice()),
    ] {
        bytes[offset..offset + 2].copy_from_slice(&4_u16.to_le_bytes());
        bytes[offset + 4..offset + 6].copy_from_slice(&mode.to_le_bytes());
        bytes[offset + 6..offset + 8].copy_from_slice(&links.to_le_bytes());
        bytes[offset + 8..offset + 12].copy_from_slice(&(content.len() as u32).to_le_bytes());
        bytes[offset + 16..offset + 20].copy_from_slice(&u32::MAX.to_le_bytes());
        bytes[offset + 32..offset + 32 + content.len()].copy_from_slice(content);
    }
    bytes
}

pub(super) fn write_extracted_tree(directory: &std::path::Path) {
    use std::{fs, os::unix::fs::PermissionsExt};
    let file = directory.join("file");
    fs::write(&file, b"abc").unwrap();
    for path in [directory, file.as_path()] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o775)).unwrap();
        fs::File::open(path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
            .unwrap();
    }
}
