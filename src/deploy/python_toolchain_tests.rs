use std::ffi::OsString;

use super::python_toolchain::{
    ArchiveFormat, PythonInstallMode, artifact_for, install_command, install_commands,
};

#[test]
fn pins_uv_artifacts_for_every_cli_release_platform() {
    let cases = [
        (
            "linux",
            "x86_64",
            "x86_64-unknown-linux-musl",
            ArchiveFormat::TarGz,
        ),
        (
            "macos",
            "x86_64",
            "x86_64-apple-darwin",
            ArchiveFormat::TarGz,
        ),
        (
            "macos",
            "aarch64",
            "aarch64-apple-darwin",
            ArchiveFormat::TarGz,
        ),
        (
            "windows",
            "x86_64",
            "x86_64-pc-windows-msvc",
            ArchiveFormat::Zip,
        ),
    ];

    for (os, arch, target, format) in cases {
        let artifact = artifact_for(os, arch).unwrap();
        assert_eq!(artifact.target, target);
        assert_eq!(artifact.format, format);
        assert_eq!(artifact.archive_sha256.len(), 64);
        assert_eq!(artifact.binary_sha256.len(), 64);
    }
    assert!(artifact_for("linux", "aarch64").is_err());
}

#[test]
fn requirements_install_uses_managed_python_and_copy_materialization() {
    assert_eq!(
        install_command(
            "requirements.txt",
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
            nrz_source_bundle::PythonMinor::default()
        )
        .unwrap()
        .arguments,
        [
            "pip",
            "install",
            "--python",
            "3.14.8",
            "--managed-python",
            "--link-mode",
            "copy",
            "--target",
            ".onreza/python/3.14/site-packages",
            "--python-platform",
            "x86_64-manylinux_2_39",
            "--only-binary",
            ":all:",
            "--requirements",
            "requirements.txt",
        ]
        .map(OsString::from)
    );
    assert!(
        install_command(
            "requirements.txt",
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
            nrz_source_bundle::PythonMinor::default()
        )
        .unwrap()
        .display
        .contains("uv 0.12.23 / CPython 3.14")
    );
}

#[test]
fn project_manifest_install_resolves_only_runtime_compatible_wheels() {
    let arguments = install_command(
        "pyproject.toml",
        PythonInstallMode::ManagedLocal,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap()
    .arguments;
    assert!(arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--python-platform"),
            OsString::from("x86_64-manylinux_2_39"),
        ]
    }));
    assert!(arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--requirements"),
            OsString::from("pyproject.toml"),
        ]
    }));
    assert_ne!(arguments.last(), Some(&OsString::from(".")));
}

#[test]
fn platform_runner_uses_the_pinned_rootfs_python() {
    let command = install_command(
        "requirements.txt",
        PythonInstallMode::PinnedPlatform,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap();

    assert!(command.program.as_os_str().is_empty());
    assert_eq!(
        command.arguments,
        [
            "pip",
            "install",
            "--python",
            "/usr/local/bin/python3.14",
            "--no-managed-python",
            "--no-python-downloads",
            "--link-mode",
            "copy",
            "--target",
            ".onreza/python/3.14/site-packages",
            "--requirements",
            "requirements.txt",
        ]
        .map(OsString::from)
    );
    assert!(command.display.contains("uv 0.12.23 / CPython 3.14"));
    assert!(!command.arguments.contains(&OsString::from("--only-binary")));
}

#[test]
fn every_local_install_targets_linux_wheels_only() {
    let command = install_command(
        "pyproject.toml",
        PythonInstallMode::ManagedLocal,
        "macos",
        "aarch64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap();

    assert!(command.arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--python-platform"),
            OsString::from("x86_64-manylinux_2_39"),
        ]
    }));
    assert!(
        command
            .arguments
            .windows(2)
            .any(|pair| { pair == [OsString::from("--only-binary"), OsString::from(":all:"),] })
    );
    assert!(command.arguments.windows(2).any(|pair| {
        pair == [
            OsString::from("--requirements"),
            OsString::from("pyproject.toml"),
        ]
    }));
    assert_ne!(command.arguments.last(), Some(&OsString::from(".")));
}

