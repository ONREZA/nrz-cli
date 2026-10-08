use super::native_build::{
    GoModuleInputs, apply_flutter_static_cache_policy, compiler_version, default_command,
    ensure_no_native_hooks, install_command, recipe_commands, validate_output,
};
use crate::detect::native::{NATIVE_RUNTIME_TARGET, NativeRecipe};

fn file(root: &std::path::Path, name: &str, text: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn fixture_root() -> std::path::PathBuf {
    std::env::var_os("NRZ_QUALIFICATION_SOURCE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .join("tests/fixtures")
}

fn copy_fixture(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let output = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &output);
        } else {
            std::fs::copy(entry.path(), output).unwrap();
        }
    }
}

#[test]
fn authored_flutter_spa_and_cache_rules_match_the_existing_edge_contract() {
    let project = tempfile::tempdir().unwrap();
    file(
        project.path(),
        "onreza.rules.toml",
        include_str!("../../tests/fixtures/flutter-web/edge-rules.example.toml"),
    );
    let report = crate::functions::check_edge_rules(project.path())
        .unwrap()
        .unwrap();
    assert_eq!(report.rule_count, 2);
}

#[tokio::test]
async fn flutter_static_cache_default_survives_wire_and_verified_source_publication() {
    let output = tempfile::tempdir().unwrap();
    file(output.path(), "index.html", "<html></html>");
    file(output.path(), "main.dart.js", "window.fixture = true;");
    let mut manifest = crate::build::manifest::generate_static_manifest();
    apply_flutter_static_cache_policy(&mut manifest);
    let wire =
        crate::deploy::conform_manifest_to_wire_contract(serde_json::to_value(&manifest).unwrap())
            .unwrap();
    assert_eq!(wire["routes"][0]["headers"]["Cache-Control"], "no-cache");
    let plan = crate::artifact::source_bundle_v1::build_source_bundle_plan(
        output.path(),
        &manifest,
        &crate::deploy::scan_dir(output.path()).unwrap(),
    )
    .unwrap();
    let logical: nrz_source_bundle::SourceLogicalManifest =
        serde_json::from_value(serde_json::to_value(&plan.logical_manifest).unwrap()).unwrap();
    let owner = uuid::Uuid::nil().to_string();
    let input = nrz_source_bundle::SourceBundleVerificationInput {
        owner_workspace_id: owner.clone(),
        source_artifact_id: nrz_source_bundle::compute_source_artifact_id(
            &owner,
            &plan.logical_manifest_sha256,
            &plan.source_sha256,
            None,
        ),
        source_sha256: plan.source_sha256.clone(),
        logical_manifest_sha256: plan.logical_manifest_sha256.clone(),
        budget: nrz_source_bundle::SourceBundleVerificationBudget::from_manifest(&logical).unwrap(),
    };
    let verified = nrz_source_bundle::verify_source_bundle_bytes(
        input,
        std::fs::read(plan.source_path()).unwrap().into(),
    )
    .await
    .unwrap();
    assert_eq!(
        verified.logical_manifest["routes"][0]["headers"]["Cache-Control"],
        "no-cache"
    );
}

#[test]
fn flutter_static_cache_default_preserves_authored_header_case_and_compute_routes() {
    let mut manifest = crate::build::manifest::generate_static_manifest();
    manifest.routes[0].headers = Some(std::collections::HashMap::from([
        ("cAcHe-CoNtRoL".into(), "public, max-age=15".into()),
        ("X-Authored".into(), "keep".into()),
    ]));
    let compute = crate::build::manifest::generate_compute_manifest("server");
    manifest.layers.extend(compute.layers);
    manifest.routes.extend(compute.routes);
    apply_flutter_static_cache_policy(&mut manifest);
    let headers = manifest.routes[0].headers.as_ref().unwrap();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers["cAcHe-CoNtRoL"], "public, max-age=15");
    assert_eq!(headers["X-Authored"], "keep");
    assert!(manifest.routes[1].headers.is_none());
}

