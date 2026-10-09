use super::*;
#[cfg(unix)]
use nrz_source_bundle::{ApplicationRuntimeFamily, BuildToolchainFamily};

#[test]
fn generated_python_state_is_gitignored_for_manual_project_configuration() {
    let project = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new("git");
    super::super::ignored_build::remove_git_repository_environment(&mut command);
    assert!(
        command
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
        let mut command = std::process::Command::new("git");
        super::super::ignored_build::remove_git_repository_environment(&mut command);
        let ignored = command
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
    crate::deploy::test_support::write_project_config(project.path(), &config);
    let (command, args) =
        crate::deploy::test_support::deploy_context(project.path(), &config, &["--skip-install"]);
    let result = crate::deploy::test_support::build_plan(&args, &command, &[]).await;
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
        ("pure_mz.py", b"MZ = 42\nprint(MZ)\n".as_slice(), false),
        (
            "Java.class",
            b"\xca\xfe\xba\xbe\0\0\0\x3d\0\x1b\x0a\0\x02\0\x03\x07".as_slice(),
            false,
        ),
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
        assert_eq!(
            validate_retained_python_native_platform(&artifact, &files).is_err(),
            matches!(
                path,
                "native-opaque.data" | ".onreza/python/3.14/site-packages/dependency/native.so"
            ),
            "platform content custody: {path}"
        );
    }
}

fn scanned_python_output(
    project_dir: &std::path::Path,
) -> (
    crate::artifact::RuntimeArtifact,
    nrz_runtime_artifact::ArtifactRoot,
    crate::artifact::ArtifactFileCollection,
) {
    let artifact = crate::artifact::RuntimeArtifact {
        root_dir: project_dir.to_owned(),
        manifest: serde_json::from_value(serde_json::json!({
            "version":1,"routes":[],"layers":[{
                "name":"python","target":"COMPUTE","directory":".","entry":"main.py",
                "runtime":{"applicationRuntime":{"family":"PYTHON","args":[]},"buildRuntimeVersion":"python-3.14"}
            }]
        })).unwrap(),
        scan: crate::artifact::RuntimeArtifactScan::PythonRuntimeRoot(PythonMinor::default()),
    };
    let root = nrz_runtime_artifact::ArtifactRoot::open(project_dir).unwrap();
    let files = crate::artifact::classify_artifact_files(
        &artifact.manifest,
        super::super::scan::scan_runtime_artifact_rooted(&root, &artifact.scan).unwrap(),
        &crate::detect::detect(project_dir),
        crate::artifact::ArtifactRootScope::ProjectRoot,
        &artifact.scan,
    );
    (artifact, root, files)
}

#[test]
fn python_qualification_rejects_changed_bytes_before_native_payload_is_restored() {
    let mut admitted = Vec::new();
    for guard in ["platform", "local", "authored"] {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("main.py"), "print('PYTHON')\n").unwrap();
        let relative = if guard == "authored" {
            format!(
                "{}/demo/native.data",
                PythonMinor::default().site_packages_root()
            )
        } else {
            "native.data".to_owned()
        };
        let path = project.path().join(&relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut original = b"\xcf\xfa\xed\xfeMACH_O".to_vec();
        original.resize(96, 0);
        std::fs::write(&path, &original).unwrap();
        let (artifact, root, files) = scanned_python_output(project.path());
        std::fs::write(&path, vec![b'p'; original.len()]).unwrap();
        let qualification = match guard {
            "platform" => validate_retained_python_native_platform_rooted(&root, &artifact, &files),
            "local" => validate_local_python_build_output_rooted(&root, &artifact, &files),
            "authored" => validate_authored_python_dependency_output_rooted(
                &root,
                &artifact,
                &files,
                PythonInstallMode::ManagedLocal,
            ),
            _ => unreachable!(),
        };
        std::fs::write(&path, &original).unwrap();
        if let Err(error) = qualification {
            assert!(
                error.to_string().contains("changed after scanning"),
                "{guard}"
            );
        } else {
            let plan = crate::artifact::source_bundle_v1::build_source_bundle_plan_with_root(
                &root,
                &artifact.manifest,
                &files.deployable_entries(),
                &artifact.scan,
                crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded,
                None,
            )
            .unwrap();
            let restored = crate::test_support::unpack_source_bundle(&plan);
            assert_eq!(
                std::fs::read(restored.path().join(&relative)).unwrap(),
                original
            );
            admitted.push(guard);
        }
    }
    assert!(
        admitted.is_empty(),
        "{admitted:?} archived a native payload after qualifying different bytes"
    );
}

#[cfg(unix)]
#[test]
fn python_qualification_preserves_scanned_file_and_directory_aliases() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.py"), "print('PYTHON')\n").unwrap();
    std::os::unix::fs::symlink("main.py", project.path().join("alias.py")).unwrap();
    let stage = project
        .path()
        .join(PythonMinor::default().site_packages_root());
    std::fs::create_dir_all(stage.join("resources")).unwrap();
    std::fs::write(stage.join("resources/pure.py"), "RESOURCE = 1\n").unwrap();
    std::os::unix::fs::symlink("resources/pure.py", stage.join("alias.py")).unwrap();
    std::os::unix::fs::symlink("resources", stage.join("folder.so")).unwrap();
    std::os::unix::fs::symlink(
        PythonMinor::default().site_packages_root(),
        project.path().join("folder.so"),
    )
    .unwrap();
    let (artifact, root, files) = scanned_python_output(project.path());
    validate_retained_python_native_platform_rooted(&root, &artifact, &files).unwrap();
    validate_local_python_build_output_rooted(&root, &artifact, &files).unwrap();
    validate_authored_python_dependency_output_rooted(
        &root,
        &artifact,
        &files,
        PythonInstallMode::ManagedLocal,
    )
    .unwrap();
    let alias = files
        .files
        .iter()
        .find(|file| file.path == "alias.py")
        .unwrap();
    std::fs::write(project.path().join("main.py"), "print('CHANGED')\n").unwrap();
    let error = qualify_retained_python_file(&root, alias, &files, |reader| {
        nrz_runtime_artifact::verify_linux_x86_64_native_platform(reader)?;
        Ok(())
    })
    .unwrap_err();
    assert!(
        error.to_string().contains("changed after scanning"),
        "{error}"
    );
}

#[test]
fn pure_python_and_java_stage_resources_remain_portable_on_non_linux_hosts() {
    let project = tempfile::tempdir().unwrap();
    let minor = PythonMinor::default();
    let directory = project.path().join(minor.site_packages_root()).join("demo");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("pure_mz.py"), "MZ = 42\nprint(MZ)\n").unwrap();
    let mut java = b"\xca\xfe\xba\xbe\0\0\0\x3d\0\x1b\x0a\0\x02\0\x03\x07".to_vec();
    java.resize(96, 0);
    std::fs::write(directory.join("Java.class"), java).unwrap();
    for host in [("macos", "aarch64"), ("windows", "x86_64")] {
        validate_local_build_dependency_host(
            project.path(),
            PythonInstallMode::ManagedLocal,
            minor,
            host,
        )
        .unwrap();
    }
}
