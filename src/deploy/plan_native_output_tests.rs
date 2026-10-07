use super::*;
use clap::Parser as _;

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[tokio::test]
#[ignore = "requires C compiler; executes an authored native layer and its extracted source bundle"]
async fn real_native_subdirectory_closure_uses_full_artifact_root() {
    let fixture = tempfile::tempdir().unwrap();
    let output = fixture.path().join("dist");
    std::fs::create_dir_all(output.join("bin")).unwrap();
    std::fs::create_dir_all(output.join("lib")).unwrap();
    std::fs::create_dir_all(output.join(".onreza")).unwrap();
    std::fs::write(
        fixture.path().join("provider.c"),
        "int answer(void){return 42;}\n",
    )
    .unwrap();
    std::fs::write(fixture.path().join("main.c"), "#include <stdio.h>\nextern int answer(void);\nint main(int n,char **a){char b[32];FILE *f=fopen(\"asset.txt\",\"r\");if(!f)return 3;fgets(b,32,f);fclose(f);printf(\"%d:%s:%s|%s\\n\",answer(),b,n>1?a[1]:\"\",n>2?a[2]:\"\");return 0;}\n").unwrap();
    std::fs::write(output.join("bin/asset.txt"), "ASSET").unwrap();
    for args in [
        vec![
            "-shared",
            "-fPIC",
            "provider.c",
            "-Wl,-soname,libanswer.so",
            "-o",
            "dist/lib/libanswer.so",
        ],
        vec![
            "main.c",
            "-Ldist/lib",
            "-lanswer",
            "-Wl,-rpath,$ORIGIN/../lib",
            "-o",
            "dist/bin/server",
        ],
    ] {
        let result = std::process::Command::new("cc")
            .args(args)
            .current_dir(fixture.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let manifest = serde_json::json!({
        "version": 1, "routes": [{"pattern": "^/.*$", "layer": "server"}],
        "layers": [{"name": "server", "target": "COMPUTE", "directory": "bin", "entry": "server",
            "runtime": {"applicationRuntime": {"family": "EXECUTABLE", "args": ["$(id)", "two words"]},
                        "buildRuntimeVersion": "native-linux-x86_64-glibc"}}]
    });
    std::fs::write(
        output.join(".onreza/manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    for framework in ["go", "dart"] {
        for skip_build in [true, false] {
            let project = tempfile::tempdir().unwrap();
            copy_native_output_directory(fixture.path(), project.path());
            let mut config = crate::config::ProjectConfig::default();
            config.project.framework = Some(framework.into());
            config.build.output_directory = Some("dist".into());
            config.build.command = Some("true".into());
            config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Executable);
            config.deploy.entry = Some("bin/server".into());
            config.deploy.args = Some(vec!["$(id)".into(), "two words".into()]);
            let command = crate::context::CommandContext::resolve_platform_root(
                project.path(),
                &config,
                true,
            )
            .unwrap();
            let mut args = DeployArgs::try_parse_from([
                "deploy",
                project.path().to_str().unwrap(),
                "--dry",
                "--skip-install",
            ])
            .unwrap();
            args.skip_build = skip_build;
            let plan = build(DeployPlanRequest {
                args: &args,
                command: &command,
                explicit_compute: None,
                build_logs: None,
                execution_env: &[],
                target_production: None,
                platform_runner: false,
            })
            .await
            .unwrap();
            let source = plan
                .materialize_source_bundle(true, RuntimeDependencyPackaging::Embedded)
                .unwrap();
            let logical: nrz_source_bundle::SourceLogicalManifest =
                serde_json::from_value(serde_json::to_value(&source.logical_manifest).unwrap())
                    .unwrap();
            let owner = uuid::Uuid::nil().to_string();
            let input = nrz_source_bundle::SourceBundleVerificationInput {
                owner_workspace_id: owner.clone(),
                source_artifact_id: nrz_source_bundle::compute_source_artifact_id(
                    &owner,
                    &source.logical_manifest_sha256,
                    &source.source_sha256,
                    None,
                ),
                source_sha256: source.source_sha256.clone(),
                logical_manifest_sha256: source.logical_manifest_sha256.clone(),
                budget: nrz_source_bundle::SourceBundleVerificationBudget::from_manifest(&logical)
                    .unwrap(),
            };
            let verified = nrz_source_bundle::verify_source_bundle_bytes(
                input,
                std::fs::read(source.source_path()).unwrap().into(),
            )
            .await
            .unwrap();
            let logical: nrz_source_bundle::SourceLogicalManifest =
                serde_json::from_value(verified.logical_manifest).unwrap();
            let targets = std::collections::HashMap::from([(
                "server".into(),
                "native-linux-x86_64-glibc".into(),
            )]);
            let graph =
                nrz_runtime_artifact::finalize_source_bundle_runtime_graph_for_layer_targets(
                    &source.logical_manifest_sha256,
                    &source.source_sha256,
                    source.source_size_bytes,
                    &logical,
                    &[],
                    &targets,
                )
                .unwrap();
            nrz_runtime_artifact::verify_source_runtime_graph_dependencies(&logical, &graph)
                .unwrap();
            let runtime = &graph.wire().runtime_layers[0];
            let unpacked = tempfile::tempdir().unwrap();
            let decoder = zstd::stream::read::Decoder::new(
                std::fs::File::open(source.source_path()).unwrap(),
            )
            .unwrap();
            tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
            let cwd = unpacked.path().join(runtime.application_root.as_str());
            let entry = cwd.join(runtime.entrypoint.as_str());
            nrz_runtime_artifact::NativeExecutableRequirements::verify_artifact_closure(
                unpacked.path(),
                &entry,
                &cwd,
            )
            .unwrap();
            let result = std::process::Command::new(entry)
                .args(
                    runtime
                        .launch
                        .as_ref()
                        .unwrap()
                        .args
                        .iter()
                        .map(|arg| arg.as_str()),
                )
                .current_dir(cwd)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(result.stdout, b"42:ASSET:$(id)|two words\n");
            std::fs::rename(
                project.path().join("dist/lib"),
                project.path().join("outside-lib"),
            )
            .unwrap();
            std::os::unix::fs::symlink("../outside-lib", project.path().join("dist/lib")).unwrap();
            let error = build(DeployPlanRequest {
                args: &args,
                command: &command,
                explicit_compute: None,
                build_logs: None,
                execution_env: &[],
                target_production: None,
                platform_runner: false,
            })
            .await
            .err()
            .expect("library outside the artifact was accepted");
            assert!(error.to_string().contains("escape"), "{error:#}");
        }
    }
}

#[tokio::test]
async fn native_default_equivalent_output_reaches_compiler_admission() {
    for framework in ["go", "dart", "flutter", "hugo"] {
        let project = tempfile::tempdir().unwrap();
        let fixture = match framework {
            "go" => "native-go",
            "dart" => "native-dart",
            "flutter" => "flutter-web",
            _ => "hugo-static",
        };
        copy_native_output_fixture(fixture, project.path());
        let recipe = crate::detect::native::native_recipe(framework).unwrap();
        let mut config = crate::config::ProjectConfig::default();
        config.project.framework = Some(framework.into());
        config.build.output_directory = Some(format!("./{}", recipe.output_directory()));
        let command =
            crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
                .unwrap();
        let args = DeployArgs::try_parse_from([
            "deploy",
            project.path().to_str().unwrap(),
            "--dry",
            "--skip-install",
        ])
        .unwrap();
        let tools = tempfile::tempdir().unwrap();
        let environment = [("PATH".into(), tools.path().to_string_lossy().into_owned())];
        let error = build(DeployPlanRequest {
            args: &args,
            command: &command,
            explicit_compute: None,
            build_logs: None,
            execution_env: &environment,
            target_production: None,
            platform_runner: false,
        })
        .await
        .err()
        .expect("missing compiler must fail at compiler admission");
        let compiler = super::super::native_build::compiler_probe(recipe).program;
        assert!(
            error
                .to_string()
                .contains(&format!("{compiler} is unavailable")),
            "{framework}: {error:#}"
        );
        assert!(!project.path().join(recipe.output_directory()).exists());
    }
}

fn copy_native_output_fixture(fixture: &str, destination: &Path) {
    let source = std::env::var_os("NRZ_QUALIFICATION_SOURCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .join("tests/fixtures")
        .join(fixture);
    copy_native_output_directory(&source, destination);
}

fn copy_native_output_directory(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let output = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_native_output_directory(&entry.path(), &output);
        } else {
            std::fs::copy(entry.path(), output).unwrap();
        }
    }
}

#[tokio::test]
#[ignore = "requires qualified Hugo; exercises planner, default build and source publication"]
async fn real_hugo_equivalent_output_directory_publishes_fresh_default_output() {
    let project = tempfile::tempdir().unwrap();
    copy_native_output_fixture("hugo-static", project.path());
    std::fs::create_dir(project.path().join("public")).unwrap();
    std::fs::write(project.path().join("public/stale.txt"), "STALE_OUTPUT").unwrap();
    let mut config = crate::config::ProjectConfig::default();
    config.project.framework = Some("hugo".into());
    config.build.output_directory = Some("./public".into());
    let command =
        crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
            .unwrap();
    let args = DeployArgs::try_parse_from([
        "deploy",
        project.path().to_str().unwrap(),
        "--dry",
        "--skip-install",
    ])
    .unwrap();
    let hugo = std::env::var_os("NRZ_HUGO_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| "hugo".into());
    let mut paths = vec![hugo.parent().unwrap_or(Path::new(".")).to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let environment = [(
        "PATH".into(),
        std::env::join_paths(paths)
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    )];
    let plan = build(DeployPlanRequest {
        args: &args,
        command: &command,
        explicit_compute: None,
        build_logs: None,
        execution_env: &environment,
        target_production: None,
        platform_runner: false,
    })
    .await
    .unwrap();
    let source = plan
        .materialize_source_bundle(true, RuntimeDependencyPackaging::Embedded)
        .unwrap();
    let unpacked = tempfile::tempdir().unwrap();
    let decoder =
        zstd::stream::read::Decoder::new(std::fs::File::open(source.source_path()).unwrap())
            .unwrap();
    tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
    assert!(unpacked.path().join("about/index.html").is_file());
    assert_eq!(
        std::fs::read(unpacked.path().join("fixture.svg")).unwrap(),
        std::fs::read(project.path().join("static/fixture.svg")).unwrap(),
    );
    assert!(!unpacked.path().join("stale.txt").exists());
}

#[tokio::test]
async fn native_default_output_conflict_is_rejected_before_install_or_cleanup() {
    for framework in ["go", "dart", "flutter", "hugo"] {
        let project = tempfile::tempdir().unwrap();
        let stale = project.path().join("custom-output");
        std::fs::create_dir(&stale).unwrap();
        std::fs::write(stale.join("old.txt"), "STALE_OUTPUT").unwrap();
        let mut config = crate::config::ProjectConfig::default();
        config.project.framework = Some(framework.into());
        config.build.output_directory = Some("custom-output".into());
        let command =
            crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
                .unwrap();
        let args =
            DeployArgs::try_parse_from(["deploy", project.path().to_str().unwrap(), "--dry"])
                .unwrap();
        let error = build(DeployPlanRequest {
            args: &args,
            command: &command,
            explicit_compute: None,
            build_logs: None,
            execution_env: &[],
            target_production: None,
            platform_runner: false,
        })
        .await
        .err()
        .expect("conflicting output must fail before tool execution");
        assert!(
            error.to_string().contains("build.output_directory"),
            "{framework}: {error:#}"
        );
        assert!(
            error.to_string().contains("build.command"),
            "{framework}: {error:#}"
        );
        assert_eq!(
            std::fs::read_to_string(stale.join("old.txt")).unwrap(),
            "STALE_OUTPUT"
        );
        assert!(!project.path().join(".onreza").exists());
    }
}

#[tokio::test]
async fn native_default_unsafe_output_is_rejected_before_cleanup() {
    let mut cases = vec![
        ("hugo", "../public"),
        ("hugo", "public/../public"),
        ("hugo", "/public"),
        ("hugo", "C:\\public"),
        ("hugo", "public\\"),
        ("hugo", "./other"),
    ];
    if cfg!(unix) {
        cases.extend([
            ("go", "build\\onreza-go"),
            ("dart", "build\\onreza-dart\\bundle"),
            ("flutter", "build\\web"),
        ]);
    }
    for (framework, output) in cases {
        let project = tempfile::tempdir().unwrap();
        copy_native_output_fixture(
            match framework {
                "go" => "native-go",
                "dart" => "native-dart",
                "flutter" => "flutter-web",
                _ => "hugo-static",
            },
            project.path(),
        );
        std::fs::create_dir(project.path().join("public")).unwrap();
        std::fs::write(project.path().join("public/stale.txt"), "PRESERVED").unwrap();
        let mut config = crate::config::ProjectConfig::default();
        config.project.framework = Some(framework.into());
        config.build.output_directory = Some(output.into());
        let command =
            crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
                .unwrap();
        let args = DeployArgs::try_parse_from([
            "deploy",
            project.path().to_str().unwrap(),
            "--dry",
            "--skip-install",
        ])
        .unwrap();
        let tools = tempfile::tempdir().unwrap();
        let environment = [("PATH".into(), tools.path().to_string_lossy().into_owned())];
        let error = build(DeployPlanRequest {
            args: &args,
            command: &command,
            explicit_compute: None,
            build_logs: None,
            execution_env: &environment,
            target_production: None,
            platform_runner: false,
        })
        .await
        .err()
        .expect("unsafe/different output must fail before compiler admission");
        assert!(
            error.to_string().contains("build.output_directory"),
            "{framework} {output}: wrong boundary: {error:#}"
        );
        assert_eq!(
            std::fs::read_to_string(project.path().join("public/stale.txt")).unwrap(),
            "PRESERVED"
        );
        assert!(!project.path().join(".onreza").exists());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn custom_native_command_and_prebuilt_output_keep_the_authored_directory() {
    for skip_build in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let output = project.path().join("custom-output");
        std::fs::create_dir(&output).unwrap();
        std::fs::write(output.join("index.html"), "AUTHORED_STATIC_OUTPUT").unwrap();
        let mut config = crate::config::ProjectConfig::default();
        config.project.framework = Some("hugo".into());
        config.build.output_directory = Some("custom-output".into());
        if !skip_build {
            config.build.command = Some("printf BUILT > custom-output/build-marker.txt".into());
        }
        let command =
            crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
                .unwrap();
        let mut args = DeployArgs::try_parse_from([
            "deploy",
            project.path().to_str().unwrap(),
            "--dry",
            "--skip-install",
        ])
        .unwrap();
        args.skip_build = skip_build;
        let plan = build(DeployPlanRequest {
            args: &args,
            command: &command,
            explicit_compute: None,
            build_logs: None,
            execution_env: &[],
            target_production: None,
            platform_runner: false,
        })
        .await
        .unwrap();
        let source = plan
            .materialize_source_bundle(true, RuntimeDependencyPackaging::Embedded)
            .unwrap();
        let unpacked = tempfile::tempdir().unwrap();
        let decoder =
            zstd::stream::read::Decoder::new(std::fs::File::open(source.source_path()).unwrap())
                .unwrap();
        tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(unpacked.path().join("index.html")).unwrap(),
            "AUTHORED_STATIC_OUTPUT"
        );
        if !skip_build {
            assert_eq!(
                std::fs::read_to_string(unpacked.path().join("build-marker.txt")).unwrap(),
                "BUILT"
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn native_default_recipe_cannot_publish_stale_custom_output() {
    use std::os::unix::fs::PermissionsExt as _;
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("custom-output")).unwrap();
    std::fs::write(
        project.path().join("custom-output/index.html"),
        "STALE_HTML",
    )
    .unwrap();
    let tools = tempfile::tempdir().unwrap();
    let pins: serde_json::Value =
        serde_json::from_str(include_str!("../../assets/native-toolchains.json")).unwrap();
    let compiler = tools.path().join("hugo");
    std::fs::write(&compiler, format!(
        "#!/bin/sh\nset -eu\nif [ \"$1\" = version ]; then\n echo 'hugo v{} linux/amd64'\nelse\n /bin/mkdir -p public\n printf FRESH_HTML > public/index.html\n printf EXECUTED > compiler-marker.txt\nfi\n", pins["hugo"].as_str().unwrap(),
    )).unwrap();
    std::fs::set_permissions(&compiler, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut config = crate::config::ProjectConfig::default();
    config.project.framework = Some("hugo".into());
    config.build.output_directory = Some("custom-output".into());
    let command =
        crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
            .unwrap();
    let args = DeployArgs::try_parse_from([
        "deploy",
        project.path().to_str().unwrap(),
        "--dry",
        "--skip-install",
    ])
    .unwrap();
    let environment = [("PATH".into(), tools.path().to_string_lossy().into_owned())];
    let error = build(DeployPlanRequest {
        args: &args,
        command: &command,
        explicit_compute: None,
        build_logs: None,
        execution_env: &environment,
        target_production: None,
        platform_runner: false,
    })
    .await
    .err()
    .expect("default recipe must reject a different stale output directory");
    assert!(
        error.to_string().contains("build.output_directory"),
        "{error:#}"
    );
    assert!(!project.path().join("compiler-marker.txt").exists());
    assert!(!project.path().join("public").exists());
    assert_eq!(
        std::fs::read_to_string(project.path().join("custom-output/index.html")).unwrap(),
        "STALE_HTML"
    );
}