// Exercise the actual producer and shared consumer contracts before executing
// extracted bytes. In particular, a Windows-style non-executable source file
// must become an executable archive entry only after declared ELF validation.
async fn execute_verified_native_source_bundle(
    output: &std::path::Path,
    entry: &str,
    framework: &str,
) -> std::process::Output {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(output.join(entry), std::fs::Permissions::from_mode(0o644))
            .unwrap();
    }
    let mut manifest = crate::build::manifest::generate_compute_manifest(entry);
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Executable,
        python_version: None,
        entry: Some(entry.into()),
        args: vec!["--self-check".into(), "$(id)".into(), "two words".into()],
    };
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        Some(&declaration),
        Some(NATIVE_RUNTIME_TARGET),
        framework,
    )
    .unwrap();
    crate::build::manifest::validate(&manifest).unwrap();
    crate::build::manifest::verify_files(output, &manifest).unwrap();
    let files = crate::deploy::scan_dir(output).unwrap();
    let plan =
        crate::artifact::source_bundle_v1::build_source_bundle_plan(output, &manifest, &files)
            .unwrap();
    let logical: nrz_source_bundle::SourceLogicalManifest =
        serde_json::from_value(serde_json::to_value(&plan.logical_manifest).unwrap()).unwrap();
    assert!(
        logical
            .files
            .iter()
            .find(|file| file.path == entry)
            .unwrap()
            .executable
    );
    let compressed = std::fs::read(plan.source_path()).unwrap();
    let owner = uuid::Uuid::nil().to_string();
    let input = nrz_source_bundle::SourceBundleVerificationInput {
        owner_workspace_id: owner.clone(),
        source_artifact_id: nrz_source_bundle::compute_source_artifact_id(
            &owner,
            &plan.logical_manifest_sha256,
            &plan.source_sha256,
            None,
        ),
        source_sha256: plan.source_sha256.clone(),
        logical_manifest_sha256: plan.logical_manifest_sha256.clone(),
        budget: nrz_source_bundle::SourceBundleVerificationBudget::from_manifest(&logical).unwrap(),
    };
    let verified = nrz_source_bundle::verify_source_bundle_bytes(input, compressed.clone().into())
        .await
        .unwrap();
    let logical: nrz_source_bundle::SourceLogicalManifest =
        serde_json::from_value(verified.logical_manifest).unwrap();
    let targets = std::collections::HashMap::from([(
        logical.layers[0].name.clone(),
        NATIVE_RUNTIME_TARGET.into(),
    )]);
    let graph = nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
        &plan.logical_manifest_sha256,
        &plan.source_sha256,
        plan.source_size_bytes,
        &logical,
        &[],
        &targets,
    )
    .unwrap();
    nrz_runtime_artifact::verify_source_runtime_graph_dependencies(&logical, &graph).unwrap();
    let runtime = &graph.wire().runtime_layers[0];
    nrz_runtime_artifact::verify_source_runtime_layer_for_target(
        &logical.layers[0],
        runtime,
        Some(NATIVE_RUNTIME_TARGET),
    )
    .unwrap();
    let launch = runtime.launch.as_ref().unwrap();
    assert_eq!(
        launch.profile,
        nrz_runtime_artifact::RuntimeProfile::Executable
    );
    let extracted = tempfile::tempdir().unwrap();
    let decoded = zstd::stream::read::Decoder::new(std::io::Cursor::new(compressed)).unwrap();
    tar::Archive::new(decoded).unpack(extracted.path()).unwrap();
    let application = extracted.path().join(runtime.application_root.as_str());
    let executable = application.join(runtime.entrypoint.as_str());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(&executable).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
    nrz_runtime_artifact::verify_native_executable(&std::fs::read(&executable).unwrap()).unwrap();
    if std::env::var("ONREZA_BUILDER_QUALIFICATION").as_deref() == Ok("1") {
        let destination = std::path::PathBuf::from("/qualification/output/native").join(framework);
        copy_fixture(&application, &destination);
        std::fs::write(destination.join("launch.json"), serde_json::to_vec(&serde_json::json!({
            "family": "native", "framework": framework, "entry": runtime.entrypoint.as_str(),
            "selfCheckArgs": launch.args.iter().map(|arg| arg.as_str()).collect::<Vec<_>>(),
            "args": launch.args.iter().skip(1).map(|arg| arg.as_str()).collect::<Vec<_>>(), "cwd": launch.cwd.as_str(),
            "expectedBody": "native asset ready\n",
        })).unwrap()).unwrap();
    }
    let mut process = std::process::Command::new(executable);
    process
        .args(launch.args.iter().map(|arg| arg.as_str()))
        .current_dir(application.join(launch.cwd.as_str()));
    if framework == "dart" {
        assert!(application.join("lib/libfixture.so").is_file());
        process.env("NRZ_FIXTURE_NATIVE_LIBRARY", "1");
    }
    process.output().unwrap()
}

