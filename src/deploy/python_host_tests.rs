use super::*;

#[tokio::test]
async fn incompatible_host_build_refuses_native_target_tree_before_compiler_selection() {
    for (os, arch) in [
        ("macos", "aarch64"),
        ("macos", "x86_64"),
        ("windows", "x86_64"),
        ("linux", "aarch64"),
    ] {
        for (name, bytes) in [
            ("native.bin", b"\x7fELFfake-linux-payload".as_slice()),
            ("native.data", b"\xfe\xed\xfa\xcfMach-O".as_slice()),
            ("fat.data", b"\xca\xfe\xba\xbffat-Mach-O".as_slice()),
            ("windows.data", b"MZportable-executable".as_slice()),
            ("archive.data", b"!<arch>\nstatic-library".as_slice()),
            ("extension.SO", b"".as_slice()),
            ("extension.pyd", b"".as_slice()),
        ] {
            let project = tempfile::tempdir().unwrap();
            let minor = PythonMinor::default();
            let stage = project.path().join(minor.site_packages_root());
            std::fs::create_dir_all(stage.join("demo/data")).unwrap();
            std::fs::write(stage.join("demo/data").join(name), bytes).unwrap();
            let result = build_environment_for_host(
                project.path(),
                PythonInstallMode::ManagedLocal,
                minor,
                &[],
                (os, arch),
                None,
            )
            .await;
            assert!(
                result.is_err(),
                "{os}/{arch} accepted Linux-target native stage in host Python build environment"
            );
            let error = result.unwrap_err();
            assert!(error.to_string().contains("native payload"), "{error:#}");
            assert!(error.to_string().contains(name), "{error:#}");
            assert!(error.to_string().contains("Cloud Builder"), "{error:#}");
            assert!(
                error
                    .chain()
                    .filter_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
                    .any(|error| error.code == "PYTHON_PLATFORM_UNSUPPORTED")
            );
        }
    }
}

#[test]
fn portable_or_absent_target_dependencies_preserve_local_build_support() {
    let project = tempfile::tempdir().unwrap();
    let minor = PythonMinor::default();
    for host in [
        ("macos", "aarch64"),
        ("windows", "x86_64"),
        ("linux", "x86_64"),
    ] {
        validate_local_build_dependency_host(
            project.path(),
            PythonInstallMode::ManagedLocal,
            minor,
            host,
        )
        .unwrap();
    }
    let stage = project.path().join(minor.site_packages_root());
    std::fs::create_dir_all(stage.join("demo/node_modules")).unwrap();
    std::fs::write(stage.join("demo/__init__.py"), "VALUE=42\n").unwrap();
    std::fs::write(
        stage.join("demo/node_modules/schema.json"),
        "{\"type\":\"string\"}",
    )
    .unwrap();
    std::fs::write(stage.join("demo/image.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    for host in [("macos", "aarch64"), ("windows", "x86_64")] {
        validate_local_build_dependency_host(
            project.path(),
            PythonInstallMode::ManagedLocal,
            minor,
            host,
        )
        .unwrap();
    }
    std::fs::write(stage.join("demo/native.so"), b"\x7fELF").unwrap();
    validate_local_build_dependency_host_with_glibc(
        project.path(),
        PythonInstallMode::ManagedLocal,
        minor,
        ("linux", "x86_64"),
        Some(python_target_glibc_floor()),
    )
    .unwrap();
    validate_local_build_dependency_host(
        project.path(),
        PythonInstallMode::PinnedPlatform,
        minor,
        ("linux", "x86_64"),
    )
    .unwrap();
}

#[test]
fn portable_wheel_recipe_rejects_native_payload_with_shared_host_evidence() {
    for (name, contents, accepted) in [
        ("demo/__init__.py", "VALUE=42\n".as_bytes(), true),
        ("demo/data/native.bin", b"\x7fELFpayload".as_slice(), false),
        (
            "demo/data/native.bin",
            b"\xfe\xed\xfa\xcfpayload".as_slice(),
            false,
        ),
        ("demo/data/native.bin", b"MZpayload".as_slice(), false),
        ("demo/native.so", b"".as_slice(), false),
    ] {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("pyproject.toml"),"[project]\nname='demo'\nversion='1.0'\n[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n").unwrap();
        let commands = install_commands(
            project.path(),
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
            PythonMinor::default(),
        )
        .unwrap();
        let qualification = commands
            .iter()
            .find(|command| {
                command
                    .display
                    .contains("qualify portable application wheel")
            })
            .unwrap();
        let script_index = qualification
            .arguments
            .iter()
            .position(|argument| argument == "-c")
            .unwrap()
            + 1;
        let wheel = project
            .path()
            .join(PROJECT_WHEELS)
            .join("demo-1.0-py3-none-any.whl");
        std::fs::create_dir_all(wheel.parent().unwrap()).unwrap();
        let payload = contents
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let output=std::process::Command::new("python3").args(["-I","-c","import zipfile,sys; archive=zipfile.ZipFile(sys.argv[1], 'w'); archive.writestr(sys.argv[2], bytes.fromhex(sys.argv[3])); archive.close()"]).arg(&wheel).args([name,&payload]).output().unwrap();
        assert!(output.status.success());
        let output = std::process::Command::new("python3")
            .args(["-I", "-c"])
            .args(&qualification.arguments[script_index..])
            .current_dir(project.path())
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            accepted,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if !accepted {
            assert!(String::from_utf8_lossy(&output.stderr).contains("native payload"));
        }
    }
}

