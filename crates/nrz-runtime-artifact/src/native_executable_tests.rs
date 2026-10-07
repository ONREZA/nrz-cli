use super::{
    NativeExecutableRequirements, qualified_system_path, qualified_versions, version_definitions,
};
use goblin::elf::{Elf, dynamic};
use std::{collections::BTreeMap, path::PathBuf};

#[test]
fn qualified_system_paths_use_unix_loader_components() {
    for path in ["/lib", "/lib64", "/usr/lib/x86_64-linux-gnu", "/usr/lib64"] {
        assert!(qualified_system_path(path));
    }
    for path in [
        "/library",
        "/lib/../../private",
        "/usr/lib/../../private",
        "/private",
    ] {
        assert!(!qualified_system_path(path));
    }
}

struct Artifact(PathBuf);

impl Artifact {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "nrz-native-versions-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn check(&self) -> Result<(), crate::RuntimeArtifactError> {
        NativeExecutableRequirements::verify_artifact_closure(
            &self.0,
            &self.0.join("server"),
            &self.0,
        )
        .map(|_| ())
    }
}

impl Drop for Artifact {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

// Valid file-backed ELF metadata without section headers, as permitted by the
// loader. Real compiled/loader counterexamples are also exercised by qualification.
fn versioned_elf(
    needed: Option<(&str, &str)>,
    definition: Option<&str>,
    runpath: Option<&str>,
    executable: bool,
) -> Vec<u8> {
    let mut bytes = vec![0; 4096];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&(if executable { 2_u16 } else { 3_u16 }).to_le_bytes());
    bytes[18..20].copy_from_slice(&62_u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    if executable {
        bytes[24..32].copy_from_slice(&0x400100_u64.to_le_bytes());
    }
    bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&2_u16.to_le_bytes());
    bytes[64..68].copy_from_slice(&1_u32.to_le_bytes());
    bytes[68..72].copy_from_slice(&5_u32.to_le_bytes());
    bytes[80..88].copy_from_slice(&0x400000_u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&4096_u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&4096_u64.to_le_bytes());
    let mut tags: Vec<(u64, u64)> = vec![(dynamic::DT_STRTAB, 0x400300), (dynamic::DT_STRSZ, 512)];
    if let Some((library, version)) = needed {
        bytes[769..769 + library.len()].copy_from_slice(library.as_bytes());
        bytes[896..896 + version.len()].copy_from_slice(version.as_bytes());
        tags.extend([
            (dynamic::DT_NEEDED, 1),
            (dynamic::DT_VERNEED, 0x400500),
            (dynamic::DT_VERNEEDNUM, 1),
        ]);
        bytes[1280..1282].copy_from_slice(&1_u16.to_le_bytes());
        bytes[1282..1284].copy_from_slice(&1_u16.to_le_bytes());
        bytes[1284..1288].copy_from_slice(&1_u32.to_le_bytes());
        bytes[1288..1292].copy_from_slice(&16_u32.to_le_bytes());
        bytes[1296..1300].copy_from_slice(&super::version_name_hash(version).to_le_bytes());
        bytes[1302..1304].copy_from_slice(&2_u16.to_le_bytes());
        bytes[1304..1308].copy_from_slice(&128_u32.to_le_bytes());
    }
    if let Some(version) = definition {
        bytes[1024..1024 + version.len()].copy_from_slice(version.as_bytes());
        tags.extend([(dynamic::DT_VERDEF, 0x400600), (dynamic::DT_VERDEFNUM, 1)]);
        bytes[1536..1538].copy_from_slice(&1_u16.to_le_bytes());
        bytes[1540..1542].copy_from_slice(&2_u16.to_le_bytes());
        bytes[1542..1544].copy_from_slice(&1_u16.to_le_bytes());
        bytes[1544..1548].copy_from_slice(&super::version_name_hash(version).to_le_bytes());
        bytes[1548..1552].copy_from_slice(&20_u32.to_le_bytes());
        bytes[1556..1560].copy_from_slice(&256_u32.to_le_bytes());
    }
    if let Some(path) = runpath {
        bytes[1152..1152 + path.len()].copy_from_slice(path.as_bytes());
        tags.push((dynamic::DT_RUNPATH, 384));
    }
    tags.push((dynamic::DT_NULL, 0));
    bytes[120..124].copy_from_slice(&2_u32.to_le_bytes());
    bytes[128..136].copy_from_slice(&512_u64.to_le_bytes());
    bytes[136..144].copy_from_slice(&0x400200_u64.to_le_bytes());
    bytes[152..160].copy_from_slice(&(tags.len() as u64 * 16).to_le_bytes());
    bytes[160..168].copy_from_slice(&(tags.len() as u64 * 16).to_le_bytes());
    for (index, (tag, value)) in tags.iter().enumerate() {
        let offset = 512 + index * 16;
        bytes[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
        bytes[offset + 8..offset + 16].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[test]
fn system_version_requirements_are_checked_without_section_headers() {
    let artifact = Artifact::new();
    for (library, version) in [
        ("libc.so.6", "GLIBC_9.99"),
        ("libstdc++.so.6", "GLIBCXX_9.99"),
        ("libstdc++.so.6", "CXXABI_9.99"),
        ("libgcc_s.so.1", "GCC_99.0"),
    ] {
        std::fs::write(
            artifact.0.join("server"),
            versioned_elf(Some((library, version)), None, None, true),
        )
        .unwrap();
        let error = artifact.check().unwrap_err().to_string();
        assert!(error.contains(version), "{error}");
    }
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(Some(("libc.so.6", "GLIBC_2.2.5")), None, None, true),
    )
    .unwrap();
    artifact.check().unwrap();
}

#[test]
fn weak_version_nodes_are_not_mandatory_but_unknown_flags_are_rejected() {
    let artifact = Artifact::new();
    let mut bytes = versioned_elf(Some(("libc.so.6", "GLIBC_9.99")), None, None, true);
    std::fs::write(artifact.0.join("server"), &bytes).unwrap();
    assert!(artifact.check().is_err());
    bytes[1300..1302].copy_from_slice(&goblin::elf::symver::VER_FLG_WEAK.to_le_bytes());
    std::fs::write(artifact.0.join("server"), &bytes).unwrap();
    artifact.check().unwrap();
    // The loader also treats a hash/name mismatch as a missing weak node: a
    // warning, not a mandatory load failure.
    bytes[1296..1300].copy_from_slice(&(!super::version_name_hash("GLIBC_9.99")).to_le_bytes());
    std::fs::write(artifact.0.join("server"), &bytes).unwrap();
    artifact.check().unwrap();
    bytes[1300..1302].copy_from_slice(&4_u16.to_le_bytes());
    std::fs::write(artifact.0.join("server"), bytes).unwrap();
    assert!(artifact.check().is_err());
}

#[test]
fn strong_version_name_hashes_and_provider_hashes_must_match() {
    let artifact = Artifact::new();
    let mut bytes = versioned_elf(Some(("libc.so.6", "GLIBC_2.2.5")), None, None, true);
    bytes[1296..1300].copy_from_slice(&(!super::version_name_hash("GLIBC_2.2.5")).to_le_bytes());
    std::fs::write(artifact.0.join("server"), bytes).unwrap();
    assert!(
        artifact.check().is_err(),
        "same name with wrong mandatory hash is not compatible"
    );

    std::fs::create_dir(artifact.0.join("lib")).unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(
            Some(("libfoo.so", "FOO_1")),
            None,
            Some("$ORIGIN/lib"),
            true,
        ),
    )
    .unwrap();
    let mut provider = versioned_elf(None, Some("FOO_1"), None, false);
    provider[1544..1548].copy_from_slice(&(!super::version_name_hash("FOO_1")).to_le_bytes());
    std::fs::write(artifact.0.join("lib/libfoo.so"), provider).unwrap();
    assert!(
        artifact.check().is_err(),
        "provider hash must match its declared version name"
    );
}

#[test]
fn bundled_versions_override_the_system_version_baseline() {
    let artifact = Artifact::new();
    std::fs::create_dir(artifact.0.join("lib")).unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(
            Some(("libc.so.6", "GLIBC_2.2.5")),
            None,
            Some("$ORIGIN/lib"),
            true,
        ),
    )
    .unwrap();
    std::fs::write(
        artifact.0.join("lib/libc.so.6"),
        versioned_elf(None, Some("GLIBC_9.99"), None, false),
    )
    .unwrap();
    assert!(
        artifact
            .check()
            .unwrap_err()
            .to_string()
            .contains("GLIBC_2.2.5")
    );
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(
            Some(("libc.so.6", "GLIBC_9.99")),
            None,
            Some("$ORIGIN/lib"),
            true,
        ),
    )
    .unwrap();
    artifact.check().unwrap();
}

