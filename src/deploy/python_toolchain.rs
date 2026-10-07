use std::ffi::{OsStr, OsString};
#[cfg(windows)]
use std::io::Cursor;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use nrz_source_bundle::{PythonMinor, python_toolchain_versions};

const QUALIFIED_UV_VERSION: &str = "0.12.23";
const UV_RELEASE_ORIGIN: &str = "https://github.com";
const MAX_UV_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_UV_BINARY_BYTES: u64 = 96 * 1024 * 1024;
const PLATFORM_PYTHON_TARGET: &str = "x86_64-manylinux_2_39";
const NATIVE_PAYLOAD_EXTENSIONS: &[&str] = &[".so", ".pyd", ".dll", ".dylib", ".a", ".o"];
const NATIVE_PAYLOAD_MAGICS: &[&[u8]] = &[
    b"\x7fELF",
    b"\xfe\xed\xfa\xce",
    b"\xce\xfa\xed\xfe",
    b"\xfe\xed\xfa\xcf",
    b"\xcf\xfa\xed\xfe",
    b"\xca\xfe\xba\xbe",
    b"\xbe\xba\xfe\xca",
    b"\xca\xfe\xba\xbf",
    b"\xbf\xba\xfe\xca",
    b"MZ",
    b"!<arch>\n",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PythonInstallMode {
    ManagedLocal,
    PinnedPlatform,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct PythonInstallCommand {
    pub(super) program: PathBuf,
    pub(super) arguments: Vec<OsString>,
    pub(super) display: String,
}

impl PythonInstallCommand {
    pub(super) fn process(
        &self,
        uv: &Path,
        environment: &[(String, String)],
    ) -> std::process::Command {
        let program = if self.program.as_os_str().is_empty() {
            uv
        } else {
            self.program.as_path()
        };
        let mut command = std::process::Command::new(program);
        command
            .args(&self.arguments)
            .envs(environment.iter().map(|(key, value)| (key, value)));
        // The authored manifest/lock and selected interpreter own this recipe.
        // Index credentials remain available; ambient overrides cannot replace
        // locked requirements, omit their closure, or disable authored hashes.
        for key in [
            "PYTHONPATH",
            "PYTHONHOME",
            "UV_PROJECT",
            "UV_WORKING_DIR",
            "UV_CONFIG_FILE",
            "UV_NO_CONFIG",
            "UV_FROZEN",
            "UV_OVERRIDE",
            "UV_EXCLUDE",
            "UV_NO_VERIFY_HASHES",
            "UV_NO_DEPS",
            "UV_PYTHON",
            "UV_PYTHON_VERSION",
            "UV_MANAGED_PYTHON",
            "UV_NO_MANAGED_PYTHON",
            "UV_PYTHON_PREFERENCE",
        ] {
            command.env_remove(key);
        }
        command
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ArchiveFormat {
    TarGz,
    Zip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct UvArtifact {
    pub(super) target: &'static str,
    archive_name: &'static str,
    pub(super) archive_sha256: &'static str,
    pub(super) binary_sha256: &'static str,
    binary_name: &'static str,
    pub(super) format: ArchiveFormat,
}

pub(super) async fn resolve() -> anyhow::Result<PathBuf> {
    let artifact = artifact_for(std::env::consts::OS, std::env::consts::ARCH)?;
    let cached_path = default_cache_root()?
        .join("nrz")
        .join("python-tools")
        .join("uv")
        .join(&python_toolchain_versions().uv)
        .join(artifact.target)
        .join(artifact.binary_name);

    if cached_binary_is_valid(&cached_path, artifact).await? {
        return Ok(cached_path);
    }

    let archive = download_archive(artifact).await?;
    let binary = extract_binary(&archive, artifact)?;
    if sha256_hex(&binary) != artifact.binary_sha256 {
        bail!("extracted uv binary does not match the CLI pin");
    }
    install_binary(&cached_path, &binary, artifact).await?;
    Ok(cached_path)
}

pub(super) async fn resolve_for(mode: PythonInstallMode) -> anyhow::Result<PathBuf> {
    match mode {
        PythonInstallMode::ManagedLocal => resolve().await,
        PythonInstallMode::PinnedPlatform => Ok(PathBuf::from("/usr/local/bin/uv")),
    }
}

/// Select the actual compiler and make the installed build tools visible to
/// authored commands. Serving dependencies keep their independent ABI witness.
pub(super) async fn build_environment(
    project_dir: &Path,
    mode: PythonInstallMode,
    minor: PythonMinor,
    environment: &[(String, String)],
) -> anyhow::Result<Vec<(String, String)>> {
    build_environment_for_host(
        project_dir,
        mode,
        minor,
        environment,
        (std::env::consts::OS, std::env::consts::ARCH),
        if mode == PythonInstallMode::ManagedLocal {
            host_glibc_version()
        } else {
            None
        },
    )
    .await
}

async fn build_environment_for_host(
    project_dir: &Path,
    mode: PythonInstallMode,
    minor: PythonMinor,
    environment: &[(String, String)],
    host: (&str, &str),
    glibc_version: Option<(u32, u32)>,
) -> anyhow::Result<Vec<(String, String)>> {
    validate_local_build_dependency_host_with_glibc(project_dir, mode, minor, host, glibc_version)?;
    let interpreter = match mode {
        PythonInstallMode::PinnedPlatform => PathBuf::from(minor.platform_interpreter()),
        PythonInstallMode::ManagedLocal => {
            let uv = resolve_for(mode).await?;
            for arguments in [
                vec![
                    "python",
                    "install",
                    "--no-bin",
                    "--no-registry",
                    "--no-config",
                    minor.exact_version(),
                ],
                vec![
                    "python",
                    "find",
                    "--no-project",
                    "--no-config",
                    "--managed-python",
                    minor.exact_version(),
                ],
            ] {
                let command = PythonInstallCommand {
                    program: uv.clone(),
                    arguments: arguments.into_iter().map(OsString::from).collect(),
                    display: "select managed Python build compiler".into(),
                };
                let mut process: tokio::process::Command = command.process(&uv, environment).into();
                let output = process
                    .current_dir(project_dir)
                    .kill_on_drop(true)
                    .output()
                    .await?;
                if !output.status.success() {
                    bail!(
                        "cannot select pinned Python build compiler: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                if command
                    .arguments
                    .get(1)
                    .is_some_and(|argument| argument == "find")
                {
                    let path = String::from_utf8(output.stdout)?.trim().to_string();
                    return build_environment_for_interpreter(
                        project_dir,
                        minor,
                        Path::new(&path),
                        environment,
                    )
                    .await;
                }
            }
            unreachable!("managed interpreter selection returns its path")
        }
    };
    build_environment_for_interpreter(project_dir, minor, &interpreter, environment).await
}

#[cfg(test)]
fn validate_local_build_dependency_host(
    project_dir: &Path,
    mode: PythonInstallMode,
    minor: PythonMinor,
    host: (&str, &str),
) -> anyhow::Result<()> {
    validate_local_build_dependency_host_with_glibc(
        project_dir,
        mode,
        minor,
        host,
        host_glibc_version(),
    )
}

fn validate_local_build_dependency_host_with_glibc(
    project_dir: &Path,
    mode: PythonInstallMode,
    minor: PythonMinor,
    host: (&str, &str),
    glibc_version: Option<(u32, u32)>,
) -> anyhow::Result<()> {
    let glibc_floor = python_target_glibc_floor();
    if mode == PythonInstallMode::PinnedPlatform
        || host == ("linux", "x86_64")
            && glibc_version.is_some_and(|version| version >= glibc_floor)
    {
        return Ok(());
    }
    let root = project_dir.join(minor.site_packages_root());
    let payload = find_native_python_payload(&root).map_err(|error| crate::output::coded_error(
        "PYTHON_PLATFORM_UNSUPPORTED",
        format!("Cannot inspect Linux-target Python dependency stage for host {}/{}: {error:#}. Use ONREZA Cloud Builder for qualified Linux builds.", host.0, host.1),
    ))?;
    if let Some(payload) = payload {
        return Err(crate::output::coded_error(
            "PYTHON_PLATFORM_UNSUPPORTED",
            format!(
                "Linux-target Python dependency stage contains native payload '{}'; host {}/{} with {} cannot use target {PLATFORM_PYTHON_TARGET} for local build commands. Native local builds require Linux x86_64 and verified glibc {}.{} or newer (getconf GNU_LIBC_VERSION). Use ONREZA Cloud Builder for qualified Linux builds, or deploy a prebuilt artifact with --skip-build.",
                payload.strip_prefix(project_dir)?.display(),
                host.0,
                host.1,
                glibc_version.map_or_else(
                    || "musl or unverified libc".to_string(),
                    |(major, minor)| format!("glibc {major}.{minor}")
                ),
                glibc_floor.0,
                glibc_floor.1,
            ),
        ));
    }
    Ok(())
}

fn python_target_glibc_floor() -> (u32, u32) {
    let version = PLATFORM_PYTHON_TARGET
        .rsplit_once("manylinux_")
        .expect("qualified Python target must identify manylinux ABI")
        .1;
    let (major, minor) = version
        .split_once('_')
        .expect("qualified manylinux ABI must identify glibc major and minor");
    (
        major
            .parse()
            .expect("qualified glibc major must be numeric"),
        minor
            .parse()
            .expect("qualified glibc minor must be numeric"),
    )
}

fn parse_host_glibc_version(output: &str) -> Option<(u32, u32)> {
    let version = output.trim().strip_prefix("glibc ")?;
    let (major, minor) = version.split_once('.')?;
    if !major.bytes().all(|byte| byte.is_ascii_digit())
        || !minor.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some((major.parse().ok()?, minor.parse().ok()?))
}

fn host_glibc_version() -> Option<(u32, u32)> {
    if std::env::consts::OS != "linux" {
        return None;
    }
    // The released CLI is musl-linked; its own libc is not host ABI evidence.
    // Probe the host utility without a shell or any target-stage Python imports.
    let output = std::process::Command::new("getconf")
        .arg("GNU_LIBC_VERSION")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_host_glibc_version(std::str::from_utf8(&output.stdout).ok()?)
}

fn find_native_python_payload(root: &Path) -> anyhow::Result<Option<PathBuf>> {
    fn visit(
        path: &Path,
        visited: &mut std::collections::HashSet<PathBuf>,
    ) -> anyhow::Result<Option<PathBuf>> {
        let metadata = std::fs::metadata(path)?;
        if metadata.is_dir() {
            if !visited.insert(path.canonicalize()?) {
                return Ok(None);
            }
            for entry in std::fs::read_dir(path)? {
                if let Some(payload) = visit(&entry?.path(), visited)? {
                    return Ok(Some(payload));
                }
            }
        } else if metadata.is_file() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            if NATIVE_PAYLOAD_EXTENSIONS
                .iter()
                .any(|extension| name.ends_with(extension))
            {
                return Ok(Some(path.to_owned()));
            }
            let mut header = Vec::with_capacity(8);
            std::fs::File::open(path)?
                .take(8)
                .read_to_end(&mut header)?;
            if NATIVE_PAYLOAD_MAGICS
                .iter()
                .any(|magic| header.starts_with(magic))
            {
                return Ok(Some(path.to_owned()));
            }
        }
        Ok(None)
    }
    match std::fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
        Ok(_) => visit(root, &mut std::collections::HashSet::new()),
    }
}

async fn build_environment_for_interpreter(
    project_dir: &Path,
    minor: PythonMinor,
    interpreter: &Path,
    environment: &[(String, String)],
) -> anyhow::Result<Vec<(String, String)>> {
    let interpreter = interpreter
        .canonicalize()
        .context("selected Python compiler is unavailable")?;
    let mut probe = tokio::process::Command::new(&interpreter);
    let output = probe.args(["-I", "-c", "import sys; print('.'.join(map(str, sys.version_info[:3])) if sys.implementation.name == 'cpython' else '')"])
        .kill_on_drop(true).output().await?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim() != minor.exact_version()
    {
        bail!(
            "Python build compiler does not match selected CPython {}",
            minor.exact_version()
        );
    }
    let dependency_root = project_dir.join(minor.site_packages_root());
    let startup = materialize_python_build_startup(project_dir, &dependency_root)?;
    let inherited_path = environment
        .iter()
        .rev()
        .find(|(name, _)| name == "PATH")
        .map(|(_, value)| OsString::from(value))
        .or_else(|| std::env::var_os("PATH"))
        .unwrap_or_default();
    let paths = [
        interpreter
            .parent()
            .context("Python compiler has no directory")?
            .to_owned(),
        dependency_root.join("bin"),
    ]
    .into_iter()
    .chain(std::env::split_paths(&inherited_path));
    Ok(vec![
        (
            "PATH".into(),
            std::env::join_paths(paths)?.to_string_lossy().into_owned(),
        ),
        (
            "PYTHONPATH".into(),
            std::env::join_paths([startup, dependency_root])?
                .to_string_lossy()
                .into_owned(),
        ),
        ("PYTHONHOME".into(), String::new()),
    ])
}

/// `PYTHONPATH` alone does not initialize wheel .pth files. Keep this standard
/// startup hook in the build-only namespace, which is excluded from artifacts.
fn materialize_python_build_startup(
    project_dir: &Path,
    dependency_root: &Path,
) -> anyhow::Result<PathBuf> {
    let directory = project_dir.join(".onreza/python/build/startup");
    ensure_python_directory(project_dir, &directory)?;
    let initialize = format!(
        "onreza_add_site_packages({}, ({}, {}))",
        serde_json::to_string(&dependency_root.to_string_lossy())?,
        serde_json::to_string(&project_dir.to_string_lossy())?,
        serde_json::to_string(&project_dir.join("src").to_string_lossy())?
    );
    let source = format!(
        "{}\n{initialize}\n{}\n{}",
        nrz_runtime_artifact::PYTHON_SITE_PACKAGES_INITIALIZER,
        "import importlib.machinery, importlib.util, pathlib, sys",
        r#"_startup_root = pathlib.Path(__file__).resolve().parent
_spec = importlib.machinery.PathFinder.find_spec("sitecustomize", [path for path in sys.path if pathlib.Path(path or ".").resolve() != _startup_root])
if _spec is not None and _spec.loader is not None:
    _module = importlib.util.module_from_spec(_spec)
    sys.modules["sitecustomize"] = _module
    _spec.loader.exec_module(_module)
"#
    );
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    std::io::Write::write_all(&mut temporary, source.as_bytes())?;
    temporary
        .persist(directory.join("sitecustomize.py"))
        .map_err(|error| error.error)?;
    Ok(directory)
}

const GENERATED_REQUIREMENTS: &str = ".onreza/python/build/requirements.txt";
const PROJECT_WHEELS: &str = ".onreza/python/build/wheels";
const PORTABLE_WHEEL_CHECK: &str = r#"from pathlib import Path
import sys
import zipfile
import json

wheels = list(Path(sys.argv[1]).glob('*.whl'))
if not wheels:
    sys.exit('Python application build produced no wheel')
native_extensions = tuple(json.loads(sys.argv[2]))
native_magic = tuple(bytes(values) for values in json.loads(sys.argv[3]))
for wheel in wheels:
    with zipfile.ZipFile(wheel) as archive:
        for entry in archive.infolist():
            if entry.is_dir():
                continue
            with archive.open(entry) as stream:
                header = stream.read(8)
            if (entry.filename.lower().endswith(native_extensions)
                    or any(header.startswith(magic) for magic in native_magic)):
                sys.exit('local Python application wheel contains native payload ' + entry.filename + '; use ONREZA Cloud Builder for qualified Linux builds or an explicitly qualified custom artifact')
"#;

/// Both paths use the pinned installer; Builder selects its frozen interpreter.
pub(super) fn install_command(
    manifest: &str,
    mode: PythonInstallMode,
    host_os: &str,
    host_arch: &str,
    minor: PythonMinor,
) -> anyhow::Result<PythonInstallCommand> {
    if manifest == "setup.py" && mode == PythonInstallMode::ManagedLocal {
        bail!(
            "setup.py source builds cannot be qualified for Linux x86_64 from {host_os}/{host_arch}; deploy through ONREZA Cloud Builder"
        );
    }
    let mut arguments = pip_arguments(mode, minor);
    if manifest == "setup.py" {
        arguments.push(OsString::from("."));
    } else {
        arguments.extend([OsString::from("--requirements"), OsString::from(manifest)]);
    }
    Ok(uv_command(minor, arguments, format!("install {manifest}")))
}

/// Validation/export/build/install are separate bounded operations so log phases
/// and failures retain their real boundary. No command edits authored manifests.
pub(super) fn install_commands(
    project_dir: &Path,
    mode: PythonInstallMode,
    host_os: &str,
    host_arch: &str,
    minor: PythonMinor,
) -> anyhow::Result<Vec<PythonInstallCommand>> {
    use crate::detect::python::{PythonDependencyKind, dependency_plan};
    let Some(plan) = dependency_plan(&crate::detect::fs::LocalFs::new(project_dir))? else {
        return Ok(Vec::new());
    };
    let generated = project_dir.join(".onreza/python/build");
    ensure_python_directory(project_dir, &generated)?;
    if matches!(
        plan.kind,
        PythonDependencyKind::Uv | PythonDependencyKind::Poetry
    ) {
        ensure_python_output_path(project_dir, &project_dir.join(GENERATED_REQUIREMENTS))?;
    }
    let setup_package =
        plan.kind == PythonDependencyKind::Requirements && project_dir.join("setup.py").is_file();
    if plan.install_project && !setup_package {
        ensure_python_output_path(project_dir, &project_dir.join(PROJECT_WHEELS))?;
    }
    let mut commands = Vec::new();
    let exact_version = minor.exact_version();
    if mode == PythonInstallMode::PinnedPlatform {
        commands.push(PythonInstallCommand {
            program: PathBuf::from(minor.platform_interpreter()),
            arguments: ["-I", "-c", "import sys; expected = sys.argv[1]; actual = '.'.join(map(str, sys.version_info[:3])); sys.exit(0 if actual == expected and sys.implementation.name == 'cpython' else 'frozen CPython differs from selected exact patch: ' + actual)", exact_version].map(OsString::from).to_vec(),
            display: format!("verify frozen CPython {exact_version}"),
        });
    }
    for (constraint, poetry_syntax) in [
        (plan.requires_python.as_deref(), false),
        (plan.poetry_requires_python.as_deref(), true),
    ] {
        let Some(constraint) = constraint else {
            continue;
        };
        let (package, script) = if poetry_syntax {
            (
                format!("poetry=={}", python_toolchain_versions().poetry),
                "from poetry.core.constraints.version import parse_constraint, Version; import sys; version = sys.argv[2]; supported = parse_constraint(sys.argv[1]).allows(Version.parse(version)); sys.exit(0 if supported else 'requires-python excludes selected CPython ' + version)",
            )
        } else {
            (
                format!("packaging=={}", python_toolchain_versions().packaging),
                "from packaging.specifiers import SpecifierSet; import sys; version = sys.argv[2]; supported = SpecifierSet(sys.argv[1]).contains(version); sys.exit(0 if supported else 'requires-python excludes selected CPython ' + version)",
            )
        };
        let mut validation = python_tool_command(
            mode,
            minor,
            &package,
            script,
            constraint,
            "validate requires-python",
        );
        validation.arguments.push(OsString::from(&exact_version));
        commands.push(validation);
    }
    let requirements = match plan.kind {
        PythonDependencyKind::Uv => {
            let mut arguments = vec![OsString::from("export")];
            arguments.extend(python_arguments(mode, minor));
            arguments.extend(
                [
                    "--locked",
                    "--no-dev",
                    "--no-default-groups",
                    "--no-editable",
                    "--no-emit-project",
                    "--format",
                    "requirements-txt",
                    "--output-file",
                    GENERATED_REQUIREMENTS,
                ]
                .map(OsString::from),
            );
            commands.push(uv_command(
                minor,
                arguments,
                "validate and export uv.lock".into(),
            ));
            GENERATED_REQUIREMENTS
        }
        PythonDependencyKind::Poetry => {
            commands.push(poetry_command(
                mode,
                minor,
                &["check", "--lock"],
                "validate poetry.lock",
            ));
            commands.push(poetry_command(
                mode,
                minor,
                &[
                    "export",
                    "--only",
                    "main",
                    "--format",
                    "requirements.txt",
                    "--output",
                    GENERATED_REQUIREMENTS,
                ],
                "export poetry.lock",
            ));
            GENERATED_REQUIREMENTS
        }
        PythonDependencyKind::Requirements => "requirements.txt",
        PythonDependencyKind::Project => "pyproject.toml",
        PythonDependencyKind::Setup => {
            commands.push(install_command(
                "setup.py", mode, host_os, host_arch, minor,
            )?);
            return Ok(commands);
        }
    };
    let mut install = pip_arguments(mode, minor);
    // Exported lock sets own the entire dependency solution, including direct URLs.
    if matches!(
        plan.kind,
        PythonDependencyKind::Uv | PythonDependencyKind::Poetry
    ) {
        install.push(OsString::from("--no-deps"));
    }
    install.extend([
        OsString::from("--requirements"),
        OsString::from(requirements),
    ]);
    commands.push(uv_command(
        minor,
        install,
        "install Python runtime dependencies".into(),
    ));
    if plan.install_project {
        if setup_package {
            let mut project = install_command("setup.py", mode, host_os, host_arch, minor)?;
            project.arguments.push(OsString::from("--no-deps"));
            project.display = format!(
                "pinned uv {} / CPython {}: install Python application without resolving dependencies",
                python_toolchain_versions().uv,
                minor.version()
            );
            commands.push(project);
            return Ok(commands);
        }
        let name = plan.project_name.as_deref().ok_or_else(|| {
            anyhow::anyhow!("installable Python project must declare its package name")
        })?;
        if !name.starts_with(|character: char| character.is_ascii_alphanumeric())
            || !name.bytes().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_' | b'.')
            })
        {
            bail!("Python project name is not a valid distribution name");
        }
        let mut build = vec![OsString::from("build"), OsString::from("--wheel")];
        build.extend(python_arguments(mode, minor));
        build.extend(
            [
                "--out-dir",
                PROJECT_WHEELS,
                "--clear",
                "--no-create-gitignore",
            ]
            .map(OsString::from),
        );
        commands.push(uv_command(
            minor,
            build,
            "build Python application wheel".into(),
        ));
        if mode == PythonInstallMode::ManagedLocal {
            let mut qualification = python_tool_command(
                mode,
                minor,
                &format!("packaging=={}", python_toolchain_versions().packaging),
                PORTABLE_WHEEL_CHECK,
                PROJECT_WHEELS,
                "qualify portable application wheel",
            );
            qualification.arguments.extend([
                OsString::from(serde_json::to_string(NATIVE_PAYLOAD_EXTENSIONS)?),
                OsString::from(serde_json::to_string(NATIVE_PAYLOAD_MAGICS)?),
            ]);
            commands.push(qualification);
        }
        let mut install = pip_arguments(mode, minor);
        install.extend(
            [
                "--no-deps",
                "--no-index",
                "--find-links",
                PROJECT_WHEELS,
                name,
            ]
            .map(OsString::from),
        );
        commands.push(uv_command(
            minor,
            install,
            "install Python application wheel".into(),
        ));
    }
    Ok(commands)
}

fn ensure_python_output_path(project_dir: &Path, output: &Path) -> anyhow::Result<()> {
    let root = project_dir.canonicalize()?;
    let mut ancestor = output;
    loop {
        match std::fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ancestor = ancestor.parent().context("invalid Python output path")?;
            }
            Err(error) => return Err(error).context("cannot inspect Python output path"),
        }
    }
    let canonical = ancestor.canonicalize()?;
    if !canonical.starts_with(&root) {
        bail!("Python output path escapes the project root");
    }
    if canonical != root.join(ancestor.strip_prefix(project_dir)?) {
        bail!("generated Python output paths must not contain symlinks");
    }
    Ok(())
}

pub(super) fn ensure_python_directory(project_dir: &Path, directory: &Path) -> anyhow::Result<()> {
    ensure_python_output_path(project_dir, directory)?;
    std::fs::create_dir_all(directory).context("cannot create Python output directory")?;
    ensure_python_output_path(project_dir, directory)
}

pub(super) fn python_arguments(mode: PythonInstallMode, minor: PythonMinor) -> Vec<OsString> {
    match mode {
        PythonInstallMode::ManagedLocal => ["--python", minor.exact_version(), "--managed-python"]
            .map(OsString::from)
            .to_vec(),
        PythonInstallMode::PinnedPlatform => [
            "--python",
            minor.platform_interpreter(),
            "--no-managed-python",
            "--no-python-downloads",
        ]
        .map(OsString::from)
        .to_vec(),
    }
}

fn pip_arguments(mode: PythonInstallMode, minor: PythonMinor) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("pip"), OsString::from("install")];
    arguments.extend(python_arguments(mode, minor));
    arguments.extend(
        [
            "--link-mode",
            "copy",
            "--target",
            minor.site_packages_root(),
        ]
        .map(OsString::from),
    );
    if mode == PythonInstallMode::ManagedLocal {
        arguments.extend(
            [
                "--python-platform",
                PLATFORM_PYTHON_TARGET,
                "--only-binary",
                ":all:",
            ]
            .map(OsString::from),
        );
    }
    arguments
}

