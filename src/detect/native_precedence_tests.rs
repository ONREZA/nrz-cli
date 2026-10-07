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