#[test]
fn every_local_setup_py_fails_before_host_native_build() {
    let error = install_command(
        "setup.py",
        PythonInstallMode::ManagedLocal,
        "windows",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("cannot be qualified"));
    assert!(error.to_string().contains("Cloud Builder"));

    let linux_error = install_command(
        "setup.py",
        PythonInstallMode::ManagedLocal,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap_err();
    assert!(linux_error.to_string().contains("Cloud Builder"));
}

#[test]
fn uv_locked_plan_excludes_dev_and_installs_the_project_separately() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("pyproject.toml"), "[project]\nname='demo'\nversion='1.0'\nrequires-python='>=3.14,<3.15'\n[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n").unwrap();
    std::fs::write(project.path().join("uv.lock"), "").unwrap();
    let commands = install_commands(
        project.path(),
        PythonInstallMode::ManagedLocal,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap();
    assert_eq!(commands.len(), 6);
    let export = &commands[1].arguments;
    for flag in [
        "--locked",
        "--no-dev",
        "--no-default-groups",
        "--no-editable",
        "--no-emit-project",
    ] {
        assert!(export.contains(&OsString::from(flag)));
    }
    assert!(!export.contains(&OsString::from("--frozen")));
    assert!(commands[2].arguments.contains(&OsString::from("--no-deps")));
    assert!(commands[3].arguments.contains(&OsString::from("build")));
    assert!(commands[5].arguments.contains(&OsString::from("demo")));
}

#[test]
fn poetry_platform_plan_uses_frozen_tools_and_validates_both_python_constraints() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("pyproject.toml"), "[project]\nname='demo'\nversion='1.0'\nrequires-python='>=3.10'\n[tool.poetry]\npackage-mode=false\n[tool.poetry.dependencies]\npython='>=3.10,<3.14'\n").unwrap();
    std::fs::write(project.path().join("poetry.lock"), "").unwrap();
    let commands = install_commands(
        project.path(),
        PythonInstallMode::PinnedPlatform,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap();
    assert_eq!(commands.len(), 6);
    let commands = &commands[1..];
    for command in &commands[..2] {
        assert_eq!(
            command.program,
            std::path::PathBuf::from("/opt/onreza/poetry/bin/python")
        );
    }
    assert_eq!(
        commands[2].program,
        std::path::PathBuf::from("/opt/onreza/poetry/bin/poetry")
    );
    assert_eq!(
        commands[2].arguments,
        ["check", "--lock"].map(OsString::from)
    );
    assert!(
        commands[3]
            .arguments
            .windows(2)
            .any(|pair| pair == [OsString::from("--only"), OsString::from("main")])
    );
    assert!(commands[4].arguments.contains(&OsString::from("--no-deps")));
    assert!(
        commands
            .iter()
            .all(|command| !command.arguments.contains(&OsString::from("tool")))
    );
}

#[cfg(unix)]
#[test]
fn python_generated_directory_rejects_an_external_symlink_before_creating_children() {
    let project = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("requirements.txt"), "").unwrap();
    std::os::unix::fs::symlink(external.path(), project.path().join(".onreza")).unwrap();
    assert!(
        install_commands(
            project.path(),
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
            nrz_source_bundle::PythonMinor::default()
        )
        .unwrap_err()
        .to_string()
        .contains("escapes")
    );
    assert!(!external.path().join("python").exists());
}

