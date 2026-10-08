use super::{
    NativeExecutableRequirements, qualified_system_path, qualified_versions, version_definitions,
};
use goblin::elf::{Elf, dynamic, program_header};
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
        static NEXT_ARTIFACT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT_ARTIFACT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "nrz-native-versions-{}-{nonce}-{sequence}",
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
    bytes[56..58].copy_from_slice(
        &(if executable && needed.is_some() {
            3_u16
        } else {
            2_u16
        })
        .to_le_bytes(),
    );
    bytes[64..68].copy_from_slice(&1_u32.to_le_bytes());
    bytes[68..72].copy_from_slice(&5_u32.to_le_bytes());
    bytes[80..88].copy_from_slice(&0x400000_u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&4096_u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&4096_u64.to_le_bytes());
    if executable && needed.is_some() {
        let interpreter = b"/lib64/ld-linux-x86-64.so.2\0";
        bytes[176..180].copy_from_slice(&program_header::PT_INTERP.to_le_bytes());
        bytes[180..184].copy_from_slice(&program_header::PF_R.to_le_bytes());
        bytes[184..192].copy_from_slice(&2048_u64.to_le_bytes());
        bytes[192..200].copy_from_slice(&0x400800_u64.to_le_bytes());
        bytes[208..216].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[216..224].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[2048..2048 + interpreter.len()].copy_from_slice(interpreter);
    }
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
fn absent_contained_loader_directories_do_not_hide_resolved_libraries() {
    let artifact = Artifact::new();
    std::fs::create_dir(artifact.0.join("lib")).unwrap();
    std::fs::write(
        artifact.0.join("lib/libfoo.so"),
        versioned_elf(None, Some("FOO_1"), None, false),
    )
    .unwrap();
    for path in [
        "$ORIGIN/lib:$ORIGIN/optional",
        "$ORIGIN/optional:$ORIGIN/lib",
    ] {
        std::fs::write(
            artifact.0.join("server"),
            versioned_elf(Some(("libfoo.so", "FOO_1")), None, Some(path), true),
        )
        .unwrap();
        artifact.check().unwrap();
    }

    for path in [
        "$ORIGIN/../missing",
        "../missing",
        "/__nrz_missing_outside__",
    ] {
        std::fs::write(
            artifact.0.join("server"),
            versioned_elf(Some(("libfoo.so", "FOO_1")), None, Some(path), true),
        )
        .unwrap();
        assert!(artifact.check().is_err(), "outside missing path: {path}");
    }
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(
            Some(("libfoo.so", "FOO_1")),
            None,
            Some("$ORIGIN/optional"),
            true,
        ),
    )
    .unwrap();
    assert!(
        artifact
            .check()
            .unwrap_err()
            .to_string()
            .contains("libfoo.so")
    );
}

#[cfg(unix)]
#[test]
fn missing_loader_directory_does_not_bypass_existing_symlink_ancestry() {
    let artifact = Artifact::new();
    let outside = Artifact::new();
    std::os::unix::fs::symlink(&outside.0, artifact.0.join("escape")).unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(None, None, Some("$ORIGIN/escape/missing"), true),
    )
    .unwrap();
    assert!(artifact.check().is_err());
    std::os::unix::fs::symlink(outside.0.join("missing"), artifact.0.join("dangling")).unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(None, None, Some("$ORIGIN/dangling"), true),
    )
    .unwrap();
    assert!(artifact.check().is_err());

    std::os::unix::fs::symlink("missing-inside", artifact.0.join("optional-link")).unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(None, None, Some("$ORIGIN/optional-link"), true),
    )
    .unwrap();
    artifact.check().unwrap();

    std::fs::write(artifact.0.join("not-a-directory"), "plain file").unwrap();
    std::fs::write(
        artifact.0.join("server"),
        versioned_elf(None, None, Some("$ORIGIN/not-a-directory/missing"), true),
    )
    .unwrap();
    assert!(
        artifact.check().is_err(),
        "non-NotFound lookup errors must reject"
    );
}

