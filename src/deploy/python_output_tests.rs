use super::*;
#[cfg(unix)]
use clap::Parser as _;
#[cfg(unix)]
use nrz_source_bundle::{ApplicationRuntimeFamily, BuildToolchainFamily};

#[test]
fn generated_python_state_is_gitignored_for_manual_project_configuration() {
    let project = tempfile::tempdir().unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .arg(project.path())
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(project.path().join(".gitignore"), "dist/\n").unwrap();
    let state = project.path().join(".onreza/python/build/startup");
    ensure_python_directory(project.path(), &state).unwrap();
    std::fs::write(state.join("sitecustomize.py"), "GENERATED_BUILD_STATE").unwrap();
    let launch = crate::detect::python_launch::PythonLaunch {
        entry: crate::detect::python_launch::PYTHON_BOOTSTRAP_ENTRY.into(),
        args: vec!["MODULE".into(), "main".into()],
    };
    super::super::python_launch::materialize_python_entry(
        project.path(),
        &launch,
        PythonMinor::default(),
    )
    .unwrap();
    for path in [
        ".onreza/python/build/startup/sitecustomize.py",
        ".onreza/python/launch.py",
    ] {
        let ignored = std::process::Command::new("git")
            .arg("-C")
            .arg(project.path())
            .args(["check-ignore", path])
            .output()
            .unwrap();
        assert!(
            ignored.status.success(),
            "generated state remains visible to Git: {path}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(project.path().join(".gitignore")).unwrap(),
        "dist/\n.onreza/\n"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn authored_local_build_rejects_python_owned_native_output() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("main.py"),
        "print('CODE_ONLY_PYTHON')\n",
    )
    .unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    // A Python serving owner is authoritative even when the authored compiler
    // is another family; no interpreter download is needed for this command.
    config.build.toolchain = Some(BuildToolchainFamily::Node);
    config.build.output_directory = Some(".".into());
    config.build.command = Some("printf '\\317\\372\\355\\376HOST_MACH_O' > native-host.so".into());
    config.deploy.runtime = Some(ApplicationRuntimeFamily::Python);
    config.deploy.entry = Some("main.py".into());
    std::fs::write(
        project.path().join("onreza.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    let command =
        crate::context::CommandContext::resolve_platform_root(project.path(), &config, true)
            .unwrap();
    let args = crate::cli::DeployArgs::try_parse_from([
        "deploy",
        project.path().to_str().unwrap(),
        "--dry",
        "--skip-install",
    ])
    .unwrap();
    let result = super::super::plan::build(super::super::plan::DeployPlanRequest {
        args: &args,
        command: &command,
        explicit_compute: None,
        build_logs: None,
        execution_env: &[],
        target_production: None,
        platform_runner: false,
    })
    .await;
    assert!(
        project.path().join("native-host.so").is_file(),
        "authored build was not executed"
    );
    let error = result
        .err()
        .expect("local Python application build published an unqualified native extension");
    assert!(error.to_string().contains("native payload"), "{error:#}");
    assert!(
        error.to_string().contains("ONREZA Cloud Builder"),
        "{error:#}"
    );
}

#[test]
fn python_output_guard_uses_retained_layer_and_dependency_custody() {
    use crate::artifact::{ArtifactRootScope, RuntimeArtifact, RuntimeArtifactScan};
    for (path, bytes, rejected) in [
        ("native-host.so", b"HOST_EXTENSION".as_slice(), true),
        (
            "native-opaque.data",
            b"\xcf\xfa\xed\xfeMACH_O".as_slice(),
            true,
        ),
        ("pure.py", b"print('PURE_OUTPUT')".as_slice(), false),
        (
            "static/native-host.so",
            b"STATIC_DOWNLOAD".as_slice(),
            false,
        ),
        ("node/native-host.so", b"NON_PYTHON_OWNER".as_slice(), false),
        (
            ".onreza/python/build/native-host.so",
            b"BUILD_ONLY".as_slice(),
            false,
        ),
        (
            ".onreza/python/3.14/site-packages/dependency/native.so",
            b"\x7fELFNATIVE_TARGET_STAGE".as_slice(),
            false,
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("main.py"), "print('PYTHON')\n").unwrap();
        let payload = project.path().join(path);
        std::fs::create_dir_all(payload.parent().unwrap()).unwrap();
        std::fs::write(&payload, bytes).unwrap();
        let manifest = serde_json::from_value(serde_json::json!({
            "version": 1,
            "layers": [
                {"name":"python","target":"COMPUTE","directory":".","entry":"main.py","runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.14"}},
                {"name":"node","target":"COMPUTE","directory":"node","entry":"server.js","runtime":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-22"}},
                {"name":"static","target":"STATIC","directory":"static"}
            ],
            "routes": [{"pattern":"^/.*$","layer":"python"}]
        })).unwrap();
        let artifact = RuntimeArtifact {
            root_dir: project.path().to_path_buf(),
            manifest,
            scan: RuntimeArtifactScan::PythonRuntimeRoot(PythonMinor::default()),
        };
        let scanned =
            super::super::scan::scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
        let files = crate::artifact::classify_artifact_files(
            &artifact.manifest,
            scanned,
            &crate::detect::detect(project.path()),
            ArtifactRootScope::ProjectRoot,
            &artifact.scan,
        );
        assert_eq!(
            validate_local_python_build_output(&artifact, &files).is_err(),
            rejected,
            "{path}"
        );
    }
}
