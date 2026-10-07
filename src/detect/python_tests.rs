use super::fs::{LocalFs, VirtualFs};
use super::python::dependency_manifest;
use super::types::{ComputeType, PackageManagerType, RuntimeType};
use super::{detect, detect_with_fs, resolve_entry_point};

#[test]
fn python_requirement_stage_evidence_does_not_depend_on_package_names() {
    use super::python::{dependency_names, requires_dependency_stage};
    let project = tempfile::tempdir().unwrap();
    let input = LocalFs::new(project.path());
    for (requirements, requires_stage) in [
        ("", false),
        (" \n# runtime dependencies are empty\n", false),
        (
            "--index-url https://packages.example/simple\n--no-index\n",
            false,
        ),
        ("--find-links \\\n ./wheels\n-c constraints.txt\n", false),
        ("packaging==26.3\n", true),
        ("-r production.txt\n", true),
        ("--requirement=production.txt\n", true),
        ("-e ./localproject\n", true),
        ("--editable=./localproject\n", true),
        ("./wheels/acme-1.0-py3-none-any.whl\n", true),
        ("../localproject\n", true),
        ("/opt/localproject\n", true),
        ("localproject/\n", true),
        ("https://packages.example/acme.whl\n", true),
        ("git+https://git.example/acme.git\n", true),
    ] {
        std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
        assert_eq!(
            requires_dependency_stage(&input).unwrap(),
            requires_stage,
            "{requirements:?}"
        );
    }
    for requirement in [
        "./fastapi/",
        "fastapi/",
        "fastapi.whl",
        "https://packages.example/fastapi.whl",
        "git+https://git.example/fastapi.git",
    ] {
        std::fs::write(project.path().join("requirements.txt"), requirement).unwrap();
        assert!(
            dependency_names(&input).unwrap().is_empty(),
            "{requirement}"
        );
        assert_eq!(super::python::framework(&input).unwrap(), "python");
    }
    std::fs::write(
        project.path().join("requirements.txt"),
        "fastapi[standard]>=0.100\nFlask @ https://packages.example/flask.whl\n",
    )
    .unwrap();
    assert_eq!(
        dependency_names(&input)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>(),
        ["fastapi", "flask"]
    );
    std::fs::remove_file(project.path().join("requirements.txt")).unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\ndependencies=['acme.whl']\n",
    )
    .unwrap();
    assert!(requires_dependency_stage(&input).unwrap());
}

#[test]
fn inactive_requirements_do_not_create_a_locked_python_dependency_stage() {
    use super::python::requires_dependency_stage;
    let project = tempfile::tempdir().unwrap();
    let input = LocalFs::new(project.path());
    for (lock, manifest) in [
        (
            "uv.lock",
            "[project]\nname='demo'\ndependencies=[]\n[tool.uv]\npackage=false\n",
        ),
        (
            "poetry.lock",
            "[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\npython='^3.14'\n",
        ),
    ] {
        std::fs::write(project.path().join(lock), "").unwrap();
        std::fs::write(project.path().join("pyproject.toml"), manifest).unwrap();
        for requirements in [
            "-r build-helper.txt\n",
            "-e ./build-helper\n",
            "./build-helper\n",
        ] {
            std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
            assert!(
                !requires_dependency_stage(&input).unwrap(),
                "{lock}: {requirements}"
            );
        }
        std::fs::remove_file(project.path().join(lock)).unwrap();
    }
}

#[test]
fn detects_requirements_project_as_python_process() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
    std::fs::write(dir.path().join("requirements.txt"), "orjson==3.11.3\n").unwrap();

    let result = detect(dir.path());

    assert_eq!(result.framework, "python");
    assert_eq!(result.suggested_compute, ComputeType::Process);
    assert_eq!(result.metadata.runtime.runtime_type, RuntimeType::Python);
    assert_eq!(
        result.metadata.runtime.version.as_deref(),
        Some(nrz_source_bundle::PythonMinor::default().version())
    );
    let package_manager = result.metadata.package_manager.unwrap();
    assert_eq!(package_manager.pm_type, PackageManagerType::Pip);
    assert_eq!(
        package_manager.lockfile.as_deref(),
        Some("requirements.txt")
    );
    assert_eq!(
        result.metadata.build_info.unwrap().entry_point.as_deref(),
        Some("main.py")
    );
}

#[test]
fn detects_dependency_free_python_process() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/server.py"), "print('ready')").unwrap();

    let result = detect(dir.path());

    assert_eq!(result.framework, "python");
    assert_eq!(
        result.metadata.package_manager.unwrap().pm_type,
        PackageManagerType::Pip
    );
    assert_eq!(
        resolve_entry_point("python", dir.path(), dir.path()).as_deref(),
        Some("src/server.py")
    );
}

#[test]
fn python_manifest_without_entry_does_not_override_javascript() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("requirements.txt"), "httpx==0.28.1\n").unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"express":"5.1.0"}}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("server.js"), "require('express')").unwrap();

    let result = detect(dir.path());

    assert_eq!(result.framework, "express");
    assert_ne!(result.metadata.runtime.runtime_type, RuntimeType::Python);
}

#[test]
fn incidental_python_entry_without_manifest_does_not_override_javascript() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.py"), "print('tooling')").unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"express":"5.1.0"}}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("server.js"), "require('express')").unwrap();

    let result = detect(dir.path());

    assert_eq!(result.framework, "express");
    assert_ne!(result.metadata.runtime.runtime_type, RuntimeType::Python);
}