#[test]
fn bundled_library_transitive_version_needs_use_the_compute_baseline() {
    let artifact = Artifact::new();
    std::fs::create_dir(artifact.0.join("lib")).unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(
            Some(("libfoo.so", "FOO_1")),
            None,
            Some("$ORIGIN/lib"),
            true,
        ),
    )
    .unwrap();
    for version in ["GLIBC_9.99", "GLIBC_2.2.5"] {
        std::fs::write(
            artifact.0.join("lib/libfoo.so"),
            versioned_elf(Some(("libc.so.6", version)), Some("FOO_1"), None, false),
        )
        .unwrap();
        let result = artifact.check();
        if version == "GLIBC_9.99" {
            assert!(result.unwrap_err().to_string().contains(version));
        } else {
            result.unwrap();
        }
    }
}

#[test]
fn malformed_loader_version_metadata_fails_closed() {
    let artifact = Artifact::new();
    let valid = versioned_elf(Some(("libc.so.6", "GLIBC_2.2.5")), None, None, true);
    for (offset, bad) in [
        (568, 0xffff_ffff_u64.to_le_bytes().to_vec()), // DT_VERNEED outside PT_LOAD
        (584, 4097_u64.to_le_bytes().to_vec()),        // declared count bound
        (584, 2_u64.to_le_bytes().to_vec()),           // truncated verneed chain
        (1282, 2_u16.to_le_bytes().to_vec()),          // truncated auxiliary chain
        (1288, 0_u32.to_le_bytes().to_vec()),          // aux points inside its header
        (1304, 0xffff_ffff_u32.to_le_bytes().to_vec()), // invalid dynamic string
        (1308, 16_u32.to_le_bytes().to_vec()),         // unaccounted auxiliary entry
    ] {
        let mut bytes = valid.clone();
        bytes[offset..offset + bad.len()].copy_from_slice(&bad);
        std::fs::write(artifact.0.join("server"), bytes).unwrap();
        assert!(artifact.check().is_err(), "malformed field offset {offset}");
    }
}

