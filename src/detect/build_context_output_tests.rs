use super::application_runtime::{
    bind_source_build_context, resolve_and_bind_source_build_context,
};
use super::fs::{Fs, LocalFs, VirtualFs};
use super::types::PackageManagerType;

#[test]
fn independent_compilers_preserve_fresh_framework_build_hints() {
    for config_text in [
        "[build]\ntoolchain='python'\ncommand='python generate.py'\n[deploy]\ncompute='static'\n",
        "[build]\ntoolchain='node'\ncommand='node generate.js'\n[deploy]\nruntime='python'\nentry='dist/server.py'\n",
    ] {
        let project = tempfile::tempdir().unwrap();
        let package = r#"{"devDependencies":{"vite":"7.0.0"},"scripts":{"build":"vite build"}}"#;
        std::fs::write(project.path().join("package.json"), package).unwrap();
        let local = LocalFs::new(project.path());
        let remote = VirtualFs::from_json(
            &serde_json::json!({"tree":["package.json"],"files":{"package.json":package}})
                .to_string(),
        )
        .unwrap();
        let config = toml::from_str(config_text).unwrap();
        for fs in [&local as &dyn Fs, &remote] {
            let mut detection = super::detect_with_fs(fs);
            assert_eq!(detection.framework, "vite");
            let inferred = detection.metadata.build_info.clone().unwrap();
            let inferred_manager = detection.metadata.package_manager.as_ref().unwrap().pm_type;
            assert_eq!(inferred.output_dir.as_deref(), Some("dist"));
            let context =
                resolve_and_bind_source_build_context(fs, &mut detection, &config, None, None)
                    .unwrap();
            for refreshed in [false, true] {
                if refreshed {
                    detection = super::detect_with_fs(fs);
                    bind_source_build_context(fs, &mut detection, &context, &config, None, None)
                        .unwrap();
                }
                let build = detection.metadata.build_info.as_ref().unwrap();
                assert_eq!(build.output_dir, inferred.output_dir, "{config_text}");
                assert_eq!(build.build_command, inferred.build_command, "{config_text}");
                assert_eq!(build.entry_point, inferred.entry_point, "{config_text}");
                if context.build_toolchain.resolved_python_minor().is_some() {
                    assert_eq!(
                        detection.metadata.package_manager.as_ref().unwrap().pm_type,
                        PackageManagerType::Pip
                    );
                    assert_eq!(build.install_command, None);
                } else {
                    assert_eq!(
                        detection.metadata.package_manager.as_ref().unwrap().pm_type,
                        inferred_manager
                    );
                }
            }
        }
    }
}

#[test]
fn shallow_selection_rejects_the_same_authored_namespace_conflicts_as_full_context() {
    use super::application_runtime::{resolve_build_toolchain, resolve_serving_python_minor};
    use super::types::ComputeType;
    let fs = VirtualFs::from_json(r#"{"tree":[],"files":{}}"#).unwrap();
    let mut cases = Vec::new();
    for field in [
        "runtime='python'",
        "entry='server.py'",
        "args=[]",
        "module='company.worker'",
        "application='company.web:app'",
        "server='asgi'",
        "python_version='3.12'",
    ] {
        for explicit in [None, Some(ComputeType::Static)] {
            let compute = if explicit.is_some() {
                "process"
            } else {
                "static"
            };
            cases.push((format!("[deploy]\ncompute='{compute}'\n{field}\n"), explicit,
                "STATIC serving conflicts with explicit PROCESS launch fields; use build.toolchain and build.python_version for build-only selectors"));
        }
    }
    for family in ["node", "bun", "executable"] {
        for field in [
            "module='company.worker'",
            "application='company.web:app'",
            "server='asgi'",
            "python_version='3.12'",
        ] {
            cases.push((
                format!("[deploy]\nruntime='{family}'\n{field}\n"),
                None,
                "Python launch conflicts with the declared application runtime",
            ));
        }
    }
    for (text, explicit_compute, expected) in cases {
        let config = toml::from_str(&text).unwrap();
        let mut detection = super::detect_with_fs(&fs);
        let before = detection.metadata.source_build_context.clone();
        let build = resolve_build_toolchain(&detection, &config).unwrap();
        let full = resolve_and_bind_source_build_context(
            &fs,
            &mut detection,
            &config,
            None,
            explicit_compute,
        )
        .unwrap_err();
        assert_eq!(full.to_string(), expected, "{text}");
        assert_eq!(detection.metadata.source_build_context, before, "{text}");
        let shallow = resolve_serving_python_minor(&detection, &config, &build, explicit_compute)
            .unwrap_err();
        assert_eq!(shallow.to_string(), expected, "{text}");
    }
}

#[test]
fn shallow_selection_preserves_independent_compilers_and_incomplete_launches() {
    use super::application_runtime::{resolve_build_toolchain, resolve_serving_python_minor};
    use nrz_source_bundle::{ApplicationRuntimeFamily, PythonMinor};
    let fs = VirtualFs::from_json(r#"{"tree":["main.py","pyproject.toml"],"files":{"main.py":"print('ready')\n","pyproject.toml":"[tool.poetry]\npackage-mode=false\n"}}"#).unwrap();
    for minor in PythonMinor::ALL {
        let mut detection = super::detect_with_fs(&fs);
        let config = toml::from_str(&format!("[build]\ntoolchain='python'\npython_version='{}'\n[deploy]\nruntime='node'\nentry='server.js'\n", minor.version())).unwrap();
        let build = resolve_build_toolchain(&detection, &config).unwrap();
        assert_eq!(build.resolved_python_minor(), Some(minor));
        assert_eq!(
            resolve_serving_python_minor(&detection, &config, &build, None).unwrap(),
            None
        );
        let context =
            resolve_and_bind_source_build_context(&fs, &mut detection, &config, None, None)
                .unwrap();
        assert_eq!(
            context.application_runtime.unwrap().family,
            ApplicationRuntimeFamily::Node
        );
        let config = toml::from_str(&format!(
            "[build]\npython_version='{}'\n[deploy]\nruntime='python'\n",
            minor.version()
        ))
        .unwrap();
        let build = resolve_build_toolchain(&detection, &config).unwrap();
        assert_eq!(
            resolve_serving_python_minor(&detection, &config, &build, None).unwrap(),
            Some(minor)
        );
        let missing = VirtualFs::from_json(r#"{"tree":["pyproject.toml"],"files":{"pyproject.toml":"[tool.poetry]\npackage-mode=false\n"}}"#).unwrap();
        let detection = super::detect_with_fs(&missing);
        let build = resolve_build_toolchain(&detection, &config).unwrap();
        assert_eq!(
            resolve_serving_python_minor(&detection, &config, &build, None).unwrap(),
            Some(minor)
        );
    }
}
