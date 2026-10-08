use super::*;

const EDITABLE_BACKEND: &str = r#"
from pathlib import Path
import zipfile
Path(__file__).with_name('BACKEND_IMPORTED').write_text('host-backend-import')

def get_requires_for_build_editable(config_settings=None): return []
def get_requires_for_build_wheel(config_settings=None): return []
def build_editable(wheel_directory, config_settings=None, metadata_directory=None):
    Path(__file__).with_name('BACKEND_EXECUTED').write_text('host-source-build')
    wheel = 'editable_demo-1.0-py3-none-any.whl'
    with zipfile.ZipFile(Path(wheel_directory) / wheel, 'w') as z:
        z.writestr('editable_demo.pth', str(Path(__file__).parent.absolute()) + '\n')
        z.writestr('editable_demo/native.data', b'\xcf\xfa\xed\xfeHOST_PAYLOAD')
        z.writestr('editable_demo-1.0.dist-info/METADATA', 'Metadata-Version: 2.1\nName: editable-demo\nVersion: 1.0\n')
        z.writestr('editable_demo-1.0.dist-info/WHEEL', 'Wheel-Version: 1.0\nRoot-Is-Purelib: true\nTag: py3-none-any\n')
        z.writestr('editable_demo-1.0.dist-info/RECORD', '')
    return wheel
def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    return build_editable(wheel_directory, config_settings, metadata_directory)
"#;

#[test]
fn local_editable_policy_preserves_the_platform_source_build_recipe() {
    for minor in [
        PythonMinor::Python312,
        PythonMinor::Python313,
        PythonMinor::Python314,
    ] {
        for manifest in ["requirements.txt", "pyproject.toml"] {
            let local = install_command(
                manifest,
                PythonInstallMode::ManagedLocal,
                "linux",
                "x86_64",
                minor,
            )
            .unwrap();
            assert!(local.arguments.contains(&OsString::from("--no-editable")));
            let platform = install_command(
                manifest,
                PythonInstallMode::PinnedPlatform,
                "linux",
                "x86_64",
                minor,
            )
            .unwrap();
            assert!(
                !platform
                    .arguments
                    .contains(&OsString::from("--no-editable"))
            );
            assert!(
                !platform
                    .arguments
                    .contains(&OsString::from("--only-binary"))
            );
        }
    }
}

#[tokio::test]
#[ignore = "runs pinned uv and managed CPython against real editable backends and local wheels"]
async fn python_local_editables_cannot_bypass_target_wheel_policy() {
    let uv = resolve().await.unwrap();
    let minor = PythonMinor::default();
    let python = std::process::Command::new(&uv)
        .args([
            "python",
            "find",
            minor.exact_version(),
            "--managed-python",
            "--offline",
            "--no-python-downloads",
        ])
        .output()
        .unwrap();
    assert!(
        python.status.success(),
        "{}",
        String::from_utf8_lossy(&python.stderr)
    );
    let python = String::from_utf8(python.stdout).unwrap();
    let mut unguarded = Vec::new();
    for (name, requirements) in [("direct", "-e ./dep\n"), ("nested", "-r nested.txt\n")] {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join("dep")).unwrap();
        std::fs::write(project.path().join("dep/pyproject.toml"), "[build-system]\nrequires=[]\nbuild-backend='backend'\nbackend-path=['.']\n[project]\nname='editable-demo'\nversion='1.0'\n").unwrap();
        std::fs::write(project.path().join("dep/backend.py"), EDITABLE_BACKEND).unwrap();
        std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
        std::fs::write(project.path().join("nested.txt"), "-e ./dep\n").unwrap();
        let commands = install_commands(
            project.path(),
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
            minor,
        )
        .unwrap();
        assert_eq!(commands.len(), 1);
        let result = commands[0]
            .process(&uv, &[("UV_OFFLINE".into(), "1".into())])
            .arg("--no-config")
            .arg("--no-python-downloads")
            .current_dir(project.path())
            .output()
            .unwrap();
        let stage = project.path().join(minor.site_packages_root());
        let backend_executed = project.path().join("dep/BACKEND_EXECUTED").exists()
            || project.path().join("dep/BACKEND_IMPORTED").exists();
        eprintln!(
            "editable {name}: exit={:?} backend={backend_executed} pth={} native={} stderr={}",
            result.status.code(),
            stage.join("editable_demo.pth").exists(),
            stage.join("editable_demo/native.data").exists(),
            String::from_utf8_lossy(&result.stderr)
        );
        if result.status.success()
            || backend_executed
            || stage.join("editable_demo.pth").exists()
            || stage.join("editable_demo/native.data").exists()
        {
            unguarded.push(name);
        } else {
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains("Building source distributions is disabled")
            );
        }
    }
    let project = tempfile::tempdir().unwrap();
    let wheel = project.path().join("editable_demo-1.0-py3-none-any.whl");
    let created = std::process::Command::new(python.trim()).args(["-I", "-c", r#"
import sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w') as z:
    z.writestr('editable_demo/__init__.py', 'VALUE = 42\n')
    z.writestr('editable_demo-1.0.dist-info/METADATA', 'Metadata-Version: 2.1\nName: editable-demo\nVersion: 1.0\n')
    z.writestr('editable_demo-1.0.dist-info/WHEEL', 'Wheel-Version: 1.0\nRoot-Is-Purelib: true\nTag: py3-none-any\n')
    z.writestr('editable_demo-1.0.dist-info/RECORD', '')
"#]).arg(&wheel).output().unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    for requirement in [
        wheel.display().to_string(),
        url::Url::from_file_path(&wheel).unwrap().to_string(),
    ] {
        std::fs::write(
            project.path().join("requirements.txt"),
            format!("{requirement}\n"),
        )
        .unwrap();
        let command = install_commands(
            project.path(),
            PythonInstallMode::ManagedLocal,
            "linux",
            "x86_64",
            minor,
        )
        .unwrap()
        .remove(0);
        let result = command
            .process(&uv, &[("UV_OFFLINE".into(), "1".into())])
            .arg("--no-config")
            .arg("--no-python-downloads")
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{requirement}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            std::fs::read(
                project
                    .path()
                    .join(minor.site_packages_root())
                    .join("editable_demo/__init__.py")
            )
            .unwrap(),
            b"VALUE = 42\n"
        );
        eprintln!("local wheel {requirement}: installed pure package");
        std::fs::remove_dir_all(project.path().join(minor.site_packages_root())).unwrap();
    }
    assert!(
        unguarded.is_empty(),
        "editable requirements executed the host backend: {unguarded:?}"
    );
}
