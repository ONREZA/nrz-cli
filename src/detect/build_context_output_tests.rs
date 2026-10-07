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