#[test]
fn configured_python_supports_an_explicit_non_conventional_entry() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("run.py"), "print('ready')").unwrap();
    std::fs::write(dir.path().join("pyproject.toml"), "[project]\nname='app'").unwrap();

    let result = super::detect_with_framework_override(dir.path(), Some("python"));

    assert_eq!(result.framework, "python");
    assert_eq!(result.metadata.runtime.runtime_type, RuntimeType::Python);
    assert_eq!(
        result.metadata.package_manager.unwrap().pm_type,
        PackageManagerType::Pip
    );
    assert!(result.metadata.build_info.unwrap().entry_point.is_none());
}

#[test]
fn stdin_manifest_carries_python_detection_inputs() {
    let fs = VirtualFs::from_json(
        r#"{"tree":["pyproject.toml","app.py"],"files":{"pyproject.toml":"[project]\nname='app'","app.py":"print('ready')"}}"#,
    )
    .unwrap();

    let result = detect_with_fs(&fs);

    assert_eq!(result.framework, "python");
    assert_eq!(dependency_manifest(&fs), Some("pyproject.toml"));
}

#[test]
fn local_dependency_manifest_ignores_unsupported_pipfile() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
    std::fs::write(dir.path().join("Pipfile"), "[packages]\n").unwrap();

    assert_eq!(dependency_manifest(&LocalFs::new(dir.path())), None);
}

#[test]
fn python_manifest_only_and_django_projects_are_processes() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\nversion='1.0.0'\n",
    )
    .unwrap();
    let result = detect(project.path());
    assert_eq!(result.framework, "python");
    assert_eq!(result.suggested_compute, ComputeType::Process);
    std::fs::write(project.path().join("manage.py"), "").unwrap();
    assert_eq!(detect(project.path()).framework, "django");
}

#[test]
fn python_uv_and_poetry_locks_have_explicit_package_manager_identity() {
    use super::python::{PythonDependencyKind, dependency_plan};
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\nversion='1.0.0'\ndependencies=['fastapi[standard]>=0.100']\n",
    )
    .unwrap();
    std::fs::write(project.path().join("requirements.txt"), "unrelated==1\n").unwrap();
    for (file, kind, pm) in [
        ("uv.lock", PythonDependencyKind::Uv, PackageManagerType::Uv),
        (
            "poetry.lock",
            PythonDependencyKind::Poetry,
            PackageManagerType::Poetry,
        ),
    ] {
        std::fs::write(project.path().join(file), "").unwrap();
        assert_eq!(
            dependency_plan(&LocalFs::new(project.path()))
                .unwrap()
                .unwrap()
                .kind,
            kind
        );
        let result = detect(project.path());
        assert_eq!(result.framework, "fastapi");
        assert_eq!(result.metadata.package_manager.unwrap().pm_type, pm);
        assert!(
            result
                .metadata
                .build_info
                .unwrap()
                .install_command
                .is_none()
        );
        std::fs::remove_file(project.path().join(file)).unwrap();
    }
    std::fs::write(project.path().join("uv.lock"), "").unwrap();
    std::fs::write(project.path().join("poetry.lock"), "").unwrap();
    assert!(
        dependency_plan(&LocalFs::new(project.path()))
            .unwrap_err()
            .to_string()
            .contains("choose one")
    );
}