#[tokio::test]
#[ignore = "downloads pinned Python packaging tools and exercises real uv lock/build/install"]
async fn python_uv_real_lock_src_package_and_stale_lock_qualification() {
    let mode = qualification_mode();
    let uv = super::python_toolchain::resolve_for(mode).await.unwrap();
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(project.path().join("src/demo")).unwrap();
        std::fs::write(
            project.path().join("src/demo/__init__.py"),
            "VALUE='installed-project'\n",
        )
        .unwrap();
        let manifest = "[project]\nname='nrz-python-demo'\nversion='1.0'\nrequires-python='>=3.14,<3.15'\ndependencies=['packaging==26.3']\n[dependency-groups]\ndev=['six==1.17.0']\n[build-system]\nrequires=['setuptools>=68']\nbuild-backend='setuptools.build_meta'\n";
        let manifest = manifest.replace(
            ">=3.14,<3.15",
            &format!(
                ">={},<3.{}",
                minor.version(),
                minor
                    .version()
                    .rsplit('.')
                    .next()
                    .unwrap()
                    .parse::<u8>()
                    .unwrap()
                    + 1
            ),
        );
        std::fs::write(project.path().join("pyproject.toml"), &manifest).unwrap();
        let mut lock_arguments = vec![OsString::from("lock")];
        lock_arguments.extend(super::python_toolchain::python_arguments(mode, minor));
        run(&uv, &lock_arguments, project.path());
        let lock = std::fs::read(project.path().join("uv.lock")).unwrap();
        std::fs::write(
            project.path().join("ambient-override.txt"),
            "packaging==26.2\n",
        )
        .unwrap();
        run_plan_with_environment(
            &uv,
            project.path(),
            &[
                (
                    "UV_OVERRIDE".into(),
                    project
                        .path()
                        .join("ambient-override.txt")
                        .to_string_lossy()
                        .into_owned(),
                ),
                ("UV_PYTHON_VERSION".into(), "3.13".into()),
                ("UV_NO_DEPS".into(), "1".into()),
            ],
            minor,
        );
        let python = qualification_interpreter(&uv, minor);
        let output = std::process::Command::new(&python).args(["-S", "-c", "import demo, packaging, importlib.util; print(demo.VALUE); print(packaging.__version__); assert importlib.util.find_spec('six') is None"]).env("PYTHONPATH", project.path().join(minor.site_packages_root())).current_dir(project.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "installed-project\n26.3\n"
        );
        assert_eq!(std::fs::read(project.path().join("uv.lock")).unwrap(), lock);
        qualification_artifact(project.path(), minor, "uv-package", "", &[], None).await;
        std::fs::write(
            project.path().join("pyproject.toml"),
            manifest.replace("packaging==26.3", "packaging==26.2"),
        )
        .unwrap();
        let commands = install_commands(project.path(), mode, "linux", "x86_64", minor).unwrap();
        let export = commands
            .iter()
            .find(|command| command.display.contains("validate and export uv.lock"))
            .unwrap();
        let output = export
            .process(&uv, &[])
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "stale uv.lock must fail before install: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("needs to be updated")
                && String::from_utf8_lossy(&output.stderr).contains("--locked"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read(project.path().join("uv.lock")).unwrap(), lock);
        std::fs::write(
            project.path().join("pyproject.toml"),
            manifest.replace(
                &format!(
                    ">={},<3.{}",
                    minor.version(),
                    minor
                        .version()
                        .rsplit('.')
                        .next()
                        .unwrap()
                        .parse::<u8>()
                        .unwrap()
                        + 1
                ),
                "<3.0",
            ),
        )
        .unwrap();
        let commands = install_commands(project.path(), mode, "linux", "x86_64", minor).unwrap();
        let validation = commands
            .iter()
            .find(|command| command.display.ends_with("validate requires-python"))
            .unwrap();
        let output = validation
            .process(&uv, &[])
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("requires-python excludes selected CPython"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
#[ignore = "downloads pinned Poetry/export tools and exercises real lock/build/install"]
async fn python_poetry_real_old_and_pep621_project_and_stale_lock_qualification() {
    let mode = qualification_mode();
    let uv = super::python_toolchain::resolve_for(mode).await.unwrap();
    for minor in nrz_source_bundle::PythonMinor::ALL {
        for metadata in [
            "[tool.poetry]\nname='demo'\nversion='1.0'\ndescription='qualification'\nauthors=['Test <test@example.com>']\n[tool.poetry.dependencies]\npython='>=3.14,<3.15'\npackaging='26.3'\n",
            "[project]\nname='demo'\nversion='1.0'\nrequires-python='>=3.14,<3.15'\ndependencies=['packaging==26.3']\n",
        ] {
            let metadata = metadata.replace(
                ">=3.14,<3.15",
                &format!(
                    ">={},<3.{}",
                    minor.version(),
                    minor
                        .version()
                        .rsplit('.')
                        .next()
                        .unwrap()
                        .parse::<u8>()
                        .unwrap()
                        + 1
                ),
            );
            let project = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(project.path().join("src/demo")).unwrap();
            std::fs::write(
                project.path().join("src/demo/__init__.py"),
                "VALUE='poetry-installed-project'\n",
            )
            .unwrap();
            let manifest = format!(
                "{metadata}\n[tool.poetry.group.dev.dependencies]\nsix='1.17.0'\n[build-system]\nrequires=['poetry-core==2.5.0']\nbuild-backend='poetry.core.masonry.api'\n"
            );
            std::fs::write(project.path().join("pyproject.toml"), &manifest).unwrap();
            // Generate only this authored fixture's initial lock. The deployment plan
            // below must check/export it and cannot resolve or rewrite it.
            let mut lock_arguments = ["tool", "run", "--isolated", "--no-env-file"]
                .map(OsString::from)
                .to_vec();
            lock_arguments.extend(super::python_toolchain::python_arguments(mode, minor));
            lock_arguments.extend(
                [
                    "--from",
                    "poetry==2.5.1",
                    "--with",
                    "poetry-plugin-export==1.10.1",
                    "poetry",
                    "lock",
                ]
                .map(OsString::from),
            );
            run(&uv, &lock_arguments, project.path());
            let lock = std::fs::read(project.path().join("poetry.lock")).unwrap();
            let ambient = tempfile::tempdir().unwrap();
            std::fs::write(
                ambient.path().join("poetry.py"),
                "raise RuntimeError('ambient Poetry import override')\n",
            )
            .unwrap();
            run_plan_with_environment(
                &uv,
                project.path(),
                &[(
                    "PYTHONPATH".into(),
                    ambient.path().to_string_lossy().into_owned(),
                )],
                minor,
            );
            let python = qualification_interpreter(&uv, minor);
            let output = std::process::Command::new(&python).args(["-S", "-c", "import demo, packaging, importlib.util; print(demo.VALUE); print(packaging.__version__); assert importlib.util.find_spec('six') is None"]).env("PYTHONPATH", project.path().join(minor.site_packages_root())).current_dir(project.path()).output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                "poetry-installed-project\n26.3\n"
            );
            assert_eq!(
                std::fs::read(project.path().join("poetry.lock")).unwrap(),
                lock
            );
            qualification_artifact(
                project.path(),
                minor,
                if metadata.starts_with("[project]") {
                    "poetry-pep621-package"
                } else {
                    "poetry-legacy-package"
                },
                "",
                &[],
                None,
            )
            .await;
            std::fs::write(
                project.path().join("pyproject.toml"),
                manifest.replace("26.3", "26.2"),
            )
            .unwrap();
            let commands =
                install_commands(project.path(), mode, "linux", "x86_64", minor).unwrap();
            let check = commands
                .iter()
                .find(|command| {
                    command
                        .arguments
                        .windows(2)
                        .any(|pair| pair == [OsString::from("check"), OsString::from("--lock")])
                })
                .unwrap();
            let result = check
                .process(&uv, &[])
                .current_dir(project.path())
                .output()
                .unwrap();
            assert!(
                !result.status.success(),
                "stale poetry.lock must fail before install: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains("pyproject.toml changed significantly"),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(
                std::fs::read(project.path().join("poetry.lock")).unwrap(),
                lock
            );
        }
    }
}

#[tokio::test]
#[ignore = "builds a real setuptools wheel with native package data to qualify local cross-target rejection"]
async fn python_local_rejects_native_payload_in_mislabeled_none_any_wheel() {
    let uv = super::python_toolchain::resolve().await.unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("src/demo")).unwrap();
    std::fs::write(project.path().join("src/demo/__init__.py"), "").unwrap();
    std::fs::write(project.path().join("pyproject.toml"),"[project]\nname='demo'\nversion='1.0'\n[build-system]\nrequires=['setuptools>=68']\nbuild-backend='setuptools.build_meta'\n[tool.setuptools.package-data]\ndemo=['*.so']\n").unwrap();
    let interpreter = std::process::Command::new(&uv)
        .args(["python", "find", "3.14", "--managed-python"])
        .output()
        .unwrap();
    assert!(interpreter.status.success());
    let interpreter = String::from_utf8(interpreter.stdout).unwrap();
    let native = std::process::Command::new(interpreter.trim())
        .args(["-I", "-c", "import sys; print(sys.executable)"])
        .output()
        .unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    let native = String::from_utf8(native.stdout).unwrap();
    std::fs::copy(native.trim(), project.path().join("src/demo/native.so")).unwrap();
    let mut rejection = None;
    for command in install_commands(
        project.path(),
        PythonInstallMode::ManagedLocal,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap()
    {
        let result = command
            .process(&uv, &[])
            .current_dir(project.path())
            .output()
            .unwrap();
        if !result.status.success() {
            rejection = Some(String::from_utf8_lossy(&result.stderr).into_owned());
            break;
        }
    }
    let rejection = rejection.expect("a pure tag must not qualify native package data");
    assert!(
        rejection.contains("contains native payload demo/native.so"),
        "{rejection}"
    );
    assert!(rejection.contains("Cloud Builder"));
    assert!(
        project
            .path()
            .join(".onreza/python/build/wheels/demo-1.0-py3-none-any.whl")
            .exists()
    );
    assert!(
        !project
            .path()
            .join(nrz_source_bundle::PythonMinor::default().site_packages_root())
            .join("demo")
            .exists()
    );
}

fn run_plan_with_environment(
    uv: &std::path::Path,
    project: &std::path::Path,
    environment: &[(String, String)],
    minor: nrz_source_bundle::PythonMinor,
) {
    for command in
        install_commands(project, qualification_mode(), "linux", "x86_64", minor).unwrap()
    {
        let result = command
            .process(uv, environment)
            .current_dir(project)
            .env("POETRY_VIRTUALENVS_CREATE", "false")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}: {}",
            command.display,
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

fn run(program: &std::path::Path, arguments: &[OsString], project: &std::path::Path) {
    let result = std::process::Command::new(program)
        .args(arguments)
        .current_dir(project)
        .env("POETRY_VIRTUALENVS_CREATE", "false")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{program:?} {arguments:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[cfg(unix)]
#[test]
fn python_export_and_wheel_outputs_reject_external_symlinks() {
    for output in ["requirements.txt", "wheels"] {
        let project = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("pyproject.toml"), "[project]\nname='demo'\nversion='1.0'\n[build-system]\nrequires=['setuptools']\nbuild-backend='setuptools.build_meta'\n").unwrap();
        std::fs::write(project.path().join("uv.lock"), "").unwrap();
        let generated = project.path().join(".onreza/python/build");
        std::fs::create_dir_all(&generated).unwrap();
        let target = external.path().join(output);
        if output == "wheels" {
            std::fs::create_dir(&target).unwrap();
        } else {
            std::fs::write(&target, "preserved").unwrap();
        }
        std::os::unix::fs::symlink(&target, generated.join(output)).unwrap();
        assert!(
            install_commands(
                project.path(),
                PythonInstallMode::ManagedLocal,
                "linux",
                "x86_64",
                nrz_source_bundle::PythonMinor::default()
            )
            .is_err(),
            "{output} must be confined before any tool writes"
        );
    }
}

#[tokio::test]
#[ignore = "uses pinned uv to verify ambient project selection cannot replace the authored lock"]
async fn python_locked_install_ignores_ambient_project_selection() {
    let uv = super::python_toolchain::resolve().await.unwrap();
    let project = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    for (directory, name, dependency) in [
        (project.path(), "demo", "packaging==26.3"),
        (other.path(), "other", "six==1.17.0"),
    ] {
        std::fs::write(directory.join("pyproject.toml"), format!("[project]\nname='{name}'\nversion='1.0'\nrequires-python='>=3.14'\ndependencies=['{dependency}']\n")).unwrap();
        run(
            &uv,
            &["lock", "--python", "3.14", "--managed-python"].map(OsString::from),
            directory,
        );
    }
    let environment = [
        (
            "UV_PROJECT".into(),
            other.path().to_string_lossy().into_owned(),
        ),
        (
            "UV_WORKING_DIR".into(),
            other.path().to_string_lossy().into_owned(),
        ),
    ];
    run_plan_with_environment(
        &uv,
        project.path(),
        &environment,
        nrz_source_bundle::PythonMinor::default(),
    );
    let installed = project
        .path()
        .join(nrz_source_bundle::PythonMinor::default().site_packages_root());
    assert!(installed.join("packaging").is_dir());
    assert!(!installed.join("six.py").exists());
    assert!(!other.path().join(".onreza").exists());
}

#[test]
fn python_requirements_and_setup_package_use_one_authored_dependency_graph() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("requirements.txt"), "").unwrap();
    std::fs::write(
        project.path().join("setup.py"),
        "from setuptools import setup; setup(name='demo')",
    )
    .unwrap();
    assert!(
        install_commands(
            project.path(),
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
            nrz_source_bundle::PythonMinor::default()
        )
        .is_err()
    );
    let commands = install_commands(
        project.path(),
        PythonInstallMode::PinnedPlatform,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap();
    assert_eq!(commands.len(), 3);
    let commands = &commands[1..];
    assert!(
        commands[0]
            .arguments
            .contains(&OsString::from("requirements.txt"))
    );
    assert!(commands[1].arguments.contains(&OsString::from("--no-deps")));
    assert!(commands[1].arguments.contains(&OsString::from(".")));
}

#[tokio::test]
#[ignore = "executes the generated validator with an independently pinned tools interpreter"]
async fn python_constraints_validate_the_application_patch_instead_of_the_tools_interpreter() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\nversion='1.0'\nrequires-python='>=3.12,<3.13'\n",
    )
    .unwrap();
    let commands = install_commands(
        project.path(),
        PythonInstallMode::PinnedPlatform,
        "linux",
        "x86_64",
        nrz_source_bundle::PythonMinor::Python312,
    )
    .unwrap();
    // Execute the generated validator on a different interpreter. The validator's
    // dependencies come from the tools environment; target identity remains argv.
    let validation = &commands[1];
    let uv = super::python_toolchain::resolve_for(qualification_mode())
        .await
        .unwrap();
    let execute = |arguments: &[OsString]| {
        if qualification_mode() == PythonInstallMode::PinnedPlatform {
            return std::process::Command::new(&validation.program)
                .args(arguments)
                .output()
                .unwrap();
        }
        let mut tool = ["tool", "run", "--isolated", "--from", "packaging==26.3"]
            .map(OsString::from)
            .to_vec();
        tool.extend(super::python_toolchain::python_arguments(
            PythonInstallMode::ManagedLocal,
            nrz_source_bundle::PythonMinor::Python314,
        ));
        tool.push(OsString::from("python"));
        std::process::Command::new(&uv)
            .args(tool)
            .args(arguments)
            .output()
            .unwrap()
    };
    let output = execute(&validation.arguments);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut wrong_target = validation.arguments.clone();
    *wrong_target.last_mut().unwrap() =
        OsString::from(nrz_source_bundle::PythonMinor::Python314.exact_version());
    let output = execute(&wrong_target);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("requires-python excludes selected CPython")
    );
}