fn python_tool_command(
    mode: PythonInstallMode,
    minor: PythonMinor,
    package: &str,
    script: &str,
    argument: &str,
    operation: &str,
) -> PythonInstallCommand {
    if mode == PythonInstallMode::PinnedPlatform {
        PythonInstallCommand {
            program: PathBuf::from("/opt/onreza/poetry/bin/python"),
            arguments: ["-I", "-c", script, argument].map(OsString::from).to_vec(),
            display: format!("frozen Python validator: {operation}"),
        }
    } else {
        let mut arguments = tool_arguments(mode, minor, package, package.starts_with("poetry=="));
        arguments.extend(["python", "-I", "-c", script, argument].map(OsString::from));
        uv_command(minor, arguments, operation.into())
    }
}

fn poetry_command(
    mode: PythonInstallMode,
    minor: PythonMinor,
    arguments: &[&str],
    operation: &str,
) -> PythonInstallCommand {
    if mode == PythonInstallMode::PinnedPlatform {
        PythonInstallCommand {
            program: PathBuf::from("/opt/onreza/poetry/bin/poetry"),
            arguments: arguments.iter().map(OsString::from).collect(),
            display: format!(
                "frozen Poetry {} / export {}: {operation}",
                python_toolchain_versions().poetry,
                python_toolchain_versions().poetry_export
            ),
        }
    } else {
        let mut command = tool_arguments(
            mode,
            minor,
            &format!("poetry=={}", python_toolchain_versions().poetry),
            true,
        );
        command.push(OsString::from("poetry"));
        command.extend(arguments.iter().map(OsString::from));
        uv_command(minor, command, operation.into())
    }
}