#[test]
fn python_stage_markers_use_the_frozen_target_and_keep_unknown_inputs_conservative() {
    use super::python::{requires_dependency_stage, requires_dependency_stage_for_target};
    use nrz_source_bundle::PythonMinor;
    let project = tempfile::tempdir().unwrap();
    let fs = LocalFs::new(project.path());
    for minor in PythonMinor::ALL {
        for (requirement, required) in [
            ("colorama; sys_platform == 'win32'", false),
            ("colorama; os_name == 'nt'", false),
            ("colorama; platform_machine == 'aarch64'", false),
            ("colorama; platform_system != 'Linux'", false),
            ("colorama; implementation_name != 'cpython'", false),
            (
                "colorama; platform_python_implementation != 'CPython'",
                false,
            ),
            ("colorama; sys_platform == 'linux'", true),
            ("colorama; platform_release == 'UNKNOWN_KERNEL'", true),
            ("colorama; 'UNKNOWN_KERNEL' in platform_version", true),
            ("colorama; extra == 'feature'", true),
            ("colorama; platform_release in 'UNKNOWN_KERNEL'", true),
            ("colorama; unexpected_variable == 'x'", true),
            (
                "colorama; sys_platform == 'win32' or python_version >= '3.0'",
                true,
            ),
            (
                "colorama; sys_platform == 'win32' and python_version >= '3.0'",
                false,
            ),
            ("../localproject", true),
            ("-r included.txt", true),
            ("-e ../localproject", true),
        ] {
            std::fs::write(project.path().join("requirements.txt"), requirement).unwrap();
            assert!(requires_dependency_stage(&fs).unwrap());
            assert_eq!(
                requires_dependency_stage_for_target(&fs, minor).unwrap(),
                required,
                "{minor:?}: {requirement}"
            );
        }
        for key in ["python_full_version", "implementation_version"] {
            for (operator, required) in [("==", true), ("!=", false)] {
                let requirement = format!("colorama; {key} {operator} '{}'", minor.exact_version());
                std::fs::write(project.path().join("requirements.txt"), &requirement).unwrap();
                assert_eq!(
                    requires_dependency_stage_for_target(&fs, minor).unwrap(),
                    required,
                    "{requirement}"
                );
            }
        }
        for (operator, required) in [("==", true), ("!=", false)] {
            let requirement = format!("colorama; python_version {operator} '{}'", minor.version());
            std::fs::write(project.path().join("requirements.txt"), &requirement).unwrap();
            assert_eq!(
                requires_dependency_stage_for_target(&fs, minor).unwrap(),
                required,
                "{requirement}"
            );
        }
        for (marker, required) in [
            ("python_version in '0.0 1.0'".to_string(), false),
            (format!("python_version in '{}'", minor.version()), true),
            (
                format!("python_version not in '{}'", minor.version()),
                false,
            ),
            ("python_version not in '0.0 1.0'".to_string(), true),
            (
                "python_version in '0.0 1.0' or sys_platform == 'win32'".to_string(),
                false,
            ),
            (
                format!(
                    "python_version in '{}' and sys_platform == 'win32'",
                    minor.version()
                ),
                false,
            ),
            (
                format!(
                    "python_version not in '{}' or sys_platform == 'win32'",
                    minor.version()
                ),
                false,
            ),
        ] {
            let requirement = format!("colorama; {marker}");
            std::fs::write(project.path().join("requirements.txt"), &requirement).unwrap();
            assert_eq!(
                requires_dependency_stage_for_target(&fs, minor).unwrap(),
                required,
                "{minor:?}: {requirement}"
            );
        }
    }
    std::fs::remove_file(project.path().join("requirements.txt")).unwrap();
    for (project_metadata, required) in [
        (
            "[project]\nname='app'\ndependencies=['colorama; sys_platform == \"win32\"']\n",
            false,
        ),
        (
            "[project]\nname='app'\ndependencies=['colorama; sys_platform == \"win32\"']\ndynamic=['dependencies']\n",
            true,
        ),
        (
            "[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n[tool.uv]\npackage=false\n",
            true,
        ),
    ] {
        std::fs::write(project.path().join("pyproject.toml"), project_metadata).unwrap();
        assert!(requires_dependency_stage(&fs).unwrap());
        for minor in PythonMinor::ALL {
            assert_eq!(
                requires_dependency_stage_for_target(&fs, minor).unwrap(),
                required
            );
        }
    }
    std::fs::write(project.path().join("pyproject.toml"), "[tool.poetry]\nname='app'\npackage-mode=false\n[tool.poetry.dependencies]\npython='*'\ncolorama={version='*', markers='sys_platform == \"win32\"'}\n").unwrap();
    std::fs::write(project.path().join("poetry.lock"), "").unwrap();
    assert!(requires_dependency_stage(&fs).unwrap());
    for minor in PythonMinor::ALL {
        assert!(!requires_dependency_stage_for_target(&fs, minor).unwrap());
    }
    for minor in PythonMinor::ALL {
        for (selectors, required) in [
            ("platform='win32'".to_string(), false),
            ("platform='linux'".to_string(), true),
            ("platform='freebsd'".to_string(), false),
            ("platform='aix'".to_string(), false),
            ("platform='custom_OS-123'".to_string(), false),
            ("platform='Linux'".to_string(), false),
            ("platform=''".to_string(), true),
            ("platform=' linux '".to_string(), true),
            ("platform='*'".to_string(), true),
            ("platform='linux win32'".to_string(), true),
            ("python='<3.12'".to_string(), false),
            (format!("python='=={}'", minor.exact_version()), true),
            (format!("python='!={}'", minor.exact_version()), false),
            ("python='>=3.12', platform='win32'".to_string(), false),
            ("python='<3.12', platform='linux'".to_string(), false),
            ("python='>=3.12', platform='linux'".to_string(), true),
            (format!("python='=={}'", minor.version()), true),
            (format!("python='!={}'", minor.version()), true),
            (format!("python='<={}'", minor.version()), false),
            (format!("python='>{}'", minor.version()), true),
            (format!("python='<{}'", minor.version()), false),
            (format!("python='>={}'", minor.version()), true),
            ("python='==2.7'".to_string(), false),
            ("python='>=3.12,<3.15'".to_string(), true),
            ("python='^2.7'".to_string(), true),
            ("python='~=2.7'".to_string(), true),
            ("python='==3.12.*'".to_string(), true),
            ("python='UNKNOWN_CONSTRAINT'".to_string(), true),
            ("platform='!=linux'".to_string(), true),
            ("platform='linux || win32'".to_string(), true),
            ("platform=42".to_string(), true),
            ("python=42".to_string(), true),
            (
                "platform='win32', markers='sys_platform == \"linux\"'".to_string(),
                false,
            ),
            (
                "platform='win32', markers='platform_release == \"UNKNOWN_KERNEL\"'".to_string(),
                true,
            ),
        ] {
            std::fs::write(project.path().join("pyproject.toml"), format!("[tool.poetry]\nname='app'\npackage-mode=false\n[tool.poetry.dependencies]\npython='*'\ncolorama={{version='*', {selectors}}}\n")).unwrap();
            assert!(requires_dependency_stage(&fs).unwrap());
            assert_eq!(
                requires_dependency_stage_for_target(&fs, minor).unwrap(),
                required,
                "{minor:?}: {selectors}"
            );
        }
        for (alternatives, required) in [
            (
                "[{version='*', platform='win32'}, {version='*', platform='darwin'}]",
                false,
            ),
            (
                "[{version='*', platform='win32'}, {version='*', platform='linux'}]",
                true,
            ),
            (
                "[{version='*', platform='win32'}, {version='*', python='^2.7'}]",
                true,
            ),
        ] {
            std::fs::write(project.path().join("pyproject.toml"), format!("[tool.poetry]\nname='app'\npackage-mode=false\n[tool.poetry.dependencies]\npython='*'\ncolorama={alternatives}\n")).unwrap();
            assert_eq!(
                requires_dependency_stage_for_target(&fs, minor).unwrap(),
                required,
                "{minor:?}: {alternatives}"
            );
        }
    }
}

