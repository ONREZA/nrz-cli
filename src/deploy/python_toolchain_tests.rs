use std::ffi::OsString;

use super::python_toolchain::{ArchiveFormat, artifact_for, install_arguments, install_display};

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
        install_arguments("requirements.txt"),
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
            "--requirements",
            "requirements.txt",
        ]
        .map(OsString::from)
    );
    assert!(install_display("requirements.txt").contains("uv 0.10.0 / CPython 3.14"));
}

#[test]
fn project_manifest_install_targets_the_project() {
    let arguments = install_arguments("pyproject.toml");
    assert_eq!(arguments.last(), Some(&OsString::from(".")));
    assert!(!arguments.contains(&OsString::from("--requirements")));
}