#[cfg(unix)]
#[test]
fn native_stage_inspection_handles_relative_symlinks_and_directory_cycles() {
    let project = tempfile::tempdir().unwrap();
    let minor = PythonMinor::default();
    let stage = project.path().join(minor.site_packages_root());
    std::fs::create_dir_all(stage.join("demo/data")).unwrap();
    std::fs::write(stage.join("demo/data/portable.py"), "VALUE=42\n").unwrap();
    std::os::unix::fs::symlink("..", stage.join("demo/data/parent")).unwrap();
    validate_local_build_dependency_host(
        project.path(),
        PythonInstallMode::ManagedLocal,
        minor,
        ("macos", "aarch64"),
    )
    .unwrap();
    let external = project.path().join("host-native");
    std::fs::create_dir(&external).unwrap();
    std::fs::write(external.join("payload.bin"), b"\x7fELFnative").unwrap();
    std::os::unix::fs::symlink(
        "../../../../../../host-native/payload.bin",
        stage.join("demo/data/alias.py"),
    )
    .unwrap();
    let error = validate_local_build_dependency_host(
        project.path(),
        PythonInstallMode::ManagedLocal,
        minor,
        ("macos", "aarch64"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("alias.py"), "{error:#}");
    std::fs::remove_file(stage.join("demo/data/alias.py")).unwrap();
    std::os::unix::fs::symlink(
        "../../../../../../host-native",
        stage.join("demo/data/linked-directory"),
    )
    .unwrap();
    assert!(
        validate_local_build_dependency_host(
            project.path(),
            PythonInstallMode::ManagedLocal,
            minor,
            ("windows", "x86_64")
        )
        .is_err()
    );
}

#[tokio::test]
async fn older_linux_glibc_refuses_native_stage_before_compiler_selection() {
    assert_linux_libc_refuses_native_stage(Some((2, 38))).await;
}

#[tokio::test]
async fn musl_or_unknown_linux_libc_refuses_native_stage_before_compiler_selection() {
    assert_linux_libc_refuses_native_stage(None).await;
}

async fn assert_linux_libc_refuses_native_stage(glibc_version: Option<(u32, u32)>) {
    let project = tempfile::tempdir().unwrap();
    let minor = PythonMinor::default();
    let stage = project.path().join(minor.site_packages_root());
    std::fs::create_dir_all(&stage).unwrap();
    std::fs::write(stage.join("native.bin"), b"\x7fELFsynthetic-native").unwrap();
    let result = build_environment_for_host(
        project.path(),
        PythonInstallMode::ManagedLocal,
        minor,
        &[],
        ("linux", "x86_64"),
        glibc_version,
    )
    .await;
    assert!(
        result.is_err(),
        "Linux x86_64 host libc {glibc_version:?} accepted target {PLATFORM_PYTHON_TARGET} native stage"
    );
    let error = result.unwrap_err();
    assert!(error.to_string().contains("native.bin"), "{error:#}");
    assert!(
        error.to_string().contains(PLATFORM_PYTHON_TARGET),
        "{error:#}"
    );
    assert!(error.to_string().contains("Cloud Builder"), "{error:#}");
    assert!(
        error
            .chain()
            .filter_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
            .any(|error| error.code == "PYTHON_PLATFORM_UNSUPPORTED")
    );
}

#[test]
fn linux_glibc_floor_keeps_portable_stage_and_qualified_native_builds() {
    let project = tempfile::tempdir().unwrap();
    let minor = PythonMinor::default();
    let host = ("linux", "x86_64");
    for glibc_version in [None, Some((2, 38)), Some((2, 39)), Some((2, 44))] {
        validate_local_build_dependency_host_with_glibc(
            project.path(),
            PythonInstallMode::ManagedLocal,
            minor,
            host,
            glibc_version,
        )
        .unwrap();
    }
    let stage = project.path().join(minor.site_packages_root());
    std::fs::create_dir_all(stage.join("demo/node_modules")).unwrap();
    std::fs::write(stage.join("demo/__init__.py"), "VALUE=42\n").unwrap();
    std::fs::write(
        stage.join("demo/node_modules/schema.json"),
        "PORTABLE_PACKAGE_ASSET",
    )
    .unwrap();
    for glibc_version in [None, Some((2, 38))] {
        validate_local_build_dependency_host_with_glibc(
            project.path(),
            PythonInstallMode::ManagedLocal,
            minor,
            host,
            glibc_version,
        )
        .unwrap();
    }
    std::fs::write(stage.join("native.bin"), b"\x7fELFsynthetic-native").unwrap();
    for glibc_version in [
        Some(python_target_glibc_floor()),
        Some((2, 44)),
        Some((3, 0)),
    ] {
        validate_local_build_dependency_host_with_glibc(
            project.path(),
            PythonInstallMode::ManagedLocal,
            minor,
            host,
            glibc_version,
        )
        .unwrap();
    }
    validate_local_build_dependency_host_with_glibc(
        project.path(),
        PythonInstallMode::PinnedPlatform,
        minor,
        host,
        None,
    )
    .unwrap();
}

#[test]
fn libc_probe_accepts_only_unambiguous_glibc_versions() {
    for (output, expected) in [
        ("glibc 2.38\n", Some((2, 38))),
        ("glibc 2.39\n", Some((2, 39))),
        ("glibc 2.44\n", Some((2, 44))),
        ("musl libc 1.2.5\n", None),
        ("", None),
        ("glibc 2", None),
        ("glibc 2.39.1", None),
        ("glibc +2.39", None),
        ("glibc 2.39\nglibc 2.38", None),
    ] {
        assert_eq!(parse_host_glibc_version(output), expected);
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn actual_linux_host_libc_probe_qualifies_native_stage() {
    let project = tempfile::tempdir().unwrap();
    let minor = PythonMinor::default();
    let stage = project.path().join(minor.site_packages_root());
    std::fs::create_dir_all(&stage).unwrap();
    std::fs::write(stage.join("native.bin"), b"\x7fELFsynthetic-native").unwrap();
    let observed = host_glibc_version();
    let result = validate_local_build_dependency_host(
        project.path(),
        PythonInstallMode::ManagedLocal,
        minor,
        ("linux", "x86_64"),
    );
    let qualified = observed.is_some_and(|version| version >= python_target_glibc_floor());
    assert_eq!(result.is_ok(), qualified);
    eprintln!(
        "actual host glibc={observed:?}, target={PLATFORM_PYTHON_TARGET}, qualified={qualified}"
    );
}

#[tokio::test]
#[ignore = "selects each qualified Python compiler and executes authored build startup"]
async fn real_python_build_startup_processes_pth_once() {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let project = tempfile::tempdir().unwrap();
        let stage = project.path().join(minor.site_packages_root());
        std::fs::create_dir_all(stage.join("wheel_code")).unwrap();
        std::fs::write(
            stage.join("wheel_code/packaged.py"),
            "VALUE='INSTALLED_WHEEL'\n",
        )
        .unwrap();
        std::fs::write(
            stage.join("package.pth"),
            "wheel_code\nimport builtins; builtins.pth_count=getattr(builtins,'pth_count',0)+1\n",
        )
        .unwrap();
        std::fs::write(
            stage.join("sitecustomize.py"),
            "import builtins; builtins.wheel_startup=True\n",
        )
        .unwrap();
        let environment = build_environment(
            project.path(),
            super::super::python_toolchain_tests::qualification_mode(),
            minor,
            &[],
        )
        .await
        .unwrap();
        let output = std::process::Command::new("python3")
            .args(["-c", "import builtins,packaged,sys; assert packaged.VALUE=='INSTALLED_WHEEL'; assert builtins.pth_count==1; assert builtins.wheel_startup; print(sys.argv[1:])", "$(id)", "two words"])
            .envs(environment).current_dir(project.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}: {}",
            minor.version(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "['$(id)', 'two words']"
        );
    }
}