#[test]
fn unpackaged_python_src_imports_do_not_request_project_installation() {
    use super::python::{
        PythonDependencyKind, dependency_names, dependency_plan, requires_dependency_stage,
    };
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("src/demo")).unwrap();
    std::fs::write(project.path().join("src/demo/__init__.py"), "VALUE=42\n").unwrap();
    std::fs::write(
        project.path().join("src/main.py"),
        "import demo; print(demo.VALUE)\n",
    )
    .unwrap();
    let input = LocalFs::new(project.path());
    for (requirements, requires_stage, names) in [
        ("", false, vec![]),
        ("# no dependencies\n", false, vec![]),
        ("packaging==26.3\n", true, vec!["packaging"]),
        ("./wheels/acme-1.0-py3-none-any.whl\n", true, vec![]),
    ] {
        std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
        let plan = dependency_plan(&input).unwrap().unwrap();
        assert_eq!(plan.kind, PythonDependencyKind::Requirements);
        assert!(plan.project_name.is_none());
        assert!(
            !plan.install_project,
            "import layout without packaging metadata requested own-project installation"
        );
        assert_eq!(requires_dependency_stage(&input).unwrap(), requires_stage);
        assert_eq!(
            dependency_names(&input)
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            names
        );
        let mut detection = detect(project.path());
        super::application_runtime::resolve_and_bind_detection(&input, &mut detection).unwrap();
        let runtime = detection
            .metadata
            .source_build_context
            .unwrap()
            .application_runtime
            .unwrap();
        assert_eq!(
            runtime.family,
            nrz_source_bundle::ApplicationRuntimeFamily::Python
        );
        assert_eq!(runtime.entry.as_deref(), Some("src/main.py"));
    }
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[tool.ruff]\nline-length=88\n",
    )
    .unwrap();
    assert!(
        !dependency_plan(&input).unwrap().unwrap().install_project,
        "tool-only pyproject must not supply package metadata"
    );
}

#[test]
fn python_src_package_is_materialized_but_nonpackage_poetry_is_not() {
    use super::python::dependency_plan;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("src/demo")).unwrap();
    std::fs::write(project.path().join("src/demo/__init__.py"), "").unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\nversion='1.0.0'\n",
    )
    .unwrap();
    assert!(
        dependency_plan(&LocalFs::new(project.path()))
            .unwrap()
            .unwrap()
            .install_project
    );
    std::fs::write(project.path().join("pyproject.toml"), "[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\npython='^3.14'\nflask='*'\n").unwrap();
    assert!(dependency_plan(&LocalFs::new(project.path())).is_err());
    std::fs::write(project.path().join("poetry.lock"), "").unwrap();
    assert!(
        !dependency_plan(&LocalFs::new(project.path()))
            .unwrap()
            .unwrap()
            .install_project
    );
    assert_eq!(detect(project.path()).framework, "flask");
}

#[test]
fn python_tool_only_pyproject_does_not_hide_static_html() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[tool.ruff]\nline-length=88\n",
    )
    .unwrap();
    std::fs::write(project.path().join("index.html"), "<html></html>").unwrap();
    assert_eq!(detect(project.path()).framework, "static-html");
}

#[test]
fn configured_python_framework_and_lock_owner_ignore_incidental_requirements() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.py"), "").unwrap();
    assert_eq!(
        super::python::detect_configured_python_framework(&LocalFs::new(project.path()), "fastapi")
            .framework,
        "fastapi"
    );
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\ndependencies=['starlette']\n",
    )
    .unwrap();
    std::fs::write(project.path().join("requirements.txt"), "flask\n").unwrap();
    std::fs::write(project.path().join("uv.lock"), "").unwrap();
    assert_eq!(detect(project.path()).framework, "starlette");
}

#[test]
fn python_requirements_project_keeps_its_setup_package() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("requirements.txt"), "").unwrap();
    std::fs::write(
        project.path().join("setup.py"),
        "from setuptools import setup; setup(name='demo')",
    )
    .unwrap();
    let plan = super::python::dependency_plan(&LocalFs::new(project.path()))
        .unwrap()
        .unwrap();
    assert_eq!(plan.kind, super::python::PythonDependencyKind::Requirements);
    assert!(plan.install_project);
}

#[test]
fn python_optional_poetry_framework_does_not_select_an_uninstalled_server() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("pyproject.toml"), "[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\npython='^3.14'\nflask={version='*',optional=true}\n").unwrap();
    std::fs::write(project.path().join("poetry.lock"), "").unwrap();
    assert_eq!(
        super::python::framework(&LocalFs::new(project.path())).unwrap(),
        "python"
    );
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\ndependencies=[]\n[tool.poetry.dependencies]\nflask='*'\n",
    )
    .unwrap();
    assert_eq!(
        super::python::framework(&LocalFs::new(project.path())).unwrap(),
        "python"
    );
}

#[test]
fn python_package_mode_belongs_to_the_selected_lock_owner() {
    let project = tempfile::tempdir().unwrap();
    for (owner, other_configuration) in [
        ("poetry.lock", "[tool.uv]\npackage=false"),
        (
            "uv.lock",
            "[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\npython='<3.14'",
        ),
    ] {
        std::fs::write(project.path().join("pyproject.toml"), format!("[project]\nname='demo'\nversion='1.0'\n[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n{other_configuration}\n")).unwrap();
        std::fs::write(project.path().join(owner), "").unwrap();
        let plan = super::python::dependency_plan(&LocalFs::new(project.path()))
            .unwrap()
            .unwrap();
        assert!(plan.install_project, "{owner} owns package mode");
        if owner == "uv.lock" {
            assert!(
                plan.poetry_requires_python.is_none(),
                "uv must not inherit another owner's Python constraint"
            );
        }
        std::fs::remove_file(project.path().join(owner)).unwrap();
    }
}