#[tokio::test]
#[ignore = "requires all exact native compilers on PATH; observes versions before user code"]
async fn real_native_compiler_readback_matches_qualified_pins() {
    for recipe in [
        NativeRecipe::DartServer,
        NativeRecipe::GoServer,
        NativeRecipe::FlutterWeb,
        NativeRecipe::HugoStatic,
    ] {
        super::native_build::validate_compiler_before_execution(
            recipe,
            std::env::var("ONREZA_BUILDER_QUALIFICATION").as_deref() == Ok("1"),
            &[],
        )
        .await
        .unwrap();
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    real_native_library_closure_rejects_missing_direct_and_transitive_libraries();
    if std::env::var("ONREZA_BUILDER_QUALIFICATION").as_deref() == Ok("1") {
        let output = std::env::var_os("NRZ_QUALIFICATION_OUTPUT")
            .unwrap_or_else(|| "/qualification/output".into());
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(std::path::Path::new(&output).join("native-system-libraries.json"), serde_json::to_vec(&serde_json::json!({
            "target": NATIVE_RUNTIME_TARGET,
            "libraries": nrz_runtime_artifact::NativeExecutableRequirements::QUALIFIED_SYSTEM_LIBRARIES,
        })).unwrap()).unwrap();
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), "#include <dlfcn.h>\n#include <stdio.h>\nint main(int argc, char **argv) { if (argc < 2) return 2; for (int i=1; i<argc; i++) { void *h=dlopen(argv[i], RTLD_NOW); if (!h) { fprintf(stderr, \"%s: %s\\n\", argv[i], dlerror()); return 1; } dlclose(h); } return 0; }\n").unwrap();
        let compiled = std::process::Command::new("cc")
            .args(["-x", "c"])
            .arg(source.path())
            .args(["-ldl", "-o"])
            .arg(std::path::Path::new(&output).join("native-system-libraries-probe"))
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[test]
#[ignore = "requires a C compiler; qualified native compiler readback also runs this regression"]
fn real_native_library_closure_rejects_missing_direct_and_transitive_libraries() {
    let output = tempfile::tempdir().unwrap();
    std::fs::create_dir(output.path().join("lib")).unwrap();
    file(output.path(), "bar.c", "int bar(void) { return 42; }");
    file(
        output.path(),
        "foo.c",
        "int bar(void); int foo(void) { return bar(); }",
    );
    file(
        output.path(),
        "main.c",
        "int foo(void); int main(void) { return foo() == 42 ? 0 : 1; }",
    );
    for args in [
        vec![
            "-shared",
            "-fPIC",
            "bar.c",
            "-Wl,-soname,libbar.so",
            "-o",
            "lib/libbar.so",
        ],
        vec![
            "-shared",
            "-fPIC",
            "foo.c",
            "-Llib",
            "-lbar",
            "-Wl,-soname,libfoo.so",
            "-Wl,-rpath,$ORIGIN",
            "-o",
            "lib/libfoo.so",
        ],
        vec![
            "main.c",
            "-Llib",
            "-lfoo",
            "-Wl,-rpath,$ORIGIN/lib",
            "-o",
            "server",
        ],
    ] {
        let result = std::process::Command::new("cc")
            .args(args)
            .current_dir(output.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    validate_output(
        output.path(),
        output.path(),
        NativeRecipe::DartServer,
        Some("server"),
    )
    .unwrap();
    let bar = std::fs::read(output.path().join("lib/libbar.so")).unwrap();
    let mut header_only = vec![0; 512];
    header_only[..64].copy_from_slice(&bar[..64]);
    // No program/section tables: syntactically ELF64 ET_DYN, but not loadable.
    header_only[32..48].fill(0);
    header_only[56..64].fill(0);
    std::fs::write(output.path().join("lib/libbar.so"), header_only).unwrap();
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "shared ELF without loadable segments was accepted"
    );
    std::fs::write(output.path().join("lib/libbar.so"), bar).unwrap();
    // A bundled SONAME takes precedence over the qualified Compute fallback.
    file(output.path(), "lib/libc.so.6", "not ELF");
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err()
    );
    std::fs::remove_file(output.path().join("lib/libc.so.6")).unwrap();

    let compile = |args: &[&str]| {
        let result = std::process::Command::new("cc")
            .args(args)
            .arg("-Wl,-rpath-link,lib")
            .current_dir(output.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    compile(&[
        "-shared",
        "-fPIC",
        "foo.c",
        "-Llib",
        "-lbar",
        "-Wl,-soname,libfoo.so",
        "-o",
        "lib/libfoo.so",
    ]);
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "parent RUNPATH incorrectly inherited by libfoo"
    );
    compile(&[
        "main.c",
        "-Llib",
        "-lfoo",
        "-Wl,--disable-new-dtags,-rpath,$ORIGIN/lib",
        "-o",
        "server",
    ]);
    validate_output(
        output.path(),
        output.path(),
        NativeRecipe::DartServer,
        Some("server"),
    )
    .unwrap();
    compile(&[
        "-shared",
        "-fPIC",
        "foo.c",
        "-Llib",
        "-lbar",
        "-Wl,-soname,libfoo.so",
        "-Wl,-rpath,$ORIGIN",
        "-o",
        "lib/libfoo.so",
    ]);
    compile(&[
        "main.c",
        "-Llib",
        "-lfoo",
        "-Wl,-rpath,$ORIGIN/lib",
        "-o",
        "server",
    ]);

    // Preserve both archive members in a relative SONAME symlink chain.
    std::fs::rename(
        output.path().join("lib/libfoo.so"),
        output.path().join("lib/libfoo.so.1"),
    )
    .unwrap();
    std::os::unix::fs::symlink("libfoo.so.1", output.path().join("lib/libfoo.so")).unwrap();
    let (_, members) = nrz_runtime_artifact::NativeExecutableRequirements::verify_artifact_closure(
        output.path(),
        &output.path().join("server"),
        output.path(),
    )
    .unwrap();
    assert!(members.contains(&std::path::PathBuf::from("lib/libfoo.so")));
    assert!(members.contains(&std::path::PathBuf::from("lib/libfoo.so.1")));
    std::fs::remove_file(output.path().join("lib/libfoo.so")).unwrap();
    std::fs::rename(
        output.path().join("lib/libfoo.so.1"),
        output.path().join("lib/libfoo.so"),
    )
    .unwrap();

    // The loader derives ORIGIN from the selected link name, not its target.
    std::fs::create_dir(output.path().join("vendor")).unwrap();
    for name in ["libfoo.so", "libbar.so"] {
        std::fs::rename(
            output.path().join("lib").join(name),
            output.path().join("vendor").join(name),
        )
        .unwrap();
    }
    std::os::unix::fs::symlink("../vendor/libfoo.so", output.path().join("lib/libfoo.so")).unwrap();
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "ORIGIN incorrectly used canonical library directory"
    );
    std::os::unix::fs::symlink("../vendor/libbar.so", output.path().join("lib/libbar.so")).unwrap();
    let (_, members) = nrz_runtime_artifact::NativeExecutableRequirements::verify_artifact_closure(
        output.path(),
        &output.path().join("server"),
        output.path(),
    )
    .unwrap();
    for name in [
        "lib/libfoo.so",
        "vendor/libfoo.so",
        "lib/libbar.so",
        "vendor/libbar.so",
    ] {
        assert!(
            members.contains(&std::path::PathBuf::from(name)),
            "missing {name}"
        );
    }
    for name in ["libfoo.so", "libbar.so"] {
        std::fs::remove_file(output.path().join("lib").join(name)).unwrap();
        std::fs::rename(
            output.path().join("vendor").join(name),
            output.path().join("lib").join(name),
        )
        .unwrap();
    }
    std::fs::remove_dir(output.path().join("vendor")).unwrap();

    std::fs::create_dir(output.path().join("bin")).unwrap();
    compile(&[
        "main.c",
        "-Llib",
        "-lfoo",
        "-Wl,-rpath,$ORIGIN/../lib",
        "-o",
        "bin/server",
    ]);
    validate_output(
        output.path(),
        output.path(),
        NativeRecipe::DartServer,
        Some("bin/server"),
    )
    .unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(outside.path().join("child")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("child"), output.path().join("liblink"))
        .unwrap();
    compile(&[
        "main.c",
        "-Llib",
        "-lfoo",
        "-Wl,-rpath,$ORIGIN/liblink/../lib",
        "-o",
        "server",
    ]);
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "symlink followed by parent was lexically erased"
    );
    std::fs::create_dir(output.path().join("unused")).unwrap();
    compile(&[
        "main.c",
        "-Llib",
        "-lfoo",
        "-Wl,-rpath,$ORIGIN/unused/../lib",
        "-o",
        "server",
    ]);
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "empty traversed directory would disappear from the archive"
    );
    compile(&[
        "main.c",
        "-Llib",
        "-lfoo",
        "-Wl,-rpath,$ORIGIN/lib",
        "-o",
        "server",
    ]);

    std::fs::rename(
        output.path().join("lib/libbar.so"),
        output.path().join("bar.saved"),
    )
    .unwrap();
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "missing transitive libbar was accepted"
    );
    std::fs::rename(
        output.path().join("bar.saved"),
        output.path().join("lib/libbar.so"),
    )
    .unwrap();
    std::fs::rename(
        output.path().join("lib/libfoo.so"),
        output.path().join("foo.saved"),
    )
    .unwrap();
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "missing direct libfoo was accepted"
    );
    std::fs::rename(output.path().join("lib"), output.path().join("lib.saved")).unwrap();
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::DartServer,
            Some("server")
        )
        .is_err(),
        "missing ORIGIN directory was accepted"
    );
}