#[test]
fn dynamic_entry_requires_an_interpreter_while_shared_providers_do_not() {
    let artifact = Artifact::new();
    let entry = versioned_elf(Some(("libc.so.6", "GLIBC_2.2.5")), None, None, true);
    let requirements = super::verify_native_executable(&entry).unwrap();
    assert_eq!(
        requirements.interpreter.as_deref(),
        Some("/lib64/ld-linux-x86-64.so.2")
    );
    std::fs::write(artifact.0.join("server"), &entry).unwrap();
    artifact.check().unwrap();

    let mut stripped = entry.clone();
    stripped[176..180].copy_from_slice(&program_header::PT_NULL.to_le_bytes());
    let mut unterminated = entry.clone();
    unterminated[2048 + b"/lib64/ld-linux-x86-64.so.2".len()] = b'X';
    let mut range = entry.clone();
    range[184..192].copy_from_slice(&(entry.len() as u64 + 1).to_le_bytes());
    let mut duplicate = entry.clone();
    duplicate[56..58].copy_from_slice(&4_u16.to_le_bytes());
    duplicate[232..288].copy_from_slice(&entry[176..232]);
    duplicate[240..248].copy_from_slice(&2304_u64.to_le_bytes());
    let loader = b"/lib64/ld-linux-x86-64.so.2\0";
    duplicate[2304..2304 + loader.len()].copy_from_slice(loader);
    duplicate[2048] = b'x';
    assert_eq!(
        Elf::parse(&duplicate).unwrap().interpreter,
        Some("/lib64/ld-linux-x86-64.so.2")
    );
    for (name, bytes) in [
        ("stripped", stripped),
        ("unterminated", unterminated),
        ("range", range),
        ("duplicate", duplicate),
    ] {
        std::fs::write(artifact.0.join("server"), &bytes).unwrap();
        assert!(
            super::verify_native_executable(&bytes)
                .unwrap_err()
                .to_string()
                .contains("interpreter"),
            "{name}"
        );
        assert!(
            artifact
                .check()
                .unwrap_err()
                .to_string()
                .contains("interpreter"),
            "{name}"
        );
    }

    let provider = versioned_elf(Some(("libc.so.6", "GLIBC_2.2.5")), None, None, false);
    let elf = Elf::parse(&provider).unwrap();
    assert!(elf.interpreter.is_none());
    super::inspect_elf(&elf, &provider, false).unwrap();
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[test]
#[ignore = "requires C compiler/static libc or NRZ_NATIVE_ENTRY_FIXTURE_ROOT; executes trusted native entry fixtures"]
fn real_native_entry_interpreter_qualification() {
    let artifact = Artifact::new();
    let root = std::env::var_os("NRZ_NATIVE_ENTRY_FIXTURE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| artifact.0.clone());
    std::fs::create_dir_all(&root).unwrap();
    if !root.join("dynamic-entry").is_file() || !root.join("optional-runpath-entry").is_file() {
        build_native_entry_fixtures(&root);
    }
    for (name, dynamic) in [
        ("dynamic-entry", true),
        ("dynamic-exec", true),
        ("optional-runpath-entry", true),
        ("optional-rpath-entry", true),
        ("static-entry", false),
        ("static-pie-entry", false),
    ] {
        let path = root.join(name);
        let bytes = std::fs::read(&path).unwrap();
        let requirements = super::verify_native_executable(&bytes).unwrap();
        if name == "static-pie-entry" {
            assert_eq!(
                Elf::parse(&bytes).unwrap().header.e_type,
                goblin::elf::header::ET_DYN
            );
        }
        assert_eq!(requirements.interpreter.is_some(), dynamic, "{name}");
        assert_eq!(!requirements.libraries.is_empty(), dynamic, "{name}");
        let output = std::process::Command::new(&path)
            .current_dir(&root)
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("LD_PRELOAD")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            if dynamic {
                b"NATIVE_ENTRY_OK:42\n".as_slice()
            } else {
                b"NATIVE_STATIC_OK\n".as_slice()
            }
        );
        if name.starts_with("optional-") {
            eprintln!("{name}: actual trusted loader passed with absent optional directories");
        }
        let (_, members) =
            NativeExecutableRequirements::verify_artifact_closure(&root, &path, &root).unwrap();
        if dynamic {
            assert!(members.contains(&PathBuf::from("lib/libanswer.so")));
        }
    }
    for (name, runnable) in [
        ("outside-search-entry", true),
        ("unresolved-search-entry", false),
    ] {
        let path = root.join(name);
        let output = std::process::Command::new(&path)
            .current_dir(&root)
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("LD_PRELOAD")
            .output()
            .unwrap();
        assert_eq!(output.status.success(), runnable, "{name}");
        let error =
            NativeExecutableRequirements::verify_artifact_closure(&root, &path, &root).unwrap_err();
        if !runnable {
            assert!(error.to_string().contains("libanswer.so"), "{error}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("libanswer.so"));
        }
    }
    let provider = std::fs::read(root.join("lib/libanswer.so")).unwrap();
    let elf = Elf::parse(&provider).unwrap();
    assert!(elf.interpreter.is_none());
    assert!(elf.libraries.contains(&"libc.so.6"));
    super::inspect_elf(&elf, &provider, false).unwrap();

    // The first loader header is authoritative to Linux; goblin's optional
    // interpreter value alone can hide duplicate or truncated loader metadata.
    for (name, errno) in [
        ("duplicate-entry", 2),
        ("unterminated-entry", 8),
        ("range-entry", 5),
    ] {
        let error = std::process::Command::new(root.join(name))
            .current_dir(&root)
            .output()
            .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(errno), "{name}: {error}");
    }
    // Limit core size in this child before executing the deliberately stripped,
    // trusted fixture. This does not change the parent/test runner's limits.
    let stripped = std::process::Command::new("sh")
        .args(["-c", "ulimit -c 0; exec \"$@\"", "nrz-elf-fixture"])
        .arg(root.join("stripped-entry"))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(!stripped.status.success());

    let mut admitted = Vec::new();
    for name in [
        "stripped-entry",
        "duplicate-entry",
        "unterminated-entry",
        "range-entry",
    ] {
        let path = root.join(name);
        let bytes = std::fs::read(&path).unwrap();
        if super::verify_native_executable(&bytes).is_ok() {
            admitted.push(format!("byte guard: {name}"));
        }
        if NativeExecutableRequirements::verify_artifact_closure(&root, &path, &root).is_ok() {
            admitted.push(format!("closure guard: {name}"));
        }
    }
    assert!(
        admitted.is_empty(),
        "kernel-invalid native entries admitted: {admitted:?}"
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn build_native_entry_fixtures(root: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(
        root.join("provider.c"),
        "#include <stdlib.h>\nint answer(void) { return atoi(\"42\"); }\n",
    )
    .unwrap();
    std::fs::write(root.join("main.c"), "#include <stdio.h>\nextern int answer(void);\nint main(void) { printf(\"NATIVE_ENTRY_OK:%d\\n\", answer()); return 0; }\n").unwrap();
    std::fs::write(
        root.join("static.c"),
        "#include <stdio.h>\nint main(void) { puts(\"NATIVE_STATIC_OK\"); return 0; }\n",
    )
    .unwrap();
    let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    for arguments in [
        vec![
            "-shared",
            "-fPIC",
            "provider.c",
            "-Wl,-soname,libanswer.so",
            "-o",
            "lib/libanswer.so",
        ],
        vec![
            "-pie",
            "main.c",
            "-Llib",
            "-lanswer",
            "-Wl,-rpath,$ORIGIN/lib",
            "-o",
            "dynamic-entry",
        ],
        vec![
            "-no-pie",
            "main.c",
            "-Llib",
            "-lanswer",
            "-Wl,-rpath,$ORIGIN/lib",
            "-o",
            "dynamic-exec",
        ],
        vec![
            "-pie",
            "main.c",
            "-Llib",
            "-lanswer",
            "-Wl,-rpath,$ORIGIN/lib:$ORIGIN/optional",
            "-o",
            "optional-runpath-entry",
        ],
        vec![
            "-pie",
            "main.c",
            "-Llib",
            "-lanswer",
            "-Wl,--disable-new-dtags,-rpath,$ORIGIN/optional:$ORIGIN/lib",
            "-o",
            "optional-rpath-entry",
        ],
        vec![
            "-pie",
            "main.c",
            "-Llib",
            "-lanswer",
            "-Wl,-rpath,$ORIGIN/../__nrz_absent_outside__:$ORIGIN/lib",
            "-o",
            "outside-search-entry",
        ],
        vec![
            "-pie",
            "main.c",
            "-Llib",
            "-lanswer",
            "-Wl,-rpath,$ORIGIN/optional",
            "-o",
            "unresolved-search-entry",
        ],
        vec!["-static", "static.c", "-o", "static-entry"],
        vec!["-static-pie", "static.c", "-o", "static-pie-entry"],
    ] {
        let result = std::process::Command::new(&compiler)
            .args(&arguments)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{arguments:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let original = std::fs::read(root.join("dynamic-entry")).unwrap();
    let elf = Elf::parse(&original).unwrap();
    let header_offset =
        |index: usize| elf.header.e_phoff as usize + index * elf.header.e_phentsize as usize;
    let (index, interp) = elf
        .program_headers
        .iter()
        .enumerate()
        .find(|(_, segment)| segment.p_type == program_header::PT_INTERP)
        .unwrap();
    let interp_header = header_offset(index);
    let start = interp.p_offset as usize;
    let end = start + interp.p_filesz as usize;
    let loader = &original[start..end];
    assert_eq!(loader, b"/lib64/ld-linux-x86-64.so.2\0");
    let spare = header_offset(
        elf.program_headers
            .iter()
            .position(|segment| segment.p_type == program_header::PT_GNU_STACK)
            .unwrap(),
    );
    let mut stripped = original.clone();
    stripped[interp_header..interp_header + 4]
        .copy_from_slice(&program_header::PT_NULL.to_le_bytes());
    let mut unterminated = original.clone();
    unterminated[end - 1] = b'X';
    let mut duplicate = original.clone();
    let missing = b"/__nrz_missing_loader__\0";
    duplicate[start..end].fill(0);
    duplicate[start..start + missing.len()].copy_from_slice(missing);
    duplicate[spare..spare + 56].fill(0);
    duplicate[spare..spare + 4].copy_from_slice(&program_header::PT_INTERP.to_le_bytes());
    duplicate[spare + 4..spare + 8].copy_from_slice(&program_header::PF_R.to_le_bytes());
    duplicate[spare + 8..spare + 16].copy_from_slice(&(original.len() as u64).to_le_bytes());
    duplicate[spare + 32..spare + 40].copy_from_slice(&(loader.len() as u64).to_le_bytes());
    duplicate.extend_from_slice(loader);
    let mut range = original.clone();
    range[interp_header + 8..interp_header + 16]
        .copy_from_slice(&(original.len() as u64 + 4096).to_le_bytes());
    for (name, bytes) in [
        ("stripped-entry", stripped),
        ("duplicate-entry", duplicate),
        ("unterminated-entry", unterminated),
        ("range-entry", range),
    ] {
        std::fs::write(root.join(name), bytes).unwrap();
        std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
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
    bytes[56..58].copy_from_slice(&4_u16.to_le_bytes());
    let segment = bytes[64..120].to_vec();
    bytes[232..288].copy_from_slice(&segment);
    std::fs::write(artifact.0.join("server"), bytes).unwrap();
    assert!(
        artifact
            .check()
            .unwrap_err()
            .to_string()
            .contains("version metadata"),
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