#[test]
fn python_minor_selection_is_frozen_but_not_part_of_normalized_launch_intent() {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "print('ready')").unwrap();
        std::fs::write(
            dir.path().join("onreza.toml"),
            format!(
                "[deploy]\nruntime='python'\npython_version='{}'\n",
                minor.version()
            ),
        )
        .unwrap();
        let mut detection = detect(dir.path());
        super::application_runtime::resolve_and_bind_detection(
            &LocalFs::new(dir.path()),
            &mut detection,
        )
        .unwrap();
        let declaration = detection
            .metadata
            .source_build_context
            .unwrap()
            .application_runtime
            .unwrap();
        assert_eq!(declaration.python_version, Some(minor));
        assert_eq!(
            detection.metadata.runtime.version.as_deref(),
            Some(minor.version())
        );
        let wire = serde_json::to_value(&declaration).unwrap();
        assert_eq!(wire["pythonVersion"], minor.version());
        let normalized = serde_json::to_value(declaration.intent()).unwrap();
        assert!(normalized.get("pythonVersion").is_none());
        assert!(normalized.get("entry").is_none());
    }
    for value in ["3.11", "3.15", "3.14.8"] {
        assert!(
            serde_json::from_value::<nrz_source_bundle::ApplicationRuntimeDeclaration>(
                serde_json::json!({"family":"PYTHON", "pythonVersion":value, "args":[]})
            )
            .is_err()
        );
    }
    let declaration: nrz_source_bundle::ApplicationRuntimeDeclaration = serde_json::from_value(
        serde_json::json!({"family":"NODE", "pythonVersion":"3.12", "args":[]}),
    )
    .unwrap();
    assert!(declaration.validate().is_err());
}

#[test]
fn declared_python_launch_replaces_incidental_javascript_defaults() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("package.json"),
        r#"{"devDependencies":{"vite":"7.0.0"},"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='company'\ndependencies=['fastapi','uvicorn']\n",
    )
    .unwrap();
    std::fs::write(project.path().join("onreza.toml"), "[deploy]\nruntime='python'\npython_version='3.12'\napplication='company.web:app'\nserver='asgi'\n").unwrap();
    let mut detection = detect(project.path());
    super::application_runtime::resolve_and_bind_detection(
        &LocalFs::new(project.path()),
        &mut detection,
    )
    .unwrap();
    assert_eq!(detection.framework, "fastapi");
    assert_eq!(detection.suggested_compute, ComputeType::Process);
    assert_eq!(detection.metadata.runtime.version.as_deref(), Some("3.12"));
    let build = detection.metadata.build_info.unwrap();
    assert_eq!(build.build_command, None);
    assert_eq!(build.output_dir.as_deref(), Some("."));
    assert!(
        build
            .install_command
            .as_deref()
            .unwrap()
            .starts_with("python3.12 ")
    );
    assert!(
        !build
            .install_command
            .as_deref()
            .unwrap_or_default()
            .contains("npm")
    );
}

#[test]
fn python_requirements_continuations_preserve_framework_launch_and_opacity() {
    use super::python::{dependency_names, framework_evidence_complete, requires_dependency_stage};
    use super::python_launch::{PythonLaunchRequest, resolve_launch_for_framework};
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.py"), "app = object()\n").unwrap();
    let input = LocalFs::new(project.path());
    let request = || PythonLaunchRequest {
        entry: None,
        module: None,
        application: None,
        server: None,
        args: &[],
    };
    for requirements in [
        "fastapi\\\n>=0.100\n",
        "fastapi \\\n >=0.100\n",
        "fastapi\\\r\n>=0.100\r\n",
        "# comment \\\nfastapi>=0.100\n",
        "fastapi>=0.100 \\\n --hash=sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
    ] {
        std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
        assert!(
            requires_dependency_stage(&input).unwrap(),
            "{requirements:?}"
        );
        assert!(
            dependency_names(&input).unwrap().contains("fastapi"),
            "{requirements:?}"
        );
        assert!(framework_evidence_complete(&input).unwrap());
        let launch = resolve_launch_for_framework(&input, request(), None)
            .unwrap()
            .unwrap();
        assert_eq!(launch.args, ["ASGI", "main:app"], "{requirements:?}");
    }
    for requirements in [
        "fastapi\\\n>=0.100; python_version < '3.13'\n",
        "fastapi>=0.100 \\\n; python_version < '3.13'\n",
        "fastapi\\\r\n>=0.100; python_version < '3.13'\r\n",
    ] {
        std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
        assert!(requires_dependency_stage(&input).unwrap());
        assert!(
            !framework_evidence_complete(&input).unwrap(),
            "{requirements:?}"
        );
        assert!(resolve_launch_for_framework(&input, request(), None).is_err());
    }
    std::fs::write(
        project.path().join("requirements.txt"),
        "# comment \\\n# no dependencies\n",
    )
    .unwrap();
    assert!(!requires_dependency_stage(&input).unwrap());
    assert!(dependency_names(&input).unwrap().is_empty());
}

