use super::*;

fn node_command_project(package_json: &str, lockfile: Option<&str>) -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), package_json).unwrap();
    if let Some(lockfile) = lockfile {
        fs::write(dir.path().join(lockfile), "").unwrap();
    }
    dir
}

fn prepared_python_project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
    let prepared = dir
        .path()
        .join(nrz_source_bundle::PythonMinor::default().site_packages_root())
        .join("prepared.py");
    fs::create_dir_all(prepared.parent().unwrap()).unwrap();
    fs::write(&prepared, "prepared = True").unwrap();
    (dir, prepared)
}

#[test]
fn native_output_cleanup_preserves_source_and_nested_build_siblings() {
    let project = tempdir().unwrap();
    fs::write(project.path().join("main.go"), "authored source").unwrap();
    fs::create_dir_all(project.path().join("build/onreza-dart/bundle")).unwrap();
    fs::write(
        project.path().join("build/onreza-dart/hooks.cache"),
        "compiler cache",
    )
    .unwrap();
    fs::write(
        project.path().join("build/onreza-dart/bundle/stale.txt"),
        "stale",
    )
    .unwrap();
    plan::clear_native_build_output(project.path(), "build/onreza-dart/bundle").unwrap();
    assert!(!project.path().join("build/onreza-dart/bundle").exists());
    assert_eq!(
        fs::read_to_string(project.path().join("build/onreza-dart/hooks.cache")).unwrap(),
        "compiler cache"
    );
    for invalid in [
        "",
        ".",
        "build/..",
        "../outside",
        "/tmp/output",
        "build//output",
        "build/./output",
        "main.go",
    ] {
        assert!(
            plan::clear_native_build_output(project.path(), invalid).is_err(),
            "{invalid}"
        );
        assert_eq!(
            fs::read_to_string(project.path().join("main.go")).unwrap(),
            "authored source"
        );
    }
}