fn tool_arguments(
    mode: PythonInstallMode,
    minor: PythonMinor,
    package: &str,
    poetry_export: bool,
) -> Vec<OsString> {
    let mut arguments = [
        "tool",
        "run",
        "--isolated",
        "--no-env-file",
        "--from",
        package,
    ]
    .map(OsString::from)
    .to_vec();
    arguments.extend(python_arguments(mode, minor));
    if poetry_export {
        arguments.extend([
            OsString::from("--with"),
            OsString::from(format!(
                "poetry-plugin-export=={}",
                python_toolchain_versions().poetry_export
            )),
        ]);
    }
    arguments
}

fn uv_command(
    minor: PythonMinor,
    arguments: Vec<OsString>,
    operation: String,
) -> PythonInstallCommand {
    PythonInstallCommand {
        program: PathBuf::new(),
        arguments,
        display: format!(
            "pinned uv {} / CPython {}: {operation}",
            python_toolchain_versions().uv,
            minor.version()
        ),
    }
}

pub(super) fn artifact_for(os: &str, arch: &str) -> anyhow::Result<UvArtifact> {
    if python_toolchain_versions().uv != QUALIFIED_UV_VERSION {
        bail!(
            "selected uv engine has no qualified CLI archive hashes; refresh the pinned release artifacts before building"
        );
    }
    match (os, arch) {
        ("linux", "x86_64") => Ok(UvArtifact {
            target: "x86_64-unknown-linux-musl",
            archive_name: "uv-x86_64-unknown-linux-musl.tar.gz",
            archive_sha256: "1cff8783850e794470aadb73f54b749542a511fc57b0ce6468b64bd3852e0ade",
            binary_sha256: "ff3eba23dad69edbf72f5dc5e95c865a20091bd26514032f55909282f2b55bdd",
            binary_name: "uv",
            format: ArchiveFormat::TarGz,
        }),
        ("macos", "x86_64") => Ok(UvArtifact {
            target: "x86_64-apple-darwin",
            archive_name: "uv-x86_64-apple-darwin.tar.gz",
            archive_sha256: "960da44cb4b73685206ddd250b19e0a117fa41095710c1038f081f5cb613efb4",
            binary_sha256: "566a27247ff63f72260ed10c241fe4cf815b6c1b9439e17bdc808cf2d1c0d80f",
            binary_name: "uv",
            format: ArchiveFormat::TarGz,
        }),
        ("macos", "aarch64") => Ok(UvArtifact {
            target: "aarch64-apple-darwin",
            archive_name: "uv-aarch64-apple-darwin.tar.gz",
            archive_sha256: "50487ae565ccd96e499056b4674d438f4c53170202617b4c759defe0c6a1b544",
            binary_sha256: "2f7be1879e3337eef20875c0f80ef033337305d0405efc5d72642d6292f66b3b",
            binary_name: "uv",
            format: ArchiveFormat::TarGz,
        }),
        ("windows", "x86_64") => Ok(UvArtifact {
            target: "x86_64-pc-windows-msvc",
            archive_name: "uv-x86_64-pc-windows-msvc.zip",
            archive_sha256: "75d05de6762778c31ee183398de7dd15093fad0ed90b1f236d8205ea5ec00c90",
            binary_sha256: "70742ae9bd9f64f984fd69126662cfda5c03558bcb40075b2263833e4075f744",
            binary_name: "uv.exe",
            format: ArchiveFormat::Zip,
        }),
        _ => bail!("managed Python is not published for {os}/{arch}"),
    }
}