#[test]
fn python_pyproject_framework_evidence_distinguishes_static_and_backend_metadata() {
    use super::python::{framework, framework_evidence_complete};
    use super::python_launch::{PythonLaunchRequest, resolve_launch_for_framework};
    for (metadata, package_file, lock, expected) in [
        (
            "[project]\nname='demo'\nversion='1.0'\ndynamic=['dependencies']\n[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n",
            Some("setup.cfg"),
            None,
            None,
        ),
        (
            "[project]\nname='demo'\ndynamic=['dependencies']\n[tool.uv]\npackage=false\n",
            None,
            None,
            None,
        ),
        (
            "[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n",
            Some("setup.cfg"),
            None,
            None,
        ),
        (
            "[tool.ruff]\nline-length=88\n",
            Some("setup.py"),
            None,
            None,
        ),
        (
            "[project]\nname='demo'\nversion='1.0'\n[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n",
            Some("setup.cfg"),
            None,
            Some("python"),
        ),
        (
            "[project]\nname='demo'\ndynamic=['version']\n",
            None,
            None,
            Some("python"),
        ),
        (
            "[project]\nname='demo'\ndependencies=[]\n",
            None,
            None,
            Some("python"),
        ),
        (
            "[project]\nname='demo'\ndependencies=['fastapi']\n",
            None,
            None,
            Some("fastapi"),
        ),
        ("[tool.ruff]\nline-length=88\n", None, None, Some("python")),
        (
            "[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\npython='^3.14'\nflask='*'\n",
            None,
            Some("poetry.lock"),
            Some("flask"),
        ),
        (
            "[project]\nname='demo'\ndynamic=['dependencies']\n[tool.poetry.dependencies]\npython='^3.14'\nflask='*'\n",
            None,
            Some("poetry.lock"),
            Some("flask"),
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("pyproject.toml"), metadata).unwrap();
        std::fs::write(project.path().join("main.py"), "app = object()\n").unwrap();
        if let Some(file) = package_file {
            std::fs::write(
                project.path().join(file),
                if file == "setup.py" {
                    "raise RuntimeError('detection must not execute backend metadata')\n"
                } else {
                    "[options]\ninstall_requires =\n    fastapi\n"
                },
            )
            .unwrap();
        }
        if let Some(lock) = lock {
            std::fs::write(project.path().join(lock), "").unwrap();
        }
        let input = LocalFs::new(project.path());
        assert_eq!(
            framework_evidence_complete(&input).unwrap(),
            expected.is_some(),
            "{metadata}"
        );
        let request = || PythonLaunchRequest {
            entry: None,
            module: None,
            application: None,
            server: None,
            args: &[],
        };
        if let Some(expected) = expected {
            assert_eq!(framework(&input).unwrap(), expected, "{metadata}");
            assert!(
                resolve_launch_for_framework(&input, request(), None)
                    .unwrap()
                    .is_some()
            );
        } else {
            assert!(
                resolve_launch_for_framework(&input, request(), None).is_err(),
                "{metadata}"
            );
            assert!(
                resolve_launch_for_framework(
                    &input,
                    PythonLaunchRequest {
                        module: Some("main"),
                        ..request()
                    },
                    None
                )
                .unwrap()
                .is_some()
            );
            assert!(
                resolve_launch_for_framework(
                    &input,
                    PythonLaunchRequest {
                        application: Some("main:app"),
                        server: Some("asgi"),
                        ..request()
                    },
                    None
                )
                .unwrap()
                .is_some()
            );
        }
    }
}

#[test]
fn opaque_python_pyproject_dependencies_require_a_stage_without_package_installation() {
    use super::python::{dependency_plan, requires_dependency_stage};
    let project = tempfile::tempdir().unwrap();
    let input = LocalFs::new(project.path());
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\ndynamic=['dependencies']\n[tool.uv]\npackage=false\n",
    )
    .unwrap();
    assert!(!dependency_plan(&input).unwrap().unwrap().install_project);
    assert!(requires_dependency_stage(&input).unwrap());
    for metadata in [
        "[project]\nname='demo'\nversion='1.0'\n",
        "[project]\nname='demo'\ndependencies=[]\n",
        "[tool.ruff]\nline-length=88\n",
        "[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\npython='^3.14'\n",
    ] {
        std::fs::write(project.path().join("pyproject.toml"), metadata).unwrap();
        if metadata.contains("tool.poetry") {
            std::fs::write(project.path().join("poetry.lock"), "").unwrap();
        }
        assert!(!requires_dependency_stage(&input).unwrap(), "{metadata}");
    }
}