#[cfg(unix)]
#[test]
fn native_output_cleanup_rejects_symlink_components_without_touching_targets() {
    let project = tempdir().unwrap();
    let external = tempdir().unwrap();
    fs::create_dir_all(external.path().join("onreza-go")).unwrap();
    fs::write(external.path().join("onreza-go/secret.txt"), "outside").unwrap();
    std::os::unix::fs::symlink(external.path(), project.path().join("build")).unwrap();
    assert!(plan::clear_native_build_output(project.path(), "build/onreza-go").is_err());
    assert_eq!(
        fs::read_to_string(external.path().join("onreza-go/secret.txt")).unwrap(),
        "outside"
    );
    fs::remove_file(project.path().join("build")).unwrap();
    fs::create_dir(project.path().join("build")).unwrap();
    std::os::unix::fs::symlink(
        external.path().join("onreza-go"),
        project.path().join("build/onreza-go"),
    )
    .unwrap();
    assert!(plan::clear_native_build_output(project.path(), "build/onreza-go").is_err());
    assert_eq!(
        fs::read_to_string(external.path().join("onreza-go/secret.txt")).unwrap(),
        "outside"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn native_build_replaces_stale_output_without_cleaning_source() {
    use std::os::unix::fs::PermissionsExt;
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("go.mod"),
        "module example.test/app\ngo 1.26\n",
    )
    .unwrap();
    fs::write(
        project.path().join("main.go"),
        "package main\nfunc main() {}\n",
    )
    .unwrap();
    let output_dir = project.path().join("build/onreza-go");
    fs::create_dir_all(output_dir.join("assets")).unwrap();
    fs::write(output_dir.join("assets/stale-secret.txt"), "stale secret").unwrap();
    fs::write(project.path().join("build/keep.txt"), "authored sibling").unwrap();
    let compiler_dir = tempdir().unwrap();
    let pins: serde_json::Value =
        serde_json::from_str(include_str!("../../../assets/native-toolchains.json")).unwrap();
    let compiler = compiler_dir.path().join("go");
    fs::write(&compiler, format!(
        "#!/bin/sh\nset -eu\nif [ \"$1\" = version ]; then\n  echo 'go version go{} linux/amd64'\nelse\n  /bin/mkdir -p build/onreza-go\n  /bin/cp /bin/true build/onreza-go/server\nfi\n", pins["go"].as_str().unwrap(),
    )).unwrap();
    fs::set_permissions(&compiler, fs::Permissions::from_mode(0o755)).unwrap();
    let recipe = crate::detect::native::NativeRecipe::GoServer;
    let native_plan = native_build::recipe_commands(project.path(), recipe, false).unwrap();
    run_native_build_step(
        &native_plan,
        recipe,
        project.path(),
        true,
        &[(
            "PATH".into(),
            compiler_dir.path().to_string_lossy().into_owned(),
        )],
        None,
        false,
    )
    .await
    .unwrap();
    let executable = fs::read(output_dir.join("server")).unwrap();
    assert_eq!(executable, fs::read("/bin/true").unwrap());
    assert_eq!(&executable[..4], b"\x7fELF");
    assert!(!output_dir.join("assets/stale-secret.txt").exists());
    assert_eq!(
        fs::read_to_string(project.path().join("build/keep.txt")).unwrap(),
        "authored sibling"
    );
    assert!(project.path().join("main.go").is_file());
}

// ── resolve_build_command tests ──────────────────────────────

#[test]
fn missing_output_explains_absent_build_command() {
    let error = output::coded_error(
        "MISSING_BUILD_OUTPUT",
        "no output directory found".to_string(),
    );

    let mapped = plan::contextualize_missing_build_output(error, None, false);
    let coded = mapped
        .downcast_ref::<output::CodedError>()
        .expect("missing output must stay coded");

    assert_eq!(coded.code, "MISSING_BUILD_OUTPUT");
    assert!(coded.message.contains("No build command was configured"));
    assert!(coded.message.contains("--build-command"));
    assert!(coded.message.contains("[build].command"));
}

#[test]
fn missing_prebuilt_output_explains_skipped_build() {
    let error = output::coded_error(
        "MISSING_BUILD_OUTPUT",
        "no output directory found".to_string(),
    );

    let mapped = plan::contextualize_missing_build_output(error, Some("bun run build"), true);
    let coded = mapped
        .downcast_ref::<output::CodedError>()
        .expect("missing output must stay coded");

    assert_eq!(coded.code, "MISSING_BUILD_OUTPUT");
    assert!(coded.message.contains("Build execution was skipped"));
    assert!(coded.message.contains("--skip-build"));
}

#[test]
fn build_command_explicit_wins_over_config_and_auto() {
    let dir = node_command_project("{}", Some("yarn.lock"));

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(Some("explicit cmd"), dir.path(), &effective);
    assert_eq!(result.unwrap(), "explicit cmd");
}

#[test]
fn build_command_config_wins_over_auto() {
    let dir = node_command_project("{}", Some("yarn.lock"));

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_auto_detect_bun_lock() {
    let dir = node_command_project(r#"{"scripts":{"build":"vite build"}}"#, Some("bun.lock"));

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_bun_lockb() {
    let dir = node_command_project(r#"{"scripts":{"build":"vite build"}}"#, Some("bun.lockb"));

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_pnpm() {
    let dir = node_command_project(
        r#"{"scripts":{"build":"vite build"}}"#,
        Some("pnpm-lock.yaml"),
    );

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_auto_detect_yarn() {
    let dir = node_command_project(r#"{"scripts":{"build":"vite build"}}"#, Some("yarn.lock"));

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "yarn run build");
}

#[test]
fn build_command_auto_detect_npm_fallback() {
    let dir = node_command_project(r#"{"scripts":{"build":"next build"}}"#, None);

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "npm run build");
}

#[test]
fn build_command_none_without_build_script() {
    let dir = node_command_project(r#"{"scripts":{"dev":"next dev"}}"#, None);

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn build_command_none_without_package_json() {
    let dir = tempdir().unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn recursive_deploy_build_commands_are_classified_as_invalid_config() {
    for command in [
        "nrz deploy",
        "/usr/local/bin/nrz deploy --prod",
        "npx nrz deploy",
        "npx --yes nrz@latest deploy",
        "npx -p nrz nrz deploy",
        "bunx nrz deploy",
        "CI=1 npx nrz deploy",
        "npm ci && npx nrz deploy",
    ] {
        assert!(is_recursive_deploy_command(command), "command: {command}");
    }

    let error = run_build_step("npx nrz deploy", Path::new("."), true, &[], None)
        .expect_err("recursive deploy must be rejected before spawning a child process");
    expect_code(&error, "INVALID_CONFIG");
    assert!(error.to_string().contains("npm run build"));
}

#[tokio::test]
async fn recursive_deploy_install_commands_are_classified_as_invalid_config() {
    let dir = tempdir().unwrap();
    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("npx -p nrz nrz deploy"),
            Some(nrz::config::BuildSettingSource::User),
        ),
    );

    let error = run_install_step(dir.path(), true, &effective, &[], None, false)
        .await
        .expect_err("recursive deploy must be rejected before the install child starts");

    expect_code(&error, "INVALID_CONFIG");
    assert!(error.to_string().contains("npm ci"));
}

// ── resolve_build_command server fallback ────────────────────

#[test]
fn build_command_server_wins_over_auto_detect() {
    let dir = node_command_project("{}", None);

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("server build cmd"), None),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "server build cmd");
}

#[test]
fn build_command_config_wins_over_server() {
    let dir = tempdir().unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("server cmd"), None),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_explicit_wins_over_server() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();

    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("server cmd"), None),
    );
    let result = resolve_build_command(Some("explicit"), dir.path(), &effective);
    assert_eq!(result.unwrap(), "explicit");
}

#[test]
fn build_command_server_used_without_package_json() {
    let dir = tempdir().unwrap();
    // No package.json — auto-detect would return None, but server command should still work
    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("make build"), None),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "make build");
}

#[test]
fn build_command_user_source_used_without_package_json() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(
            Some("make build"),
            Some(nrz::config::BuildSettingSource::User),
        ),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "make build");
}