#[test]
#[ignore = "requires pinned Flutter SDK and web engine cache; compiles a locked STATIC fixture"]
fn real_flutter_web_build_is_static_and_keeps_the_pub_lock() {
    let project = tempfile::tempdir().unwrap();
    copy_fixture(&fixture_root().join("flutter-web"), project.path());
    let lock = std::fs::read(project.path().join("pubspec.lock")).unwrap();
    let flutter = std::env::var("NRZ_FLUTTER_BIN").unwrap_or_else(|_| "flutter".into());
    let plan = recipe_commands(project.path(), NativeRecipe::FlutterWeb, false).unwrap();
    let install = install_command(project.path(), NativeRecipe::FlutterWeb)
        .unwrap()
        .unwrap();
    for command in [&install, &plan.build] {
        let result = std::process::Command::new(&flutter)
            .args(&command.arguments)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(
        std::fs::read(project.path().join("pubspec.lock")).unwrap(),
        lock
    );
    let output = project.path().join(&plan.output_directory);
    let evidence = validate_output(&output, &output, NativeRecipe::FlutterWeb, None).unwrap();
    assert!(evidence.target.is_none());
    for file in [
        "main.dart.js",
        "flutter_bootstrap.js",
        "assets/AssetManifest.bin.json",
    ] {
        assert!(output.join(file).is_file(), "missing STATIC output {file}");
    }
}

#[tokio::test]
#[ignore = "requires Hugo; compiles and cleans a real standard-edition STATIC fixture"]
async fn real_hugo_build_preserves_pages_and_static_assets() {
    let project = tempfile::tempdir().unwrap();
    copy_fixture(&fixture_root().join("hugo-static"), project.path());
    file(project.path(), "public/stale.txt", "stale output");
    let plan = recipe_commands(project.path(), NativeRecipe::HugoStatic, false).unwrap();
    crate::deploy::run_native_build_step(
        &plan,
        NativeRecipe::HugoStatic,
        project.path(),
        true,
        &hugo_tests::hugo_environment(project.path()),
        None,
        false,
    )
    .await
    .unwrap();
    let output = project.path().join(&plan.output_directory);
    assert!(
        validate_output(&output, &output, NativeRecipe::HugoStatic, None)
            .unwrap()
            .target
            .is_none()
    );
    assert!(output.join("about/index.html").is_file());
    assert_eq!(
        std::fs::read(output.join("fixture.svg")).unwrap(),
        std::fs::read(project.path().join("static/fixture.svg")).unwrap()
    );
    assert!(!output.join("stale.txt").exists());
}

#[test]
fn native_defaults_distinguish_preset_absence_from_explicit_user_skip() {
    use crate::config::{BuildSettingSource, EffectiveSettingOrigin, SourceAwareSetting};
    let setting = |source, value| SourceAwareSetting {
        value,
        source: Some(source),
        origin: EffectiveSettingOrigin::ServerSettings,
    };
    assert!(default_command(None, None));
    for source in [BuildSettingSource::Preset, BuildSettingSource::Detected] {
        assert!(default_command(None, Some(&setting(source, None))));
    }
    assert!(!default_command(
        None,
        Some(&setting(BuildSettingSource::User, None))
    ));
    assert!(!default_command(Some(""), None));
    assert!(!default_command(
        None,
        Some(&setting(
            BuildSettingSource::User,
            Some("make release".into())
        ))
    ));
}

#[test]
fn go_module_guard_rejects_dependency_input_changes_without_mutating_sources() {
    let project = tempfile::tempdir().unwrap();
    file(
        project.path(),
        "go.mod",
        "module example.org/server\ngo 1.27\n",
    );
    file(project.path(), "main.go", "package main");
    let mut plan = recipe_commands(project.path(), NativeRecipe::GoServer, false).unwrap();
    let guard = GoModuleInputs::freeze(project.path(), &mut plan.build).unwrap();
    guard.verify().unwrap();
    let copy = plan
        .build
        .arguments
        .iter()
        .find_map(|arg| arg.strip_prefix("-modfile="))
        .unwrap();
    std::fs::write(copy, "module changed").unwrap();
    assert!(guard.verify().is_err());
    assert_eq!(
        std::fs::read_to_string(project.path().join("go.mod")).unwrap(),
        "module example.org/server\ngo 1.27\n"
    );
    assert!(!project.path().join("go.sum").exists());
}

#[test]
fn dart_bundle_recipe_keeps_the_library_root_and_requires_a_lock() {
    let project = tempfile::tempdir().unwrap();
    file(project.path(), "pubspec.yaml", "name: server");
    file(project.path(), "bin/server.dart", "void main() {}");
    assert!(recipe_commands(project.path(), NativeRecipe::DartServer, true).is_err());
    file(project.path(), "pubspec.lock", "packages: {}");
    let plan = recipe_commands(project.path(), NativeRecipe::DartServer, true).unwrap();
    assert_eq!(plan.output_directory, "build/onreza-dart/bundle");
    assert_eq!(plan.executable_entry.as_deref(), Some("bin/server"));
    assert_eq!(plan.build.program, "dart");
    assert_eq!(
        plan.build.arguments,
        [
            "build",
            "cli",
            "--target",
            "bin/server.dart",
            "--target-os=linux",
            "--target-arch=x64",
            "--output=build/onreza-dart"
        ]
    );
}

#[test]
fn multiple_native_entries_require_an_explicit_build() {
    let project = tempfile::tempdir().unwrap();
    file(project.path(), "go.mod", "module example.org/server");
    file(project.path(), "cmd/a/main.go", "package main");
    file(project.path(), "cmd/b/main.go", "package main");
    assert!(
        recipe_commands(project.path(), NativeRecipe::GoServer, false)
            .unwrap_err()
            .to_string()
            .contains("one main package")
    );
}

#[test]
fn materialized_transitive_native_hooks_are_rejected_for_unqualified_local_cross_builds() {
    let project = tempfile::tempdir().unwrap();
    file(
        project.path(),
        ".dart_tool/package_config.json",
        r#"{"packages":[{"name":"fixture","rootUri":"../"}]}"#,
    );
    ensure_no_native_hooks(project.path()).unwrap();
    file(project.path(), "hook/build.dart", "void main() {}");
    assert!(
        ensure_no_native_hooks(project.path())
            .unwrap_err()
            .to_string()
            .contains("Linux Builder")
    );
}

#[test]
fn go_workspace_and_local_replacement_are_not_silently_dropped() {
    let project = tempfile::tempdir().unwrap();
    file(project.path(), "go.mod", "module example.org/server");
    file(project.path(), "main.go", "package main");
    file(project.path(), "go.work", "go 1.27\nuse .");
    assert!(
        recipe_commands(project.path(), NativeRecipe::GoServer, false)
            .unwrap_err()
            .to_string()
            .contains("workspace")
    );
    std::fs::remove_file(project.path().join("go.work")).unwrap();
    file(
        project.path(),
        "go.mod",
        "module example.org/server\nreplace example.org/other => ../other",
    );
    assert!(
        recipe_commands(project.path(), NativeRecipe::GoServer, false)
            .unwrap_err()
            .to_string()
            .contains("local replacement")
    );
}

#[test]
fn version_probes_observe_exact_toolchain_versions() {
    assert_eq!(
        compiler_version(
            NativeRecipe::DartServer,
            "Dart SDK version: 3.13.5 (stable) on linux_x64"
        )
        .unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../assets/native-toolchains.json"
        ))
        .unwrap()["dart"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        compiler_version(NativeRecipe::GoServer, "go version go1.27.1 linux/amd64").unwrap(),
        "1.27.1"
    );
    assert_eq!(
        compiler_version(
            NativeRecipe::HugoStatic,
            "hugo v0.167.0-abcdef linux/amd64 BuildDate=2026-09-28"
        )
        .unwrap(),
        "0.167.0"
    );
    assert_eq!(
        compiler_version(
            NativeRecipe::FlutterWeb,
            r#"{"frameworkVersion":"3.47.6","channel":"stable"}"#
        )
        .unwrap(),
        "3.47.6"
    );
    assert!(compiler_version(NativeRecipe::GoServer, "not go").is_err());
}

