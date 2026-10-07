use super::*;

#[test]
fn effective_config_explain_reports_sources() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = ProjectConfig::default();
    config.project.id = Some("proj_123".to_string());
    config.project.framework = Some("vite".to_string());
    config.build.command = Some("pnpm build".to_string());
    config.build.output_directory = Some("dist".to_string());

    let effective = EffectiveProjectConfig::from_project_config(dir.path().to_path_buf(), config);
    let explanation = effective.explain().unwrap();

    assert_eq!(explanation.project_id.value.as_deref(), Some("proj_123"));
    assert_eq!(explanation.project_id.source, "onreza.toml");
    assert_eq!(explanation.framework.value.as_deref(), Some("vite"));
    assert_eq!(explanation.framework.source, "onreza.toml");
    assert_eq!(
        explanation.build_command.value.as_deref(),
        Some("pnpm build")
    );
    assert_eq!(explanation.build_command.source, "onreza.toml");
    assert_eq!(explanation.output_directory.value.as_deref(), Some("dist"));
    assert_eq!(explanation.output_directory.source, "onreza.toml");
}

#[test]
fn effective_config_project_id_override_wins_over_config() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = ProjectConfig::default();
    config.project.id = Some("proj_root".to_string());
    let mut effective =
        EffectiveProjectConfig::from_project_config(dir.path().to_path_buf(), config);

    effective
        .apply_project_id_override(Some("proj_cli"))
        .unwrap();
    let explanation = effective.explain().unwrap();

    assert_eq!(effective.project_id(), Some("proj_cli"));
    assert_eq!(explanation.project_id.value.as_deref(), Some("proj_cli"));
    assert_eq!(explanation.project_id.source, "cli");
}

#[test]
fn effective_config_deploy_app_override_reports_cli_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = ProjectConfig::default();
    config.deploy.app = Some("api".to_string());
    let mut effective =
        EffectiveProjectConfig::from_project_config(dir.path().to_path_buf(), config);

    effective
        .apply_deploy_app_cli_override(Some("web"))
        .unwrap();
    let explanation = effective.explain().unwrap();

    assert_eq!(effective.deploy_app(), Some("web"));
    assert_eq!(explanation.deploy_app.value.as_deref(), Some("web"));
    assert_eq!(explanation.deploy_app.source, "cli");
}

#[test]
fn explained_serving_python_minor_matches_deployment_selection() {
    use crate::detect::application_runtime::resolve_and_bind_source_build_context;
    use nrz_source_bundle::{ApplicationRuntimeFamily, BuildToolchainFamily, PythonMinor};
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.py"), "print('ready')\n").unwrap();
    let fs = crate::detect::fs::LocalFs::new(dir.path());
    for family in [
        None,
        Some(BuildToolchainFamily::Python),
        Some(BuildToolchainFamily::Node),
        Some(BuildToolchainFamily::Bun),
    ] {
        for minor in PythonMinor::ALL {
            for explicit_serving in [false, true] {
                let mut config = ProjectConfig::default();
                config.build.toolchain = family;
                config.build.python_version =
                    (family == Some(BuildToolchainFamily::Python)).then_some(minor);
                config.deploy.runtime =
                    explicit_serving.then_some(ApplicationRuntimeFamily::Python);
                let mut detection = crate::detect::detect(dir.path());
                let context =
                    resolve_and_bind_source_build_context(&fs, &mut detection, &config, None, None)
                        .unwrap();
                let expected = context
                    .application_runtime
                    .and_then(|runtime| runtime.python_version);
                let explanation = EffectiveProjectConfig::from_project_config(
                    dir.path().to_owned(),
                    config.clone(),
                )
                .explain()
                .unwrap();
                assert_eq!(
                    explanation.deploy_python_version.value.as_deref(),
                    expected.map(PythonMinor::version)
                );
                assert_eq!(
                    explanation.deploy_python_version.source,
                    if config.build.python_version.is_some() {
                        "onreza.toml"
                    } else {
                        "default"
                    }
                );
                config.deploy.python_version = Some(minor);
                let explanation =
                    EffectiveProjectConfig::from_project_config(dir.path().to_owned(), config)
                        .explain()
                        .unwrap();
                assert_eq!(
                    explanation.deploy_python_version.value.as_deref(),
                    Some(minor.version())
                );
                assert_eq!(explanation.deploy_python_version.source, "onreza.toml");
            }
        }
    }
    for serving in ["compute='static'", "runtime='node'\nentry='server.js'"] {
        let config: ProjectConfig = toml::from_str(&format!(
            "[build]\ntoolchain='python'\npython_version='3.12'\n[deploy]\n{serving}"
        ))
        .unwrap();
        let explanation =
            EffectiveProjectConfig::from_project_config(dir.path().to_owned(), config)
                .explain()
                .unwrap();
        assert_eq!(
            explanation.build_python_version.value.as_deref(),
            Some("3.12")
        );
        assert_eq!(explanation.deploy_python_version.value, None);
    }
    // Version selection does not require a valid launch or a resolved lock.
    std::fs::remove_file(dir.path().join("main.py")).unwrap();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.poetry]\npackage-mode=false\n",
    )
    .unwrap();
    let config: ProjectConfig =
        toml::from_str("[build]\npython_version='3.12'\n[deploy]\nruntime='python'").unwrap();
    let explanation = EffectiveProjectConfig::from_project_config(dir.path().to_owned(), config)
        .explain()
        .unwrap();
    assert_eq!(
        explanation.deploy_python_version.value.as_deref(),
        Some("3.12")
    );
    assert_eq!(explanation.deploy_python_version.source, "onreza.toml");
}

