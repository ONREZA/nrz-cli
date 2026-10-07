use super::*;
use clap::Parser as _;

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