async fn download_archive(artifact: UvArtifact) -> anyhow::Result<Vec<u8>> {
    let url = format!(
        "{UV_RELEASE_ORIGIN}/astral-sh/uv/releases/download/{}/{}",
        python_toolchain_versions().uv,
        artifact.archive_name
    );
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(10 * 60))
        .build()
        .context("failed to create managed Python toolchain HTTP client")?;
    let response = client
        .get(&url)
        .header("User-Agent", "nrz-cli")
        .send()
        .await
        .with_context(|| {
            format!(
                "failed to download managed uv {}",
                python_toolchain_versions().uv
            )
        })?
        .error_for_status()
        .with_context(|| format!("uv release server rejected {url}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_UV_ARCHIVE_BYTES)
    {
        bail!("managed uv archive exceeds the download limit");
    }

    let mut archive = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("failed while downloading managed uv")?;
        if archive.len().saturating_add(chunk.len()) > MAX_UV_ARCHIVE_BYTES as usize {
            bail!("managed uv archive exceeds the download limit");
        }
        archive.extend_from_slice(&chunk);
    }
    if sha256_hex(&archive) != artifact.archive_sha256 {
        bail!("managed uv archive does not match the CLI pin");
    }
    Ok(archive)
}

fn extract_binary(archive: &[u8], artifact: UvArtifact) -> anyhow::Result<Vec<u8>> {
    match artifact.format {
        ArchiveFormat::TarGz => extract_binary_from_tar_gz(archive, artifact.binary_name),
        ArchiveFormat::Zip => extract_binary_from_zip(archive, artifact.binary_name),
    }
}

