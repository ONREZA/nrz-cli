use std::ffi::OsString;

use super::python_toolchain::{ArchiveFormat, PythonInstallMode, artifact_for, install_command};

#[test]
fn pins_uv_artifacts_for_every_cli_release_platform() {
    let cases = [
        (
            "linux",
            "x86_64",
            "x86_64-unknown-linux-musl",
            ArchiveFormat::TarGz,
        ),
        (
            "macos",
            "x86_64",
            "x86_64-apple-darwin",
            ArchiveFormat::TarGz,
        ),
        (
            "macos",
            "aarch64",
            "aarch64-apple-darwin",
            ArchiveFormat::TarGz,
        ),
        (
            "windows",
            "x86_64",
            "x86_64-pc-windows-msvc",
            ArchiveFormat::Zip,
        ),
    ];

    for (os, arch, target, format) in cases {
        let artifact = artifact_for(os, arch).unwrap();
        assert_eq!(artifact.target, target);
        assert_eq!(artifact.format, format);
        assert_eq!(artifact.archive_sha256.len(), 64);
        assert_eq!(artifact.binary_sha256.len(), 64);
    }
    assert!(artifact_for("linux", "aarch64").is_err());
}

#[test]
fn requirements_install_uses_managed_python_and_copy_materialization() {
    assert_eq!(
        install_command(
            "requirements.txt",
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
        )
        .unwrap()
        .arguments,
        [
            "pip",
            "install",
            "--python",
            "3.14",
            "--managed-python",
            "--link-mode",
            "copy",
            "--target",
            ".onreza/python/3.14/site-packages",
            "--python-platform",
            "x86_64-manylinux_2_39",
            "--only-binary",
            ":all:",
            "--requirements",
            "requirements.txt",
        ]
        .map(OsString::from)
    );
    assert!(
        install_command(
            "requirements.txt",
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
        )
        .unwrap()
        .display
        .contains("uv 0.10.0 / CPython 3.14")
    );
}

#[test]
fn project_manifest_install_resolves_only_runtime_compatible_wheels() {
    let arguments = install_command(
        "pyproject.toml",
        PythonInstallMode::ManagedLocal,
        "linux",
        "x86_64",
    )
    .unwrap()
    .arguments;
    assert!(arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--python-platform"),
            OsString::from("x86_64-manylinux_2_39"),
        ]
    }));
    assert!(arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--requirements"),
            OsString::from("pyproject.toml"),
        ]
    }));
    assert_ne!(arguments.last(), Some(&OsString::from(".")));
}

#[test]
fn platform_runner_uses_the_pinned_rootfs_python() {
    let command = install_command(
        "requirements.txt",
        PythonInstallMode::PinnedPlatform,
        "linux",
        "x86_64",
    )
    .unwrap();

    assert_eq!(command.program, std::path::PathBuf::from("python3.14"));
    assert_eq!(
        command.arguments,
        [
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-compile",
            "--target",
            ".onreza/python/3.14/site-packages",
            "--requirement",
            "requirements.txt",
        ]
        .map(OsString::from)
    );
    assert!(command.display.contains("pinned python3.14 / pip"));
}

#[test]
fn every_local_install_targets_linux_wheels_only() {
    let command = install_command(
        "pyproject.toml",
        PythonInstallMode::ManagedLocal,
        "macos",
        "aarch64",
    )
    .unwrap();

    assert!(command.arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--python-platform"),
            OsString::from("x86_64-manylinux_2_39"),
        ]
    }));
    assert!(
        command
            .arguments
            .windows(2)
            .any(|pair| { pair == [OsString::from("--only-binary"), OsString::from(":all:"),] })
    );
    assert!(command.arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--requirements"),
            OsString::from("pyproject.toml"),
        ]
    }));
    assert_ne!(command.arguments.last(), Some(&OsString::from(".")));
}

#[test]
fn every_local_setup_py_fails_before_host_native_build() {
    let error = install_command(
        "setup.py",
        PythonInstallMode::ManagedLocal,
        "windows",
        "x86_64",
    )
    .unwrap_err();

    assert!(error.to_string().contains("cannot be safely materialized"));
    assert!(error.to_string().contains("Cloud Builder"));

    let linux_error = install_command(
        "setup.py",
        PythonInstallMode::ManagedLocal,
        "linux",
        "x86_64",
    )
    .unwrap_err();
    assert!(linux_error.to_string().contains("Cloud Builder"));
}