#[test]
fn explained_python_toolchain_matches_the_shared_build_selection() {
    let dir = tempfile::tempdir().unwrap();
    let input = crate::detect::fs::VirtualFs::from_json(r#"{"tree":[],"files":{}}"#).unwrap();
    let detection = crate::detect::detect_with_fs(&input);
    for minor in nrz_source_bundle::PythonMinor::ALL {
        for explicit in [false, true] {
            let mut config = ProjectConfig::default();
            config.build.python_version = Some(minor);
            config.build.toolchain =
                explicit.then_some(nrz_source_bundle::BuildToolchainFamily::Python);
            let selected =
                crate::detect::application_runtime::resolve_build_toolchain(&detection, &config)
                    .unwrap();
            let effective =
                EffectiveProjectConfig::from_project_config(dir.path().to_owned(), config);
            let explained = effective.explain().unwrap();
            assert_eq!(explained.build_toolchain.value.as_deref(), Some("python"));
            assert_eq!(explained.build_toolchain.source, "onreza.toml");
            assert_eq!(
                explained.build_python_version.value.as_deref(),
                Some(minor.version())
            );
            assert_eq!(explained.build_python_version.source, "onreza.toml");
            assert_eq!(
                selected.family,
                nrz_source_bundle::BuildToolchainFamily::Python
            );
            assert_eq!(selected.resolved_python_minor(), Some(minor));
            assert_eq!(effective.config().build.toolchain.is_some(), explicit);
        }
    }
    for build_family in [
        None,
        Some(nrz_source_bundle::BuildToolchainFamily::Python),
        Some(nrz_source_bundle::BuildToolchainFamily::Node),
        Some(nrz_source_bundle::BuildToolchainFamily::Bun),
        Some(nrz_source_bundle::BuildToolchainFamily::Native),
    ] {
        for minor in nrz_source_bundle::PythonMinor::ALL {
            let mut config = ProjectConfig::default();
            config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Python);
            config.deploy.python_version = Some(minor);
            config.build.toolchain = build_family;
            let selected =
                crate::detect::application_runtime::resolve_build_toolchain(&detection, &config)
                    .unwrap();
            let explained =
                EffectiveProjectConfig::from_project_config(dir.path().to_owned(), config)
                    .explain()
                    .unwrap();
            assert_eq!(
                explained.build_python_version.value.as_deref(),
                selected
                    .resolved_python_minor()
                    .map(nrz_source_bundle::PythonMinor::version),
                "{build_family:?} / {minor:?}"
            );
            assert_eq!(
                explained.build_python_version.source,
                if selected.resolved_python_minor().is_some() {
                    "onreza.toml"
                } else {
                    "default"
                }
            );
        }
    }
    let mut default_python = ProjectConfig::default();
    default_python.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Python);
    let selected =
        crate::detect::application_runtime::resolve_build_toolchain(&detection, &default_python)
            .unwrap();
    let explained =
        EffectiveProjectConfig::from_project_config(dir.path().to_owned(), default_python)
            .explain()
            .unwrap();
    assert_eq!(
        explained.build_python_version.value.as_deref(),
        selected
            .resolved_python_minor()
            .map(nrz_source_bundle::PythonMinor::version)
    );
    assert_eq!(explained.build_python_version.source, "default");
    let empty = EffectiveProjectConfig::from_project_config(
        dir.path().to_owned(),
        ProjectConfig::default(),
    )
    .explain()
    .unwrap();
    assert_eq!(empty.build_toolchain.value, None);
    assert_eq!(empty.build_toolchain.source, "auto");
}
