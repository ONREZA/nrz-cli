use super::fs::{LocalFs, VirtualFs};
use super::python::dependency_manifest;
use super::types::{ComputeType, PackageManagerType, RuntimeType};
use super::{detect, detect_with_fs, resolve_entry_point};

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
        let declaration = detection.metadata.application_runtime.unwrap();
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