fn extract_binary_from_tar_gz(archive: &[u8], binary_name: &str) -> anyhow::Result<Vec<u8>> {
    let decoder = flate2::read::GzDecoder::new(archive);
    let mut tar = tar::Archive::new(decoder);
    let mut binary = None;
    for entry in tar.entries().context("failed to read managed uv archive")? {
        let entry = entry.context("invalid entry in managed uv archive")?;
        let path = entry.path().context("invalid path in managed uv archive")?;
        if path.file_name() != Some(OsStr::new(binary_name))
            || !entry.header().entry_type().is_file()
        {
            continue;
        }
        if binary.is_some() {
            bail!("managed uv archive contains duplicate binaries");
        }
        if entry.size() > MAX_UV_BINARY_BYTES {
            bail!("managed uv binary exceeds the extraction limit");
        }
        let mut bytes = Vec::new();
        entry
            .take(MAX_UV_BINARY_BYTES + 1)
            .read_to_end(&mut bytes)
            .context("failed to extract managed uv binary")?;
        if bytes.len() as u64 > MAX_UV_BINARY_BYTES {
            bail!("managed uv binary exceeds the extraction limit");
        }
        binary = Some(bytes);
    }
    binary.context("managed uv archive does not contain the expected binary")
}

#[cfg(windows)]
fn extract_binary_from_zip(archive: &[u8], binary_name: &str) -> anyhow::Result<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .context("failed to read managed uv zip archive")?;
    let mut binary = zip
        .by_name(binary_name)
        .context("managed uv archive does not contain the expected binary")?;
    if binary.size() > MAX_UV_BINARY_BYTES {
        bail!("managed uv binary exceeds the extraction limit");
    }
    let mut bytes = Vec::new();
    binary
        .take(MAX_UV_BINARY_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("failed to extract managed uv binary")?;
    if bytes.len() as u64 > MAX_UV_BINARY_BYTES {
        bail!("managed uv binary exceeds the extraction limit");
    }
    Ok(bytes)
}