#[test]
fn build_command_preset_source_without_package_json_skips() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(
            Some("npm run build"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn build_command_preset_source_uses_local_package_manager() {
    let dir = node_command_project(
        r#"{"scripts":{"build":"vite build"}}"#,
        Some("pnpm-lock.yaml"),
    );

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(
            Some("npm run build"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_detected_empty_keeps_auto_detect() {
    let dir = node_command_project(r#"{"scripts":{"build":"vite build"}}"#, None);

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(None, Some(nrz::config::BuildSettingSource::Detected)),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "npm run build");
}

#[test]
fn build_command_user_empty_suppresses_auto_detect() {
    let dir = node_command_project(r#"{"scripts":{"build":"vite build"}}"#, None);

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(None, Some(nrz::config::BuildSettingSource::User)),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn build_command_detected_source_uses_local_package_manager() {
    let dir = node_command_project(
        r#"{"scripts":{"build":"vite build"}}"#,
        Some("pnpm-lock.yaml"),
    );

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(
            Some("npm run build"),
            Some(nrz::config::BuildSettingSource::Detected),
        ),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn platform_runner_build_command_uses_the_immutable_snapshot() {
    let dir = node_command_project(
        r#"{"scripts":{"build":"vite build"}}"#,
        Some("pnpm-lock.yaml"),
    );
    let mut effective = effective_config(dir.path(), nrz::config::ProjectConfig::default());
    effective.apply_platform_runner_settings(&server_build_settings(
        Some("npm run build"),
        Some(nrz::config::BuildSettingSource::Detected),
    ));

    assert_eq!(
        resolve_build_command(None, dir.path(), &effective).as_deref(),
        Some("npm run build")
    );
}

#[test]
fn build_command_preset_empty_keeps_auto_detect_fallback() {
    let dir = node_command_project(r#"{"scripts":{"build":"vite build"}}"#, None);

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(None, Some(nrz::config::BuildSettingSource::Preset)),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "npm run build");
}

// ── install command source handling ──────────────────────────

#[test]
fn install_command_preset_source_without_package_json_skips() {
    let dir = tempdir().unwrap();
    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("npm install"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn install_command_preset_source_uses_local_package_manager() {
    let dir = node_command_project("{}", Some("pnpm-lock.yaml"));

    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("npm install"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm install");
}

#[test]
fn install_command_detected_source_uses_local_package_manager() {
    let dir = node_command_project("{}", Some("pnpm-lock.yaml"));

    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("npm install"),
            Some(nrz::config::BuildSettingSource::Detected),
        ),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm install");
}

#[test]
fn platform_runner_install_command_uses_the_immutable_snapshot() {
    let dir = node_command_project("{}", Some("pnpm-lock.yaml"));
    let mut effective = effective_config(dir.path(), nrz::config::ProjectConfig::default());
    effective.apply_platform_runner_settings(&server_install_settings(
        Some("npm ci"),
        Some(nrz::config::BuildSettingSource::Preset),
    ));

    assert_eq!(
        resolve_install_command(dir.path(), &effective).as_deref(),
        Some("npm ci")
    );
}

#[test]
fn install_command_user_empty_suppresses_auto_detect() {
    let dir = node_command_project("{}", None);

    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(None, Some(nrz::config::BuildSettingSource::User)),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn install_command_user_source_used_without_package_json() {
    let dir = tempdir().unwrap();
    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("make deps"),
            Some(nrz::config::BuildSettingSource::User),
        ),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert_eq!(result.unwrap(), "make deps");
}

#[test]
fn install_command_python_requirements_targets_versioned_site_packages() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
    fs::write(dir.path().join("requirements.txt"), "orjson==3.11.3\n").unwrap();
    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(None, None),
    );

    let result = resolve_install_command(dir.path(), &effective).unwrap();

    assert!(result.starts_with("python3.14 -m pip install"));
    assert!(result.contains("--target .onreza/python/3.14/site-packages"));
    assert!(result.ends_with("--requirement requirements.txt"));
}

#[test]
fn install_command_python_is_not_replaced_by_javascript_metadata() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
    fs::write(dir.path().join("requirements.txt"), "orjson==3.11.3\n").unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(None, None),
    );

    let result = resolve_install_command(dir.path(), &effective).unwrap();

    assert!(result.starts_with("python3.14 -m pip install"));
    assert!(result.ends_with("--requirement requirements.txt"));
}

#[test]
fn configured_python_non_conventional_entry_uses_versioned_install_target() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("run.py"), "print('ready')").unwrap();
    fs::write(dir.path().join("pyproject.toml"), "[project]\nname='app'").unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.project.framework = Some("python".to_string());
    let effective = effective_config(dir.path(), config);

    let result = resolve_install_command(dir.path(), &effective).unwrap();

    assert!(result.starts_with("python3.14 -m pip install"));
    assert!(result.contains("--target .onreza/python/3.14/site-packages"));
    assert!(result.ends_with(" ."));
}

#[tokio::test]
async fn python_install_step_ignores_retained_shell_command_without_manifest() {
    let dir = tempdir().unwrap();
    let stale = dir
        .path()
        .join(".onreza/python/3.14/site-packages/stale.py");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, "stale = True").unwrap();
    fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
    let mut effective = effective_config(dir.path(), nrz::config::ProjectConfig::default());
    effective.apply_platform_runner_settings(&server_install_settings(
        Some("exit 97"),
        Some(nrz::config::BuildSettingSource::Preset),
    ));

    run_install_step(dir.path(), true, &effective, &[], None, true)
        .await
        .unwrap();

    assert!(
        !dir.path()
            .join(".onreza/python/3.14/site-packages")
            .exists()
    );
}

#[tokio::test]
async fn python_install_step_preserves_authored_command_and_dependencies() {
    for platform_runner in [false, true] {
        let (dir, stale) = prepared_python_project();
        let settings = server_install_settings(
            Some("echo authored > installed.txt"),
            Some(nrz::config::BuildSettingSource::User),
        );
        let mut effective = effective_config(dir.path(), nrz::config::ProjectConfig::default());
        if platform_runner {
            effective.apply_platform_runner_settings(&settings);
        } else {
            effective.apply_server_settings(Some(&settings));
        }
        run_install_step(dir.path(), true, &effective, &[], None, platform_runner)
            .await
            .unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("installed.txt"))
                .unwrap()
                .trim(),
            "authored"
        );
        assert!(
            stale.exists(),
            "authored install owns its prepared dependency tree"
        );
    }
}

#[tokio::test]
async fn python_install_step_user_absence_preserves_prepared_dependencies() {
    for platform_runner in [false, true] {
        let (dir, prepared) = prepared_python_project();
        let settings = server_install_settings(None, Some(nrz::config::BuildSettingSource::User));
        let mut effective = effective_config(dir.path(), nrz::config::ProjectConfig::default());
        if platform_runner {
            effective.apply_platform_runner_settings(&settings);
        } else {
            effective.apply_server_settings(Some(&settings));
        }
        run_install_step(dir.path(), true, &effective, &[], None, platform_runner)
            .await
            .unwrap();
        assert!(
            prepared.exists(),
            "explicit install absence must not clean prepared dependencies"
        );
        assert!(resolve_install_command(dir.path(), &effective).is_none());
    }
}

#[test]
fn declared_python_defaults_ignore_javascript_build_tooling() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.py"), "print('ready')\n").unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"vite":"7.0.0"},"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("requirements.txt"), "flask\ngunicorn\n").unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Python);
    config.deploy.application = Some("company.web:app".into());
    config.deploy.server = Some("wsgi".into());
    let effective = effective_config(dir.path(), config.clone());
    assert!(
        resolve_install_command(dir.path(), &effective)
            .unwrap()
            .contains("--requirement requirements.txt")
    );
    assert_eq!(resolve_build_command(None, dir.path(), &effective), None);
    assert_eq!(
        resolve_build_command(Some("npm run assets"), dir.path(), &effective).as_deref(),
        Some("npm run assets")
    );
    let mut node_config = config.clone();
    node_config.build.toolchain = Some(nrz_source_bundle::BuildToolchainFamily::Node);
    let node_effective = effective_config(dir.path(), node_config.clone());
    assert_eq!(
        resolve_install_command(dir.path(), &node_effective).as_deref(),
        Some("npm install")
    );
    assert_eq!(
        resolve_build_command(None, dir.path(), &node_effective).as_deref(),
        Some("npm run build")
    );
    let mut frozen = effective_config(dir.path(), node_config);
    frozen.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
        install_command_source: Some(nrz::config::BuildSettingSource::Preset),
        build_command_source: Some(nrz::config::BuildSettingSource::Preset),
        ..Default::default()
    });
    assert_eq!(resolve_install_command(dir.path(), &frozen), None);
    assert_eq!(resolve_build_command(None, dir.path(), &frozen), None);
    config.build.command = Some("npm run configured-assets".into());
    let effective = effective_config(dir.path(), config);
    assert_eq!(
        resolve_build_command(None, dir.path(), &effective).as_deref(),
        Some("npm run configured-assets")
    );
}