#[test]
fn hugo_nested_pages_do_not_require_a_home_page() {
    let output = tempfile::tempdir().unwrap();
    file(output.path(), "about/index.html", "<h1>About</h1>");
    assert!(!output.path().join("index.html").exists());
    let evidence =
        validate_output(output.path(), output.path(), NativeRecipe::HugoStatic, None).unwrap();
    assert!(evidence.entry.is_none());
    assert!(evidence.target.is_none());
    let manifest = crate::build::manifest::generate_static_manifest();
    crate::build::manifest::verify_files(output.path(), &manifest).unwrap();
    assert!(validate_output(output.path(), output.path(), NativeRecipe::FlutterWeb, None).is_err());
    assert!(
        validate_output(
            &output.path().join("missing"),
            &output.path().join("missing"),
            NativeRecipe::HugoStatic,
            None
        )
        .is_err()
    );
}

#[test]
fn static_and_native_outputs_are_checked_before_publication() {
    let output = tempfile::tempdir().unwrap();
    assert!(validate_output(output.path(), output.path(), NativeRecipe::FlutterWeb, None).is_err());
    file(output.path(), "index.html", "<html></html>");
    assert!(
        validate_output(output.path(), output.path(), NativeRecipe::FlutterWeb, None)
            .unwrap()
            .target
            .is_none()
    );
    file(output.path(), "server", "#!/bin/sh\necho ready");
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::GoServer,
            Some("server")
        )
        .is_err()
    );
    for entry in [
        "../server",
        "/server",
        "a/../server",
        "a\\server",
        "C:/server",
    ] {
        assert!(
            validate_output(
                output.path(),
                output.path(),
                NativeRecipe::GoServer,
                Some(entry)
            )
            .is_err()
        );
    }
}