#[test]
fn incomplete_python_dependencies_require_an_authored_launch() {
    use super::python_launch::{PythonLaunchRequest, resolve_launch_for_framework};
    for (manifest, metadata) in [
        (
            "requirements.txt",
            "fastapi; python_version < '3.13'\nflask\ngunicorn\n",
        ),
        ("requirements.txt", "-r requirements/base.txt\n"),
        (
            "setup.py",
            "from setuptools import setup\nsetup(name='demo', install_requires=['fastapi'])\n",
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        let marker = project.path().join("setup-executed");
        let metadata = if manifest == "setup.py" {
            format!(
                "from pathlib import Path\nPath({:?}).write_text('DETECT_EXECUTED_SETUP')\n{metadata}",
                marker.to_str().unwrap()
            )
        } else {
            metadata.to_string()
        };
        std::fs::write(project.path().join(manifest), metadata).unwrap();
        std::fs::write(
            project.path().join("main.py"),
            "from flask import Flask\napp=Flask(__name__)\n",
        )
        .unwrap();
        let fs = LocalFs::new(project.path());
        let result = resolve_launch_for_framework(
            &fs,
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: None,
                server: None,
                args: &[],
            },
            None,
        );
        assert!(
            result.is_err(),
            "incomplete dependencies inferred a launch: {result:?}"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("framework inference")
        );
        assert!(
            resolve_launch_for_framework(
                &fs,
                PythonLaunchRequest {
                    entry: None,
                    module: None,
                    application: Some("main:app"),
                    server: None,
                    args: &[]
                },
                None
            )
            .unwrap_err()
            .to_string()
            .contains("framework inference")
        );
        let callable = resolve_launch_for_framework(
            &fs,
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: Some("main:app"),
                server: None,
                args: &[],
            },
            Some("python"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(callable.args, ["CALLABLE", "main:app"]);
        let declared = resolve_launch_for_framework(
            &fs,
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: Some("main:app"),
                server: Some("wsgi"),
                args: &[],
            },
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(declared.args, ["WSGI", "main:app"]);
        let preset = resolve_launch_for_framework(
            &fs,
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: None,
                server: None,
                args: &[],
            },
            Some("flask"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(preset.args, ["WSGI", "main:app"]);
        for (entry, module) in [(Some("main.py"), None), (None, Some("main"))] {
            let launch = resolve_launch_for_framework(
                &fs,
                PythonLaunchRequest {
                    entry,
                    module,
                    application: None,
                    server: None,
                    args: &[],
                },
                None,
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                launch.entry,
                if entry.is_some() {
                    "main.py"
                } else {
                    ".onreza/python/launch.py"
                }
            );
        }
        assert!(
            !marker.exists(),
            "detection executed dynamic setup metadata"
        );
    }
}

#[test]
fn python_framework_evidence_keeps_comments_and_django_management_distinct() {
    use super::python_launch::{PythonLaunchRequest, resolve_launch_for_framework};
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("requirements.txt"),
        "flask # deploy; production\ngunicorn\n",
    )
    .unwrap();
    std::fs::write(project.path().join("main.py"), "app = object()\n").unwrap();
    let fs = LocalFs::new(project.path());
    let request = || PythonLaunchRequest {
        entry: None,
        module: None,
        application: None,
        server: None,
        args: &[],
    };
    assert_eq!(
        resolve_launch_for_framework(&fs, request(), None)
            .unwrap()
            .unwrap()
            .args,
        ["WSGI", "main:app"]
    );
    std::fs::write(
        project.path().join("requirements.txt"),
        "-r requirements/base.txt\nfastapi; python_version < '3.13'\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join("manage.py"),
        "# Django management entry\n",
    )
    .unwrap();
    std::fs::create_dir(project.path().join("company")).unwrap();
    std::fs::write(
        project.path().join("company/wsgi.py"),
        "application=object()\n",
    )
    .unwrap();
    std::fs::write(
        project.path().join("company/asgi.py"),
        "application=object()\n",
    )
    .unwrap();
    assert_eq!(
        resolve_launch_for_framework(&fs, request(), None)
            .unwrap()
            .unwrap()
            .args,
        ["WSGI", "company.wsgi:application"]
    );
    assert_eq!(
        resolve_launch_for_framework(
            &fs,
            PythonLaunchRequest {
                server: Some("asgi"),
                ..request()
            },
            None
        )
        .unwrap()
        .unwrap()
        .args,
        ["ASGI", "company.asgi:application"]
    );
    // Concrete Django source evidence is independent of opaque package metadata.
    std::fs::remove_file(project.path().join("requirements.txt")).unwrap();
    std::fs::write(
        project.path().join("setup.py"),
        "raise RuntimeError('never execute for detection')\n",
    )
    .unwrap();
    assert_eq!(
        resolve_launch_for_framework(&fs, request(), None)
            .unwrap()
            .unwrap()
            .args,
        ["WSGI", "company.wsgi:application"]
    );
}

#[test]
fn source_build_context_separates_static_python_tools_and_serving() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("requirements.txt"), "mkdocs\n").unwrap();
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let text = format!(
            "[build]\ntoolchain='python'\npython_version='{}'\ncommand='mkdocs build'\noutput_directory='site'\n[deploy]\ncompute='static'\n",
            minor.version()
        );
        std::fs::write(project.path().join("onreza.toml"), &text).unwrap();
        let fs = LocalFs::new(project.path());
        let mut detection = detect(project.path());
        super::application_runtime::resolve_and_bind_detection(&fs, &mut detection).unwrap();
        let context = detection.metadata.source_build_context.unwrap();
        assert_eq!(context.build_toolchain.resolved_python_minor(), Some(minor));
        assert_eq!(context.application_runtime, None);
        assert_eq!(detection.suggested_compute, ComputeType::Static);
        assert_eq!(
            detection.metadata.runtime.version.as_deref(),
            Some(minor.version())
        );
        let virtual_fs=VirtualFs::from_json(&serde_json::json!({"tree":["requirements.txt","onreza.toml"],"files":{"requirements.txt":"mkdocs\n","onreza.toml":text}}).to_string()).unwrap();
        let mut remote = detect_with_fs(&virtual_fs);
        super::application_runtime::resolve_and_bind_detection(&virtual_fs, &mut remote).unwrap();
        assert_eq!(remote.metadata.source_build_context, Some(context));
        for launch in [
            "runtime='python'",
            "module='company.worker'",
            "application='company.web:app'\nserver='asgi'",
            "entry='server.js'",
            "args=[]",
            "python_version='3.12'",
        ] {
            let config: crate::config::ProjectConfig =
                toml::from_str(&format!("[deploy]\ncompute='static'\n{launch}\n")).unwrap();
            let mut detection = detect(project.path());
            assert!(
                super::application_runtime::resolve_and_bind_source_build_context(
                    &fs,
                    &mut detection,
                    &config,
                    None,
                    None
                )
                .unwrap_err()
                .to_string()
                .contains("STATIC serving conflicts")
            );
        }
    }
}

