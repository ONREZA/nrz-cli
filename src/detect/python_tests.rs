use super::fs::{LocalFs, VirtualFs};
use super::python::{PYTHON_RUNTIME_VERSION, dependency_manifest};
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
        Some(PYTHON_RUNTIME_VERSION)
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
