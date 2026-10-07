use super::fs::VirtualFs;
use super::native::{NativeRecipe, detect_native, go_main_packages, native_recipe};
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
fn go_root_main_package_does_not_require_a_main_go_filename() {
    let input = fs(&[
        ("go.mod", "module example.org/server"),
        ("server.go", "package main\nfunc main() {}"),
    ]);
    assert_eq!(go_main_packages(&input), ["."]);
    let result = detect_native(&input).unwrap();
    assert_eq!(result.framework, "go");
    assert_eq!(result.suggested_compute, ComputeType::Process);
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
fn go_packages_are_discovered_from_any_direct_non_test_source_file() {
    let input = fs(&[
        ("go.mod", "module example.org/server"),
        ("types.go", "package main\ntype Server struct{}"),
        ("server.go", "package main\nfunc main() {}"),
        ("cmd/worker/worker.go", "package main; func main() {}"),
        ("cmd/worker/types.go", "package main"),
        ("cmd/library/library.go", "package library"),
        ("cmd/nested/internal/main.go", "package main"),
        ("main_test.go", "package main"),
        ("cmd/tests/main_test.go", "package main"),
        (".hidden.go", "package main"),
        ("_ignored.go", "package main"),
    ]);
    // One candidate per package directory, including both ambiguous candidates.
    assert_eq!(go_main_packages(&input), [".", "./cmd/worker"]);
    let result = detect_native(&input).unwrap();
    assert_eq!(result.framework, "go");
    assert_eq!(result.suggested_compute, ComputeType::Process);
}

#[test]
fn go_package_clause_is_lexical_and_precedes_other_source_tokens() {
    for source in [
        "package main; func main() {}",
        "\u{feff}//go:build linux\r\n/* license */\r\npackage/* separator */main\r\nfunc main() {}",
        "package\nmain // package clause comment\nfunc main() {}",
    ] {
        assert_eq!(
            go_main_packages(&fs(&[("server.go", source)])),
            ["."],
            "{source}"
        );
    }
    for source in [
        "/*\npackage main\n*/\npackage library",
        "package library\nvar text = `\npackage main\n`",
        "package mainish\nfunc main() {}",
        "packagemain",
        "/* unterminated\npackage main",
        "// package main",
        "var text = \"package main\"",
    ] {
        assert!(
            go_main_packages(&fs(&[("server.go", source)])).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn go_test_hidden_and_nested_files_do_not_make_a_deployable_package() {
    let input = fs(&[
        ("go.mod", "module example.org/library"),
        ("library.go", "package library"),
        ("main_test.go", "package main"),
        (".hidden.go", "package main"),
        ("_ignored.go", "package main"),
        ("cmd/tests/main_test.go", "package main"),
        ("cmd/hidden/.hidden.go", "package main"),
        ("cmd/ignored/_ignored.go", "package main"),
        ("cmd/deep/internal/server.go", "package main"),
    ]);
    assert!(go_main_packages(&input).is_empty());
    assert!(detect_native(&input).is_none());
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

#[test]
fn matching_executable_declarations_preserve_inferred_native_build_recipes() {
    use nrz_source_bundle::{ApplicationRuntimeFamily, BuildToolchainFamily};
    for (framework, input) in [
        (
            "go",
            fs(&[
                ("go.mod", "module example.org/server"),
                ("main.go", "package main\nfunc main() {}"),
            ]),
        ),
        (
            "dart",
            fs(&[
                ("pubspec.yaml", "name: server"),
                ("bin/server.dart", "void main() {}"),
            ]),
        ),
    ] {
        for explicit_compiler in [false, true] {
            for configured_entry in [None, Some("explicit-server")] {
                let mut config = crate::config::ProjectConfig::default();
                config.deploy.runtime = Some(ApplicationRuntimeFamily::Executable);
                config.build.toolchain = explicit_compiler.then_some(BuildToolchainFamily::Native);
                config.deploy.entry = configured_entry.map(str::to_owned);
                let mut detection = detect_native(&input).unwrap();
                let inferred_entry = detection
                    .metadata
                    .build_info
                    .as_ref()
                    .unwrap()
                    .entry_point
                    .as_deref()
                    .unwrap()
                    .to_owned();
                let inferred_build = serde_json::to_value(&detection.metadata.build_info).unwrap();
                let context = super::application_runtime::resolve_and_bind_source_build_context(
                    &input,
                    &mut detection,
                    &config,
                    None,
                    None,
                )
                .unwrap();
                assert_eq!(detection.framework, framework);
                assert_eq!(
                    native_recipe(&detection.framework),
                    native_recipe(framework)
                );
                assert_eq!(
                    serde_json::to_value(&detection.metadata.build_info).unwrap(),
                    inferred_build
                );
                assert_eq!(context.build_toolchain.family, BuildToolchainFamily::Native);
                assert_eq!(
                    context.application_runtime.as_ref().unwrap().family,
                    ApplicationRuntimeFamily::Executable
                );
                assert_eq!(
                    context
                        .application_runtime
                        .as_ref()
                        .unwrap()
                        .entry
                        .as_deref(),
                    Some(configured_entry.unwrap_or(&inferred_entry))
                );
                let mut refreshed = detect_native(&input).unwrap();
                super::application_runtime::bind_source_build_context(
                    &input,
                    &mut refreshed,
                    &context,
                    &config,
                    None,
                    None,
                )
                .unwrap();
                assert_eq!(refreshed.framework, framework);
                assert_eq!(
                    refreshed.metadata.source_build_context.as_ref(),
                    Some(&context)
                );
                assert_eq!(
                    serde_json::to_value(&refreshed.metadata.build_info).unwrap(),
                    inferred_build
                );
            }
        }
    }
}

#[test]
fn authored_converters_keep_launch_authority_for_each_runtime_family() {
    use nrz_source_bundle::ApplicationRuntimeFamily as Family;
    let input = fs(&[
        ("go.mod", "module example.org/server"),
        ("main.go", "package main\nfunc main() {}"),
    ]);
    for family in [
        Family::Python,
        Family::Node,
        Family::Bun,
        Family::Executable,
    ] {
        let mut config = crate::config::ProjectConfig::default();
        config.build.command = Some("emit-converted-output".into());
        config.deploy.runtime = Some(family);
        config.deploy.entry = Some("converted-server".into());
        config.deploy.args = Some(vec!["literal argument".into()]);
        let mut detection = detect_native(&input).unwrap();
        let context = super::application_runtime::resolve_and_bind_source_build_context(
            &input,
            &mut detection,
            &config,
            None,
            None,
        )
        .unwrap();
        assert!(native_recipe(&detection.framework).is_none());
        let serving = context.application_runtime.as_ref().unwrap();
        assert_eq!(serving.family, family);
        assert_eq!(serving.entry.as_deref(), Some("converted-server"));
        assert_eq!(serving.args, ["literal argument"]);
        if family != Family::Executable {
            config.project.framework = Some("go".into());
            let mut declared = detect_native(&input).unwrap();
            assert!(
                super::application_runtime::resolve_and_bind_source_build_context(
                    &input,
                    &mut declared,
                    &config,
                    Some("go"),
                    None
                )
                .is_err()
            );
        }
    }
}