#[test]
fn python_console_scripts_own_launch_beside_native_helpers() {
    for scripts in [
        "[project.scripts]\nserve='company.web:main'",
        "[tool.poetry.scripts]\nserve={reference='company.web:main',type='console'}",
        "[project.scripts]\nserve='company.web:main'\nworker='company.worker:main'",
    ] {
        for helper in ["go", "dart"] {
            for javascript_tooling in [false, true] {
                let project = tempfile::tempdir().unwrap();
                std::fs::write(
                    project.path().join("pyproject.toml"),
                    format!(
                        "[project]\nname='company'\nversion='1.0.0'\ndependencies=[]\n{scripts}\n"
                    ),
                )
                .unwrap();
                if scripts.contains("tool.poetry") {
                    std::fs::write(project.path().join("poetry.lock"), "").unwrap();
                }
                std::fs::create_dir(project.path().join("company")).unwrap();
                std::fs::write(project.path().join("company/web.py"), "def main(): pass\n")
                    .unwrap();
                if helper == "go" {
                    std::fs::write(project.path().join("go.mod"), "module example.org/helper\n")
                        .unwrap();
                    std::fs::create_dir_all(project.path().join("cmd/helper")).unwrap();
                    std::fs::write(
                        project.path().join("cmd/helper/main.go"),
                        "package main\nfunc main() {}\n",
                    )
                    .unwrap();
                } else {
                    std::fs::write(project.path().join("pubspec.yaml"), "name: helper\n").unwrap();
                    std::fs::create_dir(project.path().join("bin")).unwrap();
                    std::fs::write(project.path().join("bin/helper.dart"), "void main() {}\n")
                        .unwrap();
                }
                if javascript_tooling {
                    std::fs::write(
                        project.path().join("package.json"),
                        r#"{"scripts":{"build":"echo tooling"}}"#,
                    )
                    .unwrap();
                }
                let fs = LocalFs::new(project.path());
                let mut detection = detect_with_fs(&fs);
                assert_eq!(
                    detection.framework, "python",
                    "{helper}, {scripts}, JS={javascript_tooling}"
                );
                let bound =
                    super::application_runtime::resolve_and_bind_detection(&fs, &mut detection);
                if scripts.contains("worker=") {
                    assert!(
                        bound
                            .unwrap_err()
                            .to_string()
                            .contains("multiple declared Python console scripts")
                    );
                } else {
                    bound.unwrap();
                    let runtime = detection
                        .metadata
                        .source_build_context
                        .unwrap()
                        .application_runtime
                        .unwrap();
                    assert_eq!(
                        runtime.family,
                        nrz_source_bundle::ApplicationRuntimeFamily::Python
                    );
                    assert_eq!(
                        runtime.entry.as_deref(),
                        Some(super::python_launch::PYTHON_BOOTSTRAP_ENTRY)
                    );
                    assert_eq!(runtime.args, ["CALLABLE", "company.web:main"]);
                }
            }
        }
    }
}

#[test]
fn source_build_context_defaults_follow_compiler_manager_and_python_minors() {
    use nrz_source_bundle::BuildToolchainFamily;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("package.json"),
        r#"{"scripts":{"start":"bun server.js"},"dependencies":{"express":"5.0.0"}}"#,
    )
    .unwrap();
    let fs = LocalFs::new(project.path());
    let mut detection = detect(project.path());
    super::application_runtime::resolve_and_bind_detection(&fs, &mut detection).unwrap();
    let context = detection.metadata.source_build_context.unwrap();
    assert_eq!(context.build_toolchain.family, BuildToolchainFamily::Node);
    assert_eq!(
        context.application_runtime.unwrap().family,
        nrz_source_bundle::ApplicationRuntimeFamily::Bun
    );
    std::fs::remove_file(project.path().join("package.json")).unwrap();
    std::fs::write(project.path().join("main.py"), "print('ready')\n").unwrap();
    for config in [
        "[deploy]\npython_version='3.12'",
        "[build]\npython_version='3.12'",
        "[build]\npython_version='3.12'\n[deploy]\npython_version='3.14'",
    ] {
        std::fs::write(project.path().join("onreza.toml"), config).unwrap();
        let mut detection = detect(project.path());
        super::application_runtime::resolve_and_bind_detection(&fs, &mut detection).unwrap();
        let context = detection.metadata.source_build_context.unwrap();
        assert_eq!(
            context.build_toolchain.resolved_python_minor(),
            Some(nrz_source_bundle::PythonMinor::Python312)
        );
        assert_eq!(
            context.application_runtime.unwrap().python_version,
            Some(if config.contains("3.14") {
                nrz_source_bundle::PythonMinor::Python314
            } else {
                nrz_source_bundle::PythonMinor::Python312
            })
        );
    }
}

#[test]
fn implicit_python_selectors_bind_native_evidence_like_explicit_python() {
    for (framework, native_files) in [
        (
            "go",
            vec![
                ("go.mod", "module example.org/server"),
                ("main.go", "package main\nfunc main() {}"),
            ],
        ),
        (
            "dart",
            vec![
                ("pubspec.yaml", "name: server"),
                ("bin/server.dart", "void main() {}"),
            ],
        ),
    ] {
        let mut files = native_files
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        // Keep native autodetection while the Python entry belongs to an authored
        // serving override; package metadata here only describes build tooling.
        files.insert(
            "package.json",
            r#"{"scripts":{"build":"echo auxiliary tooling"}}"#,
        );
        files.insert("main.py", "print(42)");
        let fs = super::fs::VirtualFs::from_json(&serde_json::json!({"files":files}).to_string())
            .unwrap();
        for selector in [
            "module='main'",
            "application='main:app'",
            "server='wsgi'",
            "python_version='3.12'",
        ] {
            for converter in [false, true] {
                let mut implicit: crate::config::ProjectConfig =
                    toml::from_str(&format!("[deploy]\n{selector}")).unwrap();
                implicit.build.command = converter.then(|| "emit-converted-output".into());
                let mut detected = super::detect_with_fs(&fs);
                assert_eq!(detected.framework, framework);
                let implied = super::application_runtime::resolve_and_bind_source_build_context(
                    &fs,
                    &mut detected,
                    &implicit,
                    None,
                    None,
                )
                .unwrap();
                let mut explicit = implicit.clone();
                explicit.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Python);
                let mut detected = super::detect_with_fs(&fs);
                let declared = super::application_runtime::resolve_and_bind_source_build_context(
                    &fs,
                    &mut detected,
                    &explicit,
                    None,
                    None,
                )
                .unwrap();
                assert_eq!(implied, declared);
                implicit.project.framework = Some(framework.into());
                let mut detected = super::detect_with_fs(&fs);
                assert!(
                    super::application_runtime::resolve_and_bind_source_build_context(
                        &fs,
                        &mut detected,
                        &implicit,
                        Some(framework),
                        None
                    )
                    .is_err()
                );
            }
        }
    }
}