pub(super) fn qualification_mode() -> PythonInstallMode {
    if std::env::var("ONREZA_BUILDER_QUALIFICATION").as_deref() == Ok("1") {
        PythonInstallMode::PinnedPlatform
    } else {
        PythonInstallMode::ManagedLocal
    }
}

pub(super) fn qualification_interpreter(
    uv: &std::path::Path,
    minor: nrz_source_bundle::PythonMinor,
) -> std::path::PathBuf {
    if qualification_mode() == PythonInstallMode::PinnedPlatform {
        return minor.platform_interpreter().into();
    }
    let output = std::process::Command::new(uv)
        .args(["python", "find", minor.exact_version(), "--managed-python"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

pub(super) async fn qualification_artifact(
    project: &std::path::Path,
    minor: nrz_source_bundle::PythonMinor,
    name: &str,
    entry: &str,
    args: &[String],
    expected_body: Option<serde_json::Value>,
) {
    if qualification_mode() != PythonInstallMode::PinnedPlatform {
        return;
    }
    let destination = std::path::Path::new("/qualification/output")
        .join(minor.version())
        .join(name);
    export_qualification_artifact(
        project,
        minor,
        name,
        entry,
        args,
        expected_body,
        &destination,
    )
    .await;
}

async fn export_qualification_artifact(
    project: &std::path::Path,
    minor: nrz_source_bundle::PythonMinor,
    name: &str,
    entry: &str,
    args: &[String],
    expected_body: Option<serde_json::Value>,
    destination: &std::path::Path,
) {
    // Runtime fixtures must have artifact permissions, including the generated
    // bootstrap's canonical 0644 mode rather than its private build-time 0600.
    let source_entry = if entry.is_empty() {
        "src/demo/__init__.py"
    } else {
        entry
    };
    let mut manifest = crate::build::manifest::generate_compute_manifest(source_entry);
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Python,
        python_version: Some(minor),
        entry: Some(source_entry.into()),
        args: args.to_vec(),
    };
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        Some(&declaration),
        Some(minor.target()),
        "other",
    )
    .unwrap();
    let plan = crate::artifact::source_bundle_v1::build_source_bundle_plan_with_scan(
        project,
        &manifest,
        &crate::deploy::scan_dir(project).unwrap(),
        &crate::artifact::RuntimeArtifactScan::PythonRuntimeRoot(minor),
        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
        None,
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
    let compressed = std::fs::read(plan.source_path()).unwrap();
    let verified = nrz_source_bundle::verify_source_bundle_bytes(input, compressed.clone().into())
        .await
        .unwrap();
    let logical: nrz_source_bundle::SourceLogicalManifest =
        serde_json::from_value(verified.logical_manifest).unwrap();
    let launch = nrz_runtime_artifact::source_layer_launch_for_target(
        logical.layers[0].runtime_config.as_ref(),
        Some(minor.target()),
    )
    .unwrap();
    assert_eq!(
        nrz_runtime_artifact::python_minor_for_profile(launch.profile),
        Some(minor)
    );
    std::fs::create_dir_all(destination).unwrap();
    let decoded = zstd::stream::read::Decoder::new(std::io::Cursor::new(compressed)).unwrap();
    tar::Archive::new(decoded).unpack(destination).unwrap();
    let smoke = entry.is_empty().then(|| serde_json::json!({"module":"demo", "expectedValue":if name.starts_with("poetry") {"poetry-installed-project"} else {"installed-project"}, "dependency":"packaging", "dependencyVersion":"26.3", "absent":"six"}));
    std::fs::write(destination.join("launch.json"), serde_json::to_vec(&serde_json::json!({"family":"python", "pythonMinor":minor.version(), "framework":name, "entry":entry, "args":launch.args, "cwd":launch.cwd, "expectedBody":expected_body,"smoke":smoke})).unwrap()).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn python_qualification_export_uses_canonical_artifact_permissions() {
    use std::os::unix::fs::PermissionsExt as _;
    let project = tempfile::tempdir().unwrap();
    std::fs::set_permissions(project.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let entry = ".onreza/python/launch.py";
    std::fs::create_dir_all(project.path().join(".onreza/python")).unwrap();
    std::fs::write(project.path().join(entry), "print('qualification')\n").unwrap();
    std::fs::set_permissions(
        project.path().join(entry),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let output = tempfile::tempdir().unwrap();
    let destination = output.path().join("application");
    export_qualification_artifact(
        project.path(),
        nrz_source_bundle::PythonMinor::Python312,
        "permission-regression",
        entry,
        &[],
        None,
        &destination,
    )
    .await;
    assert_eq!(
        std::fs::metadata(&destination)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        std::fs::metadata(destination.join(".onreza/python"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        std::fs::metadata(destination.join(entry))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
}