#[cfg(unix)]
#[test]
fn native_entry_symlink_cannot_escape_the_output_root() {
    let output = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    file(outside.path(), "server", "ELF");
    std::os::unix::fs::symlink(outside.path().join("server"), output.path().join("server"))
        .unwrap();
    assert!(
        validate_output(
            output.path(),
            output.path(),
            NativeRecipe::GoServer,
            Some("server")
        )
        .unwrap_err()
        .to_string()
        .contains("escapes")
    );
}

#[tokio::test]
#[ignore = "requires a Go compiler; builds and executes the real native fixture"]
async fn real_go_executable_reads_artifact_assets_and_literal_argv() {
    let project = tempfile::tempdir().unwrap();
    file(
        project.path(),
        "go.mod",
        include_str!("../../tests/fixtures/native-go/go.mod"),
    );
    file(
        project.path(),
        "main.go",
        include_str!("../../tests/fixtures/native-go/main.go"),
    );
    file(
        project.path(),
        "assets/message.txt",
        include_str!("../../tests/fixtures/native-go/assets/message.txt"),
    );
    let mut plan = recipe_commands(project.path(), NativeRecipe::GoServer, false).unwrap();
    let inputs = GoModuleInputs::freeze(project.path(), &mut plan.build).unwrap();
    let go = std::env::var("NRZ_GO_BIN").unwrap_or_else(|_| plan.build.program.clone());
    let compiled = std::process::Command::new(go)
        .args(&plan.build.arguments)
        .env("GO111MODULE", "off")
        .envs(plan.environment.iter().cloned())
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    inputs.verify().unwrap();
    let output = project.path().join(&plan.output_directory);
    assert!(
        !output.join("assets").exists(),
        "go:embed must carry source assets inside the binary"
    );
    let evidence = validate_output(
        &output,
        &output,
        NativeRecipe::GoServer,
        plan.executable_entry.as_deref(),
    )
    .unwrap();
    assert_eq!(evidence.target, Some(NATIVE_RUNTIME_TARGET));
    assert_eq!(evidence.entry.as_deref(), Some("server"));
    assert!(evidence.interpreter.is_none());
    assert!(evidence.libraries.is_empty());
    assert!(evidence.library_paths.is_empty());
    let result = execute_verified_native_source_bundle(&output, "server", "go").await;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(body["message"], "native asset ready\n");
    assert_eq!(body["args"], serde_json::json!(["$(id)", "two words"]));
}

