use super::*;

fn write(root: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn native_helpers_do_not_change_javascript_or_python_application_selection() {
    for (framework, files) in [
        (
            "nextjs",
            vec![(
                "package.json",
                r#"{"dependencies":{"next":"15.0.0","react":"19.0.0"},"scripts":{"build":"next build","start":"next start"}}"#,
            )],
        ),
        (
            "other",
            vec![
                ("package.json", r#"{"scripts":{"start":"node main.js"}}"#),
                ("main.js", "console.log('APP')"),
            ],
        ),
        (
            "fastapi",
            vec![
                ("requirements.txt", "fastapi==0.115.0\nuvicorn==0.34.0\n"),
                ("main.py", "from fastapi import FastAPI\napp = FastAPI()\n"),
            ],
        ),
        (
            "python",
            vec![
                ("requirements.txt", "# stdlib-only application\n"),
                ("main.py", "print('APP')\n"),
            ],
        ),
    ] {
        for helper in ["go", "dart"] {
            let project = tempfile::tempdir().unwrap();
            write(project.path(), &files);
            let expected = detect(project.path());
            assert_eq!(expected.framework, framework);
            let mut expected_bound = expected.clone();
            let expected_context = application_runtime::resolve_and_bind_source_build_context(
                &LocalFs::new(project.path()),
                &mut expected_bound,
                &crate::config::ProjectConfig::default(),
                None,
                None,
            )
            .unwrap();
            let helper_files = if helper == "go" {
                vec![
                    ("go.mod", "module example.org/tool\n"),
                    ("cmd/helper/main.go", "package main\nfunc main() {}\n"),
                ]
            } else {
                vec![
                    ("pubspec.yaml", "name: helper\n"),
                    ("bin/helper.dart", "void main() {}\n"),
                ]
            };
            write(project.path(), &helper_files);
            let mut actual = detect(project.path());
            assert_eq!(
                serde_json::to_value(&actual).unwrap(),
                serde_json::to_value(&expected).unwrap(),
                "{helper} helper replaced {framework} application"
            );
            let actual_context = application_runtime::resolve_and_bind_source_build_context(
                &LocalFs::new(project.path()),
                &mut actual,
                &crate::config::ProjectConfig::default(),
                None,
                None,
            )
            .unwrap();
            assert_eq!(actual_context, expected_context);
            assert_eq!(
                detect_with_framework_override(project.path(), Some(helper)).framework,
                helper
            );
        }
    }
}

#[test]
fn native_apps_keep_selection_with_auxiliary_python_and_javascript_tooling() {
    for (framework, files) in [
        (
            "go",
            vec![
                ("go.mod", "module example.org/app\n"),
                ("cmd/server/main.go", "package main\nfunc main() {}\n"),
            ],
        ),
        (
            "dart",
            vec![
                ("pubspec.yaml", "name: app\n"),
                ("bin/server.dart", "void main() {}\n"),
            ],
        ),
        (
            "flutter",
            vec![
                (
                    "pubspec.yaml",
                    "name: app\ndependencies:\n  flutter: {sdk: flutter}\n",
                ),
                ("web/index.html", "<html></html>"),
            ],
        ),
        (
            "hugo",
            vec![
                ("hugo.toml", "baseURL = 'https://example.org/'\n"),
                ("go.mod", "module example.org/site\n"),
                ("cmd/helper/main.go", "package main\nfunc main() {}\n"),
            ],
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        write(project.path(), &files);
        let expected = detect(project.path());
        assert_eq!(expected.framework, framework);
        write(
            project.path(),
            &[
                ("requirements.txt", "ruff==0.11.0\n"),
                ("package.json", r#"{"scripts":{"build":"node tooling.js"}}"#),
                ("tooling.js", "console.log('TOOL')"),
            ],
        );
        assert_eq!(
            serde_json::to_value(detect(project.path())).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(
            detect_with_framework_override(project.path(), Some("nextjs")).framework,
            "nextjs"
        );
        assert_eq!(
            detect_with_framework_override(project.path(), Some("python")).framework,
            "python"
        );
    }
}

#[test]
fn package_scripts_launching_native_apps_preserve_native_selection() {
    for (framework, files, launcher) in [
        (
            "go",
            vec![
                ("go.mod", "module example.org/app\n"),
                ("cmd/server/main.go", "package main\nfunc main() {}\n"),
            ],
            "go run ./cmd/server",
        ),
        (
            "dart",
            vec![
                ("pubspec.yaml", "name: app\n"),
                ("bin/server.dart", "void main() {}\n"),
            ],
            "dart bin/server.dart",
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        write(project.path(), &files);
        let expected = detect(project.path());
        assert_eq!(expected.framework, framework);
        let package = serde_json::json!({"scripts": {"start": launcher}}).to_string();
        write(project.path(), &[("package.json", &package)]);
        assert_eq!(
            serde_json::to_value(detect(project.path())).unwrap(),
            serde_json::to_value(expected).unwrap(),
            "package.json native launcher changed {framework} application"
        );
    }
}

#[test]
fn javascript_root_fallback_entries_keep_native_helpers_auxiliary() {
    for base in ROOT_ENTRY_BASENAMES {
        for extension in RUNNABLE_EXTENSIONS {
            let entry = format!("{base}.{extension}");
            for helper in ["go", "dart"] {
                let project = tempfile::tempdir().unwrap();
                let helper_files = native_helper_files(helper);
                write(project.path(), &helper_files);
                for package in [
                    r#"{"scripts":{"start":"npm run boot","boot":"node tooling.js"}}"#,
                    r#"{"scripts":{"build":"node tooling.js"}}"#,
                ] {
                    let mut files = helper_files.clone();
                    files.extend([
                        ("package.json", package),
                        (entry.as_str(), "console.log('APP')"),
                    ]);
                    write(project.path(), &files);
                    let json = serde_json::json!({
                        "files": files.iter().copied().collect::<std::collections::HashMap<_, _>>()
                    });
                    let virtual_fs = fs::VirtualFs::from_json(&json.to_string()).unwrap();
                    let tree_json = serde_json::json!({
                        "tree": [entry.as_str()],
                        "files": files.iter().copied().filter(|(path, _)| *path != entry).collect::<std::collections::HashMap<_, _>>()
                    });
                    let tree_fs = fs::VirtualFs::from_json(&tree_json.to_string()).unwrap();
                    let local_fs = LocalFs::new(project.path());
                    for fs in [
                        &local_fs as &dyn Fs,
                        &virtual_fs as &dyn Fs,
                        &tree_fs as &dyn Fs,
                    ] {
                        let mut detected = detect_with_fs(fs);
                        assert_eq!(detected.framework, "other", "{helper}, {entry}, {package}");
                        assert_eq!(detected.suggested_compute, ComputeType::Process);
                        assert_eq!(detected.metadata.runtime.runtime_type, RuntimeType::Node);
                        application_runtime::resolve_and_bind_detection(fs, &mut detected).unwrap();
                        let context = detected.metadata.source_build_context.unwrap();
                        assert_eq!(
                            context.build_toolchain.family,
                            nrz_source_bundle::BuildToolchainFamily::Node
                        );
                        assert!(context.application_runtime.is_none());
                        assert_eq!(
                            detect_with_fs_and_framework_override(fs, Some(helper)).framework,
                            helper
                        );
                    }
                    let resolved =
                        resolve_entry_point_detailed("other", project.path(), project.path());
                    assert!(
                        matches!(resolved, EntryPointResolution::Found(ResolvedEntryPoint { path, source: EntryPointSource::RootPattern }) if path == entry)
                    );
                }
            }
        }
    }
}

fn native_helper_files(helper: &str) -> Vec<(&'static str, &'static str)> {
    if helper == "go" {
        vec![
            ("go.mod", "module example.org/tool\n"),
            ("cmd/helper/main.go", "package main\nfunc main() {}\n"),
        ]
    } else {
        vec![
            ("pubspec.yaml", "name: helper\n"),
            ("bin/helper.dart", "void main() {}\n"),
        ]
    }
}

#[test]
fn javascript_root_fallback_preserves_explicit_launch_and_static_frameworks() {
    for helper in ["go", "dart"] {
        for runtime in ["node", "bun"] {
            let project = tempfile::tempdir().unwrap();
            let config = format!(
                "[deploy]\nruntime = '{runtime}'\nentry = 'server.js'\nargs = ['literal argument']\n"
            );
            let mut files = native_helper_files(helper);
            files.extend([
                ("package.json", r#"{"scripts":{"start":"npm run boot"}}"#),
                ("server.js", "console.log('APP')"),
                ("onreza.toml", config.as_str()),
            ]);
            write(project.path(), &files);
            let json = serde_json::json!({"files": files.iter().copied().collect::<std::collections::HashMap<_, _>>()});
            let virtual_fs = fs::VirtualFs::from_json(&json.to_string()).unwrap();
            let local_fs = LocalFs::new(project.path());
            for fs in [&local_fs as &dyn Fs, &virtual_fs as &dyn Fs] {
                let mut detected = detect_with_fs(fs);
                application_runtime::resolve_and_bind_detection(fs, &mut detected).unwrap();
                assert_eq!(detected.framework, "other");
                let declaration = detected
                    .metadata
                    .source_build_context
                    .unwrap()
                    .application_runtime
                    .unwrap();
                assert_eq!(
                    declaration.family,
                    if runtime == "node" {
                        nrz_source_bundle::ApplicationRuntimeFamily::Node
                    } else {
                        nrz_source_bundle::ApplicationRuntimeFamily::Bun
                    }
                );
                assert_eq!(declaration.entry.as_deref(), Some("server.js"));
                assert_eq!(declaration.args, ["literal argument"]);
            }
        }
    }
    for (framework, native_files) in [
        (
            "flutter",
            vec![
                (
                    "pubspec.yaml",
                    "name: app\ndependencies:\n  flutter: {sdk: flutter}\n",
                ),
                ("web/index.html", "<html></html>"),
            ],
        ),
        (
            "hugo",
            vec![("hugo.toml", "baseURL = 'https://example.org/'\n")],
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        write(project.path(), &native_files);
        write(
            project.path(),
            &[("package.json", "{}"), ("server.js", "console.log('APP')")],
        );
        let detected = detect(project.path());
        assert_eq!(detected.framework, framework);
        assert_eq!(detected.suggested_compute, ComputeType::Static);
    }
}

#[test]
fn absent_package_or_root_entry_directory_does_not_replace_native_application() {
    for helper in ["go", "dart"] {
        let project = tempfile::tempdir().unwrap();
        write(project.path(), &native_helper_files(helper));
        write(project.path(), &[("server.js", "console.log('TOOL')")]);
        assert_eq!(detect(project.path()).framework, helper);
        std::fs::remove_file(project.path().join("server.js")).unwrap();
        std::fs::create_dir(project.path().join("server.js")).unwrap();
        write(project.path(), &[("package.json", "{}")]);
        assert_eq!(detect(project.path()).framework, helper);
    }
}

#[cfg(unix)]
#[test]
fn non_regular_root_entry_does_not_replace_native_application() {
    for helper in ["go", "dart"] {
        let project = tempfile::tempdir().unwrap();
        write(project.path(), &native_helper_files(helper));
        write(project.path(), &[("package.json", "{}")]);
        let _socket =
            std::os::unix::net::UnixListener::bind(project.path().join("server.js")).unwrap();
        assert_eq!(detect(project.path()).framework, helper);
        assert_eq!(
            resolve_entry_point_detailed("other", project.path(), project.path()),
            EntryPointResolution::NotFound
        );
    }
}