#[cfg(not(windows))]
fn extract_binary_from_zip(_archive: &[u8], _binary_name: &str) -> anyhow::Result<Vec<u8>> {
    bail!("zip extraction is only available in the Windows nrz binary")
}

async fn cached_binary_is_valid(path: &Path, artifact: UvArtifact) -> anyhow::Result<bool> {
    let Ok(metadata) = fs::metadata(path).await else {
        return Ok(false);
    };
    if !metadata.is_file() || metadata.len() > MAX_UV_BINARY_BYTES {
        return Ok(false);
    }
    Ok(sha256_file(path).await? == artifact.binary_sha256)
}

async fn install_binary(path: &Path, bytes: &[u8], artifact: UvArtifact) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("managed uv cache path has no parent")?;
    fs::create_dir_all(parent)
        .await
        .with_context(|| format!("failed to create managed uv cache {}", parent.display()))?;
    let temporary_path = parent.join(format!(".uv-{}", Uuid::now_v7()));
    let mut file = fs::File::create(&temporary_path).await.with_context(|| {
        format!(
            "failed to create managed uv cache file {}",
            temporary_path.display()
        )
    })?;
    file.write_all(bytes)
        .await
        .context("failed to write managed uv cache")?;
    file.flush()
        .await
        .context("failed to flush managed uv cache")?;
    file.sync_all()
        .await
        .context("failed to persist managed uv cache")?;
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary_path, std::fs::Permissions::from_mode(0o755))
            .await
            .context("failed to make managed uv executable")?;
    }
    #[cfg(windows)]
    if fs::try_exists(path).await.unwrap_or(false) {
        let _ = fs::remove_file(path).await;
    }
    if let Err(error) = fs::rename(&temporary_path, path).await {
        let _ = fs::remove_file(&temporary_path).await;
        if !cached_binary_is_valid(path, artifact)
            .await
            .unwrap_or(false)
        {
            return Err(error)
                .with_context(|| format!("failed to install managed uv cache {}", path.display()));
        }
    }
    Ok(())
}

fn default_cache_root() -> anyhow::Result<PathBuf> {
    #[cfg(windows)]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(path));
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join("Library").join("Caches"));
    }

    #[cfg(not(windows))]
    if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .context("HOME or USERPROFILE is required to locate the nrz Python toolchain cache")?;
    Ok(PathBuf::from(home).join(".cache"))
}

async fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(path)
        .await
        .with_context(|| format!("failed to verify managed uv cache {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
            encoded
        })
}

#[cfg(test)]
#[path = "python_host_tests.rs"]
mod host_tests;