#[tokio::test]
#[ignore = "requires Linux, pinned Dart 3.13.5 and a C compiler; executes the archived native bundle"]
async fn real_dart_bundle_reads_artifact_assets_and_literal_argv() {
    let project = tempfile::tempdir().unwrap();
    copy_fixture(&fixture_root().join("native-dart"), project.path());
    let lock = std::fs::read(project.path().join("pubspec.lock")).unwrap();
    let dart = std::env::var("NRZ_DART_BIN").unwrap_or_else(|_| "dart".into());
    let version = std::process::Command::new(&dart)
        .arg("--version")
        .output()
        .unwrap();
    let observed = format!(
        "{}{}",
        String::from_utf8_lossy(&version.stdout),
        String::from_utf8_lossy(&version.stderr)
    );
    assert_eq!(
        compiler_version(NativeRecipe::DartServer, &observed).unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../assets/native-toolchains.json"
        ))
        .unwrap()["dart"]
            .as_str()
            .unwrap()
    );
    let plan = recipe_commands(project.path(), NativeRecipe::DartServer, false).unwrap();
    let install = install_command(project.path(), NativeRecipe::DartServer)
        .unwrap()
        .unwrap();
    for command in [&install, &plan.build] {
        let result = std::process::Command::new(&dart)
            .args(&command.arguments)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let output = project.path().join(&plan.output_directory);
    assert_eq!(
        std::fs::read(project.path().join("pubspec.lock")).unwrap(),
        lock
    );
    assert!(output.join("lib/libfixture.so").is_file());
    file(
        &output,
        "assets/message.txt",
        include_str!("../../tests/fixtures/native-dart/assets/message.txt"),
    );
    let evidence = validate_output(
        &output,
        &output,
        NativeRecipe::DartServer,
        plan.executable_entry.as_deref(),
    )
    .unwrap();
    assert_eq!(evidence.target, Some(NATIVE_RUNTIME_TARGET));
    let result =
        execute_verified_native_source_bundle(&output, &evidence.entry.unwrap(), "dart").await;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(body["message"], "native asset ready\n");
    assert_eq!(body["args"], serde_json::json!(["$(id)", "two words"]));
}

