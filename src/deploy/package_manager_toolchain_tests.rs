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

#[test]
fn platform_yarn_uses_the_selected_apps_nearest_workspace_generation() {
    let root = tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"packageManager":"yarn@1.22.22"}"#,
    )
    .unwrap();
    let workspace = root.path().join("nested");
    let app = workspace.join("apps/api");
    fs::create_dir_all(&app).unwrap();
    fs::write(
        workspace.join("package.json"),
        r#"{"private":true,"packageManager":"yarn@4.18.0","workspaces":["apps/*"]}"#,
    )
    .unwrap();
    fs::write(app.join("package.json"), r#"{"name":"api"}"#).unwrap();
    let detection = crate::detect::detect(&app);
    assert_eq!(
        detection
            .metadata
            .package_manager
            .unwrap()
            .version
            .as_deref(),
        Some("4.18.0")
    );
    assert_eq!(
        select_environment("YARN", &app, root.path(), PATHS)
            .unwrap()
            .as_slice(),
        [(
            "PATH".to_string(),
            "/opt/toolchains/yarn-modern/bin:/usr/local/bin:/usr/bin:/bin".to_string()
        )]
    );
}

#[test]
fn platform_installer_path_uses_admitted_node_instead_of_requested_line() {
    const CASE: &str = "NRZ_RUNTIME_POLICY_INSTALLER_PATH_TEST";
    let Ok(case) = std::env::var(CASE) else {
        for (case, compiler, ancillary) in [
            ("compiler", "node-26", None),
            ("ancillary", "bun-1.4.3", Some("26")),
            (
                "non_node",
                nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
                None,
            ),
        ] {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child.args(["--exact", "deploy::package_manager_toolchain_tests::platform_installer_path_uses_admitted_node_instead_of_requested_line"])
                .env(CASE, case).env("ONREZA_BUILD_RUNTIME_VERSION", compiler).env("PATH", "/base/bin");
            if let Some(major) = ancillary {
                child.env("ONREZA_BUILD_NODE_MAJOR", major);
            } else {
                child.env_remove("ONREZA_BUILD_NODE_MAJOR");
            }
            let output = child.output().unwrap();
            assert!(
                output.status.success(),
                "{case}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    };
    let project = tempdir().unwrap();
    let environment = super::package_manager_toolchain::environment_from_process(
        "NPM",
        project.path(),
        project.path(),
    )
    .unwrap();
    assert_eq!(
        environment.as_slice(),
        [(
            "PATH".into(),
            if case == "non_node" {
                "/base/bin"
            } else {
                "/opt/node/26/bin:/base/bin"
            }
            .into()
        )]
    );
}