#[test]
fn ambiguous_version_table_mapping_and_malformed_provider_are_rejected() {
    let artifact = Artifact::new();
    let mut bytes = versioned_elf(Some(("libc.so.6", "GLIBC_2.2.5")), None, None, true);
    bytes[56..58].copy_from_slice(&3_u16.to_le_bytes());
    let segment = bytes[64..120].to_vec();
    bytes[176..232].copy_from_slice(&segment);
    std::fs::write(artifact.0.join("server"), bytes).unwrap();
    assert!(
        artifact.check().is_err(),
        "two PT_LOAD mappings cannot select version bytes"
    );

    std::fs::create_dir(artifact.0.join("lib")).unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(
            Some(("libfoo.so", "FOO_1")),
            None,
            Some("$ORIGIN/lib"),
            true,
        ),
    )
    .unwrap();
    let provider = versioned_elf(None, Some("FOO_1"), None, false);
    for (offset, bad) in [
        (568, 2_u64.to_le_bytes().to_vec()), // truncated DT_VERDEFNUM chain
        (1542, 2_u16.to_le_bytes().to_vec()), // truncated verdaux chain
        (1548, 0_u32.to_le_bytes().to_vec()), // aux inside definition header
        (1556, 0xffff_ffff_u32.to_le_bytes().to_vec()), // invalid name
    ] {
        let mut bytes = provider.clone();
        bytes[offset..offset + bad.len()].copy_from_slice(&bad);
        std::fs::write(artifact.0.join("lib/libfoo.so"), bytes).unwrap();
        assert!(
            artifact.check().is_err(),
            "malformed provider field offset {offset}"
        );
    }
}

#[test]
fn qualified_system_version_baseline_matches_the_closed_soname_set() {
    let baseline: serde_json::Value =
        serde_json::from_str(NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARY_VERSIONS_JSON)
            .unwrap();
    assert_eq!(baseline["target"], super::NATIVE_EXECUTION_TARGET);
    let names = qualified_versions()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARIES
            .iter()
            .copied()
            .collect()
    );
    for versions in qualified_versions().values() {
        assert!(!versions.is_empty());
        assert!(
            versions
                .iter()
                .all(|name| !name.is_empty() && name.len() <= 256)
        );
    }
}

#[test]
#[ignore = "requires the qualified Compute system libraries; run in the minimal image"]
fn qualified_compute_system_library_versions_match_baseline() {
    let root = std::path::PathBuf::from(
        std::env::var_os("NRZ_NATIVE_SYSTEM_LIBRARY_ROOT")
            .expect("set qualified Compute library root"),
    );
    let mut actual = BTreeMap::new();
    for library in NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARIES {
        let bytes = std::fs::read(root.join(library)).unwrap();
        let elf = Elf::parse(&bytes).unwrap();
        super::inspect_elf(&elf, &bytes, false).unwrap();
        actual.insert(
            (*library).to_owned(),
            version_definitions(&elf, &bytes).unwrap(),
        );
    }
    if let Some(output) = std::env::var_os("NRZ_NATIVE_SYSTEM_LIBRARY_VERSIONS_OUTPUT") {
        std::fs::write(
            output,
            serde_json::to_vec_pretty(
                &serde_json::json!({"target":super::NATIVE_EXECUTION_TARGET,"libraries":actual}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    let baseline: serde_json::Value =
        serde_json::from_str(NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARY_VERSIONS_JSON)
            .unwrap();
    assert_eq!(baseline["target"], super::NATIVE_EXECUTION_TARGET);
    assert_eq!(
        &actual,
        qualified_versions(),
        "qualified Compute library version definitions drifted"
    );
    println!(
        "Compute native library version definitions PASS: {} libraries",
        actual.len()
    );
}
