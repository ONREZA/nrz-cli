use super::fs::VirtualFs;
use super::native::{NativeRecipe, detect_native, native_recipe};
use super::types::{ComputeType, PackageManagerType, RuntimeType};

fn fs(files: &[(&str, &str)]) -> VirtualFs {
    VirtualFs::from_json(
        &serde_json::json!({
            "files": files.iter().map(|(path, text)| ((*path).to_string(), (*text).to_string()))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn flutter_sdk_dependency_beats_incidental_javascript_tooling() {
    let input = fs(&[
        (
            "pubspec.yaml",
            "name: app\ndependencies:\n  flutter: {sdk: flutter}\n",
        ),
        ("web/index.html", "<html></html>"),
        ("package.json", r#"{"scripts":{"build":"node tooling.js"}}"#),
        ("pubspec.lock", "packages: {}"),
    ]);
    let result = detect_native(&input).unwrap();
    assert_eq!(result.framework, "flutter");
    assert_eq!(result.suggested_compute, ComputeType::Static);
    assert_eq!(result.metadata.runtime.runtime_type, RuntimeType::Dart);
    assert_eq!(
        result.metadata.package_manager.unwrap().pm_type,
        PackageManagerType::Pub
    );
    assert_eq!(
        result.metadata.build_info.unwrap().output_dir.as_deref(),
        Some("build/web")
    );
}

#[test]
fn dart_server_and_library_have_different_serving_intent() {
    let server = fs(&[
        ("pubspec.yaml", "name: server"),
        ("bin/server.dart", "void main() {}"),
    ]);
    let result = detect_native(&server).unwrap();
    assert_eq!(result.framework, "dart");
    assert_eq!(result.suggested_compute, ComputeType::Process);
    assert_eq!(
        result.metadata.build_info.unwrap().entry_point.as_deref(),
        Some("bin/server")
    );
    let library = fs(&[
        ("pubspec.yaml", "name: library"),
        ("lib/main.dart", "void f() {}"),
    ]);
    assert!(detect_native(&library).is_none());
}

#[test]
fn mobile_flutter_is_not_automatically_deployable_web() {
    let input = fs(&[
        (
            "pubspec.yaml",
            "dependencies:\n  flutter:\n    sdk: flutter",
        ),
        ("bin/server.dart", "void main() {}"),
        ("lib/main.dart", "void main() {}"),
    ]);
    assert!(detect_native(&input).is_none());
}

#[test]
fn flutter_text_in_comment_does_not_select_web_recipe() {
    let input = fs(&[
        ("pubspec.yaml", "name: server\n# flutter:\n#  sdk: flutter"),
        ("bin/server.dart", "void main() {}"),
        ("web/index.html", "html"),
    ]);
    assert_eq!(detect_native(&input).unwrap().framework, "dart");
}

#[test]
fn hugo_with_go_modules_stays_static_without_fake_package_manager() {
    let input = fs(&[
        ("hugo.toml", "baseURL = 'https://example.org/'"),
        ("go.mod", "module example.org/site"),
        ("main.go", "package main"),
    ]);
    let result = detect_native(&input).unwrap();
    assert_eq!(result.framework, "hugo");
    assert_eq!(result.suggested_compute, ComputeType::Static);
    assert!(result.metadata.package_manager.is_none());
    assert_eq!(native_recipe("hugo"), Some(NativeRecipe::HugoStatic));
}

#[test]
fn go_main_package_has_process_intent_but_library_does_not() {
    let input = fs(&[
        ("go.mod", "module example.org/server"),
        ("cmd/server/main.go", "package main\nfunc main() {}"),
    ]);
    let result = detect_native(&input).unwrap();
    assert_eq!(result.framework, "go");
    assert_eq!(result.suggested_compute, ComputeType::Process);
    assert_eq!(
        result.metadata.package_manager.unwrap().pm_type,
        PackageManagerType::Go
    );
    assert!(
        detect_native(&fs(&[
            ("go.mod", "module example.org/lib"),
            ("main.go", "package example")
        ]))
        .is_none()
    );
}

#[test]
fn multiple_dart_entries_preserve_process_intent_without_arbitrary_entry() {
    let input = fs(&[
        ("pubspec.yaml", "name: app"),
        ("bin/a.dart", "void main() {}"),
        ("bin/b.dart", "void main() {}"),
    ]);
    let result = detect_native(&input).unwrap();
    assert_eq!(result.suggested_compute, ComputeType::Process);
    assert!(result.metadata.build_info.unwrap().entry_point.is_none());
}

#[test]
fn advertised_remote_inputs_preserve_detected_frameworks() {
    for (expected, files) in [
        (
            "flutter",
            vec![
                ("pubspec.yaml", "dependencies:\n  flutter: {sdk: flutter}"),
                ("web/index.html", "<html></html>"),
            ],
        ),
        (
            "dart",
            vec![
                ("pubspec.yaml", "name: server"),
                ("bin/server.dart", "void main() {}"),
            ],
        ),
        (
            "go",
            vec![
                ("go.mod", "module example.org/server"),
                ("main.go", "package main\nfunc main() {}"),
            ],
        ),
        (
            "static-html",
            vec![("index.html", "<!doctype html><html></html>")],
        ),
        (
            "other",
            vec![
                ("package.json", "{}"),
                ("yarn.lock", "__metadata:\n  version: 8"),
            ],
        ),
    ] {
        let input = VirtualFs::from_json(
            &serde_json::json!({
                "tree": files.iter().map(|(path, _)| path).collect::<Vec<_>>(),
                "files": files.iter()
                    .filter(|(path, _)| super::fs::DETECTION_CONTENT_FILES.contains(path))
                    .map(|(path, text)| (*path, *text))
                    .collect::<std::collections::HashMap<_, _>>()
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(super::detect_with_fs(&input).framework, expected);
        assert_eq!(
            super::package_manager::detect_yarn_generation_if_configured(&input, None),
            super::package_manager::detect_yarn_generation_if_configured(&fs(&files), None)
        );
    }
}
