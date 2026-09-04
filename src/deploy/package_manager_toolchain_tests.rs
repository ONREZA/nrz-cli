use std::fs;

use tempfile::tempdir;

use super::package_manager_toolchain::{
    YarnToolchainPaths, select_environment, validate_toolchain_executables,
};

const PATHS: YarnToolchainPaths<'static> = YarnToolchainPaths {
    classic_bin_dir: "/opt/toolchains/yarn-classic/bin",
    modern_bin_dir: "/opt/toolchains/yarn-modern/bin",
    base_path: "/usr/local/bin:/usr/bin:/bin",
};

#[test]
fn platform_yarn_uses_classic_for_a_v1_lockfile() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("yarn.lock"), "# yarn lockfile v1\n").unwrap();

    assert_eq!(
        select_environment("YARN", dir.path(), dir.path(), PATHS)
            .unwrap()
            .as_slice(),
        [(
            "PATH".to_string(),
            "/opt/toolchains/yarn-classic/bin:/usr/local/bin:/usr/bin:/bin".to_string()
        )]
    );
}

#[test]
fn platform_yarn_uses_modern_for_a_modern_package_manager_field() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager":"yarn@4.18.0"}"#,
    )
    .unwrap();

    assert_eq!(
        select_environment("YARN", dir.path(), dir.path(), PATHS)
            .unwrap()
            .as_slice(),
        [(
            "PATH".to_string(),
            "/opt/toolchains/yarn-modern/bin:/usr/local/bin:/usr/bin:/bin".to_string()
        )]
    );
}

#[test]
fn non_yarn_plan_does_not_require_yarn_toolchains() {
    let dir = tempdir().unwrap();
    let invalid_paths = YarnToolchainPaths {
        classic_bin_dir: "",
        modern_bin_dir: "",
        base_path: "",
    };

    assert!(
        select_environment("NPM", dir.path(), dir.path(), invalid_paths)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn platform_yarn_uses_the_checkout_lock_for_a_nested_project() {
    let root = tempdir().unwrap();
    let project = root.path().join("apps/web");
    fs::create_dir_all(&project).unwrap();
    fs::write(root.path().join("yarn.lock"), "# yarn lockfile v1\n").unwrap();

    assert_eq!(
        select_environment("YARN", &project, root.path(), PATHS)
            .unwrap()
            .as_slice(),
        [(
            "PATH".to_string(),
            "/opt/toolchains/yarn-classic/bin:/usr/local/bin:/usr/bin:/bin".to_string()
        )]
    );
}

#[cfg(unix)]
#[test]
fn platform_yarn_rejects_a_non_executable_bundled_toolchain() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("yarn"), "#!/bin/sh\n").unwrap();
    fs::write(dir.path().join("yarnpkg"), "#!/bin/sh\n").unwrap();

    let error =
        validate_toolchain_executables(dir.path().to_str().unwrap(), "Yarn Classic").unwrap_err();

    assert!(error.to_string().contains("is not executable"));
}
