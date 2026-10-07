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
fn incomplete_python_dependencies_require_an_authored_launch() {
    use super::python_launch::{PythonLaunchRequest, resolve_launch_for_framework};
    for requirements in [
        "fastapi; python_version < '3.13'\nflask\ngunicorn\n",
        "-r requirements/base.txt\n",
    ] {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
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
