//! Translate Python application intent into an immutable, file-based launch.

use std::path::Path;

use anyhow::bail;

use crate::detect::python_launch::{PYTHON_BOOTSTRAP_ENTRY, PythonLaunch};

/// Fixed bootstrap bytes, while mode/target/argv remain frozen launch intent.
pub(crate) fn materialize_python_entry(
    project_dir: &Path,
    launch: &PythonLaunch,
    minor: nrz_source_bundle::PythonMinor,
) -> anyhow::Result<String> {
    if launch.entry != PYTHON_BOOTSTRAP_ENTRY {
        return Ok(launch.entry.clone());
    }
    if let Some(mode) = launch.args.first() {
        let server = match mode.as_str() {
            "ASGI" => Some("uvicorn"),
            "WSGI" => Some("gunicorn"),
            _ => None,
        };
        if let Some(server) = server
            && !project_dir
                .join(minor.site_packages_root())
                .join(server)
                .join("__main__.py")
                .is_file()
        {
            bail!(
                "Python {mode} recipe requires {server} in installed production dependencies; add it with uv add {server}, poetry add {server}, or requirements.txt and reinstall (development-only dependencies are excluded)"
            );
        }
    }
    let directory = project_dir.join(".onreza/python");
    super::python_toolchain::ensure_python_directory(project_dir, &directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    // Preserve build-minor custody even when an authorized successor runtime
    // later proves compatible with this artifact.
    let bootstrap = PYTHON_BOOTSTRAP.replace("@SITE_PACKAGES_ROOT@", minor.site_packages_root());
    std::io::Write::write_all(&mut temporary, bootstrap.as_bytes())?;
    temporary
        .persist(project_dir.join(PYTHON_BOOTSTRAP_ENTRY))
        .map_err(|error| error.error)?;
    Ok(launch.entry.clone())
}

pub(crate) const PYTHON_BOOTSTRAP: &str = r#"import os
import importlib
from pathlib import Path
import runpy
import sys

root = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(root / "@SITE_PACKAGES_ROOT@"))
sys.path.insert(1, str(root))
if (root / "src").is_dir():
    sys.path.insert(2, str(root / "src"))
mode, target, *arguments = sys.argv[1:]
if mode == "MODULE":
    sys.argv = [target, *arguments]
    runpy.run_module(target, run_name="__main__", alter_sys=True)
elif mode == "CALLABLE":
    module, attribute = target.removesuffix("()").split(":", 1)
    function = importlib.import_module(module)
    for name in attribute.split("."):
        function = getattr(function, name)
    sys.argv = [target, *arguments]
    sys.exit(function())
elif mode in ("ASGI", "WSGI"):
    port = int(os.environ["PORT"])
    if not 1 <= port <= 65535:
        raise ValueError("PORT is outside the valid TCP port range")
    if mode == "ASGI":
        factory = target.endswith("()")
        if factory:
            target = target[:-2]
        sys.argv = ["uvicorn", target, *arguments, "--host", "0.0.0.0", "--port", str(port)]
        if factory:
            sys.argv.append("--factory")
        runpy.run_module("uvicorn", run_name="__main__", alter_sys=True)
    else:
        sys.argv = ["gunicorn", target, *arguments, "--bind", "0.0.0.0:" + str(port)]
        runpy.run_module("gunicorn", run_name="__main__", alter_sys=True)
else:
    raise ValueError("unknown Python launch mode")
"#;
