use super::*;

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
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(Some("explicit cmd"), dir.path(), &effective);
    assert_eq!(result.unwrap(), "explicit cmd");
}

#[test]
fn build_command_config_wins_over_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_auto_detect_bun_lock() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("bun.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_bun_lockb() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("bun.lockb"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_pnpm() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_auto_detect_yarn() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "yarn run build");
}

#[test]
fn build_command_auto_detect_npm_fallback() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"next build"}}"#,
    )
    .unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "npm run build");
}

#[test]
fn build_command_none_without_build_script() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"dev":"next dev"}}"#,
    )
    .unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
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
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

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
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
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
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

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