#[path = "native_hugo_tests.rs"]
mod hugo_tests;

#[test]
fn go_discovery_and_default_recipe_share_the_qualified_file_selection() {
    for (case, eligible, ambiguous) in [
        ("windows-only", false, false),
        ("eligible-cmd", true, false),
        ("ambiguous", true, true),
    ] {
        let project = tempfile::tempdir().unwrap();
        file(
            project.path(),
            "go.mod",
            "module example.com/app\ngo 1.27\n",
        );
        file(
            project.path(),
            "main_windows.go",
            "package main\nfunc main() {}\n",
        );
        if eligible {
            file(
                project.path(),
                "cmd/windows/main_windows.go",
                "package main\nfunc main() {}\n",
            );
            file(
                project.path(),
                "cmd/cgo/main.go",
                "package main\nimport \"C\"\nfunc main() {}\n",
            );
            file(
                project.path(),
                "cmd/server/service_linux_amd64.go",
                "package main\nfunc main() {}\n",
            );
        }
        if ambiguous {
            file(
                project.path(),
                "service_linux_amd64.go",
                "package main\nfunc main() {}\n",
            );
        }
        let detected =
            crate::detect::native::detect_native(&crate::detect::fs::LocalFs::new(project.path()));
        assert_eq!(detected.is_some(), eligible, "{case}");
        if let Some(detected) = detected {
            assert_eq!(detected.framework, "go");
            assert_eq!(
                detected.suggested_compute,
                crate::detect::types::ComputeType::Process
            );
        }
        let plan = recipe_commands(project.path(), NativeRecipe::GoServer, false);
        assert_eq!(plan.is_ok(), eligible && !ambiguous, "{case}");
        if let Ok(plan) = plan {
            assert_eq!(plan.build.arguments.last().unwrap(), "./cmd/server");
            for (key, value) in [
                ("GOOS", "linux"),
                ("GOARCH", "amd64"),
                ("GOAMD64", "v1"),
                ("CGO_ENABLED", "0"),
                ("GOEXPERIMENT", ""),
            ] {
                assert!(
                    plan.environment.contains(&(key.into(), value.into())),
                    "{case}: {key}"
                );
            }
        }
    }
}
