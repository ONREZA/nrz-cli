//! Qualified build recipes. Commands use argv; native execution is a shared SDK contract.

use std::path::Path;

use anyhow::{Context, bail};
use serde::Deserialize;

use crate::detect::fs::{Fs, LocalFs};
use crate::detect::native::{NATIVE_RUNTIME_TARGET, NativeRecipe, dart_entries, go_main_packages};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeCommand {
    pub program: String,
    pub arguments: Vec<String>,
}

impl NativeCommand {
    fn new(program: &str, arguments: &[&str]) -> Self {
        Self {
            program: program.into(),
            arguments: arguments.iter().map(|arg| (*arg).into()).collect(),
        }
    }

    pub(crate) fn display(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.arguments.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeBuildPlan {
    pub build: NativeCommand,
    pub environment: Vec<(String, String)>,
    pub output_directory: String,
    pub executable_entry: Option<String>,
}

#[derive(Debug)]
pub(crate) struct NativeOutputEvidence {
    pub entry: Option<String>,
    pub target: Option<&'static str>,
    pub interpreter: Option<String>,
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
}

pub(crate) fn apply_flutter_static_cache_policy(manifest: &mut crate::build::manifest::Manifest) {
    let static_layers = manifest
        .layers
        .iter()
        .filter(|layer| layer.target == crate::build::manifest::LayerTarget::Static)
        .map(|layer| layer.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    for route in manifest
        .routes
        .iter_mut()
        .filter(|route| static_layers.contains(route.layer.as_str()))
    {
        let headers = route.headers.get_or_insert_with(Default::default);
        if !headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("cache-control"))
        {
            headers.insert("Cache-Control".into(), "no-cache".into());
        }
    }
}

pub(crate) fn compiler_probe(recipe: NativeRecipe) -> NativeCommand {
    match recipe {
        NativeRecipe::FlutterWeb => NativeCommand::new("flutter", &["--version", "--machine"]),
        NativeRecipe::DartServer => NativeCommand::new("dart", &["--version"]),
        NativeRecipe::GoServer => NativeCommand::new("go", &["version"]),
        NativeRecipe::HugoStatic => NativeCommand::new("hugo", &["version"]),
    }
}

/// A version probe is an observation, never a compiler execution attestation.
pub(crate) fn compiler_version(recipe: NativeRecipe, observed: &str) -> anyhow::Result<String> {
    let version = match recipe {
        NativeRecipe::FlutterWeb => {
            let value: serde_json::Value = serde_json::from_str(observed)?;
            return value
                .get("frameworkVersion")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .context("Flutter version probe lacks frameworkVersion");
        }
        NativeRecipe::DartServer => observed
            .trim()
            .strip_prefix("Dart SDK version: ")
            .and_then(|value| value.split_whitespace().next()),
        NativeRecipe::GoServer => observed
            .trim()
            .strip_prefix("go version go")
            .and_then(|value| value.split_whitespace().next()),
        NativeRecipe::HugoStatic => observed
            .trim()
            .strip_prefix("hugo v")
            .and_then(|value| value.split_whitespace().next())
            .map(|version| version.split(['-', '+']).next().unwrap_or(version)),
    };
    version
        .filter(|version| !version.is_empty())
        .map(str::to_string)
        .context("native compiler version probe returned an unsupported response")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeToolchainPins {
    schema: String,
    go: String,
    dart: String,
    flutter: String,
    flutter_dart: String,
    hugo: String,
    flutter_commit: String,
    flutter_engine: String,
}

pub(crate) async fn validate_compiler_before_execution(
    recipe: NativeRecipe,
    platform_runner: bool,
    environment: &[(String, String)],
) -> anyhow::Result<()> {
    let pins: NativeToolchainPins = if platform_runner {
        let path = Path::new("/opt/onreza/toolchains/versions.json");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            for path in [
                path,
                path.parent().unwrap(),
                path.parent().unwrap().parent().unwrap(),
            ] {
                let metadata = std::fs::metadata(path)
                    .context("frozen native toolchain pins are unavailable")?;
                if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
                    bail!(
                        "frozen native toolchain pins must be root-owned and not writable by group or others"
                    );
                }
            }
        }
        serde_json::from_slice(
            &std::fs::read(path).context("frozen native toolchain pins are unavailable")?,
        )?
    } else {
        serde_json::from_str(include_str!("../../assets/native-toolchains.json"))?
    };
    if pins.schema != "onreza.native-toolchains.v1" {
        bail!("unsupported native toolchain catalog schema");
    }
    let mut probe = compiler_probe(recipe);
    select_platform_program(&mut probe, platform_runner);
    let mut command = tokio::process::Command::new(&probe.program);
    command
        .args(&probe.arguments)
        .envs(environment.iter().cloned())
        .kill_on_drop(true);
    for key in crate::execution_context::private_cli_environment_keys() {
        command.env_remove(key);
    }
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), command.output())
        .await
        .context("native compiler version probe timed out")?
        .with_context(|| {
            format!(
                "{} is unavailable; install the selected native compiler or use Git/Builder",
                probe.program
            )
        })?;
    if !result.status.success() {
        bail!("native compiler version probe failed");
    }
    // Flutter's machine JSON is stdout; stderr can contain SDK diagnostics.
    // Dart reports its version on stderr, so retain both streams for text probes.
    let observed = if recipe == NativeRecipe::FlutterWeb {
        String::from_utf8_lossy(&result.stdout).into_owned()
    } else {
        format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        )
    };
    let actual = compiler_version(recipe, &observed)?;
    let expected = match recipe {
        NativeRecipe::FlutterWeb => &pins.flutter,
        NativeRecipe::DartServer => &pins.dart,
        NativeRecipe::GoServer => &pins.go,
        NativeRecipe::HugoStatic => &pins.hugo,
    };
    if &actual != expected {
        bail!(
            "native compiler version {actual} differs from qualified version {expected}; select the qualified compiler or use Git/Builder"
        );
    }
    if recipe == NativeRecipe::FlutterWeb {
        let value: serde_json::Value = serde_json::from_str(&observed)?;
        if value
            .get("frameworkRevision")
            .and_then(serde_json::Value::as_str)
            != Some(pins.flutter_commit.as_str())
            || value
                .get("engineRevision")
                .and_then(serde_json::Value::as_str)
                != Some(pins.flutter_engine.as_str())
            || value
                .get("dartSdkVersion")
                .and_then(serde_json::Value::as_str)
                != Some(pins.flutter_dart.as_str())
        {
            bail!(
                "Flutter compiler revision, engine, or bundled Dart differs from the qualified toolchain"
            );
        }
    }
    Ok(())
}

pub(crate) fn select_platform_program(command: &mut NativeCommand, platform_runner: bool) {
    if platform_runner {
        command.program = format!("/usr/local/bin/{}", command.program);
    } else if cfg!(windows) && command.program == "flutter" {
        command.program = "flutter.bat".into();
    }
}

pub(crate) fn default_command(
    explicit: Option<&str>,
    setting: Option<&crate::config::SourceAwareSetting>,
) -> bool {
    explicit.is_none()
        && setting.is_none_or(|setting| {
            setting.value().is_none() && !setting.source_or_preset().is_user_explicit()
        })
}

pub(crate) fn install_command(
    project_dir: &Path,
    recipe: NativeRecipe,
) -> anyhow::Result<Option<NativeCommand>> {
    match recipe {
        NativeRecipe::DartServer | NativeRecipe::FlutterWeb => {
            require_pub_inputs(&LocalFs::new(project_dir))?;
            Ok(Some(NativeCommand::new(
                if recipe == NativeRecipe::FlutterWeb {
                    "flutter"
                } else {
                    "dart"
                },
                &["pub", "get", "--enforce-lockfile"],
            )))
        }
        NativeRecipe::GoServer | NativeRecipe::HugoStatic => Ok(None),
    }
}

pub(crate) struct GoModuleInputs {
    _temporary: tempfile::TempDir,
    modfile: std::path::PathBuf,
    original_module: Vec<u8>,
    original_sums: Option<Vec<u8>>,
}

impl GoModuleInputs {
    pub(crate) fn freeze(project_dir: &Path, command: &mut NativeCommand) -> anyhow::Result<Self> {
        let temporary = tempfile::tempdir()?;
        let modfile = temporary.path().join("onreza.mod");
        let original_module = std::fs::read(project_dir.join("go.mod"))?;
        let original_sums = match std::fs::read(project_dir.join("go.sum")) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        std::fs::write(&modfile, &original_module)?;
        if let Some(sums) = &original_sums {
            std::fs::write(modfile.with_extension("sum"), sums)?;
        }
        command
            .arguments
            .insert(1, format!("-modfile={}", modfile.display()));
        Ok(Self {
            _temporary: temporary,
            modfile,
            original_module,
            original_sums,
        })
    }

    pub(crate) fn verify(&self) -> anyhow::Result<()> {
        let module = std::fs::read(&self.modfile)?;
        let sums = match std::fs::read(self.modfile.with_extension("sum")) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if module != self.original_module || sums != self.original_sums {
            bail!(
                "Go build requires changes to go.mod/go.sum; generate and review dependency inputs before deploying"
            );
        }
        Ok(())
    }
}

/// Hugo overrides GOFLAGS and writes its module checksums while loading config.
/// Keep those writes in an isolated source tree, then require reviewed inputs.
pub(crate) struct HugoModuleInputs {
    _temporary: tempfile::TempDir,
    source: std::path::PathBuf,
    inputs: Vec<(&'static str, Option<Vec<u8>>)>,
}

impl HugoModuleInputs {
    pub(crate) fn freeze(project_dir: &Path, command: &mut NativeCommand) -> anyhow::Result<Self> {
        if project_dir.join("go.mod").exists() {
            ensure_go_inputs(&LocalFs::new(project_dir))?;
        }
        let temporary = tempfile::tempdir()?;
        let root = project_dir.canonicalize()?;
        let staging = temporary.path().canonicalize()?;
        let source = staging.join("source");
        copy_hugo_source(&root, &source, &root, &staging)?;
        // Versioned config imports can resolve without go.mod and otherwise
        // omit hugo.direct.sum entirely. The private module makes Hugo record
        // that checksum, without introducing an authored module for plain sites.
        if !source.join("go.mod").exists() {
            std::fs::write(
                source.join("go.mod"),
                "module onreza.invalid/frozen-hugo\n\ngo 1.18\n",
            )?;
        }
        let inputs = ["go.mod", "go.sum", "hugo.direct.sum"]
            .into_iter()
            .map(|name| Ok((name, read_hugo_module_input(&source.join(name))?)))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let index = command
            .arguments
            .iter()
            .position(|arg| arg == "--destination")
            .context("Hugo recipe requires an explicit destination")?;
        let output = command
            .arguments
            .get_mut(index + 1)
            .context("Hugo recipe destination has no value")?;
        *output = project_dir
            .canonicalize()?
            .join(&*output)
            .to_string_lossy()
            .into_owned();
        command
            .arguments
            .extend(["--source".into(), source.to_string_lossy().into_owned()]);
        Ok(Self {
            _temporary: temporary,
            source,
            inputs,
        })
    }

    pub(crate) fn verify(&self) -> anyhow::Result<()> {
        for (name, original) in &self.inputs {
            if &read_hugo_module_input(&self.source.join(name))? != original {
                bail!(
                    "Hugo build requires changes to dependency inputs ({name}); generate and review go.mod/go.sum/hugo.direct.sum before deploying"
                );
            }
        }
        Ok(())
    }
}

fn read_hugo_module_input(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn copy_hugo_source(
    source: &Path,
    destination: &Path,
    root: &Path,
    staging: &Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        // TMPDIR may be inside the project. Omit only this invocation's
        // generated staging tree, while preserving other authored temp paths.
        if path == staging {
            continue;
        }
        let output = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            let target = std::fs::read_link(&path)?;
            let resolved = path.canonicalize()?;
            if target.is_absolute() || !resolved.starts_with(root) {
                bail!(
                    "Hugo source symlinks must be relative and contained in the project; use an explicit build.command for external source inputs"
                );
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, output)?;
            #[cfg(windows)]
            if resolved.is_dir() {
                std::os::windows::fs::symlink_dir(target, output)?;
            } else {
                std::os::windows::fs::symlink_file(target, output)?;
            }
        } else if kind.is_dir() {
            copy_hugo_source(&path, &output, root, staging)?;
        } else if kind.is_file() {
            std::fs::copy(path, output)?;
        } else {
            bail!("Hugo source inputs must be regular files, directories, or contained symlinks");
        }
    }
    Ok(())
}

pub(crate) fn recipe_commands(
    project_dir: &Path,
    recipe: NativeRecipe,
    platform_runner: bool,
) -> anyhow::Result<NativeBuildPlan> {
    let fs = LocalFs::new(project_dir);
    let mut environment = Vec::new();
    let (build, executable_entry) = match recipe {
        NativeRecipe::FlutterWeb => {
            require_pub_inputs(&fs)?;
            if !fs.exists("web/index.html") {
                bail!(
                    "Flutter Web requires web/index.html; mobile and desktop outputs are unsupported"
                );
            }
            (
                NativeCommand::new("flutter", &["build", "web", "--release", "--no-pub"]),
                None,
            )
        }
        NativeRecipe::DartServer => {
            require_pub_inputs(&fs)?;
            let entries = dart_entries(&fs);
            let [entry] = entries.as_slice() else {
                bail!(
                    "Dart build requires one bin/*.dart entry; set build.command and deploy.entry for an explicit application"
                );
            };
            validate_relative_path(entry)?;
            if !platform_runner && std::env::consts::OS != "linux" && fs.is_dir("hook") {
                bail!(
                    "Dart native hooks require the qualified Linux Builder; use a Git/Builder deployment"
                );
            }
            // A native hook may be transitive and becomes visible after pub get.
            if !platform_runner && std::env::consts::OS != "linux" {
                ensure_no_native_hooks(project_dir)?;
            }
            let executable = entry
                .strip_suffix(".dart")
                .context("invalid Dart entry")?
                .to_string();
            (
                NativeCommand::new(
                    "dart",
                    &[
                        "build",
                        "cli",
                        "--target",
                        entry,
                        "--target-os=linux",
                        "--target-arch=x64",
                        "--output=build/onreza-dart",
                    ],
                ),
                Some(executable),
            )
        }
        NativeRecipe::GoServer => {
            ensure_go_inputs(&fs)?;
            let packages = go_main_packages(&fs);
            let [package] = packages.as_slice() else {
                bail!(
                    "Go build requires one main package at the root or cmd/*; set build.command and deploy.entry for an explicit application"
                );
            };
            environment.extend([
                ("GOOS".into(), "linux".into()),
                ("GOARCH".into(), "amd64".into()),
                ("GOAMD64".into(), "v1".into()),
                ("CGO_ENABLED".into(), "0".into()),
                ("GO111MODULE".into(), "on".into()),
                ("GOTOOLCHAIN".into(), "local".into()),
                ("GOWORK".into(), "off".into()),
                ("GOENV".into(), "off".into()),
                ("GOFLAGS".into(), String::new()),
                ("GOEXPERIMENT".into(), String::new()),
            ]);
            (
                NativeCommand::new(
                    "go",
                    &[
                        "build",
                        "-mod=readonly",
                        "-trimpath",
                        "-o",
                        "build/onreza-go/server",
                        package,
                    ],
                ),
                Some("server".into()),
            )
        }
        NativeRecipe::HugoStatic => {
            // A workspace can redirect Go's module/checksum writes outside the
            // isolated site. The default recipe supports one frozen module.
            environment.extend([
                ("HUGO_MODULE_WORKSPACE".into(), "off".into()),
                ("GOTOOLCHAIN".into(), "local".into()),
                ("GOENV".into(), "off".into()),
            ]);
            (
                NativeCommand::new(
                    "hugo",
                    &[
                        "--environment",
                        "production",
                        "--destination",
                        "public",
                        "--cleanDestinationDir",
                    ],
                ),
                None,
            )
        }
    };
    Ok(NativeBuildPlan {
        build,
        environment,
        output_directory: recipe.output_directory().into(),
        executable_entry,
    })
}

fn require_pub_inputs(fs: &dyn Fs) -> anyhow::Result<()> {
    for input in ["pubspec.yaml", "pubspec.lock"] {
        if !fs.exists(input) || fs.is_dir(input) {
            bail!(
                "Pub production build requires committed {input}; generate and review the lockfile before deploying"
            );
        }
    }
    Ok(())
}

fn ensure_go_inputs(fs: &dyn Fs) -> anyhow::Result<()> {
    let module = fs.read_file("go.mod").context("Go build requires go.mod")?;
    if fs.exists("go.work") {
        bail!(
            "Go workspace builds require an explicit build.command; the first recipe supports one module"
        );
    }
    for line in module.lines() {
        let line = line.split_once("//").map_or(line, |(code, _)| code);
        if let Some((_, target)) = line.split_once("=>") {
            let target = target
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('"');
            if target.starts_with('.')
                || target.starts_with('/')
                || target.contains('\\')
                || target.as_bytes().get(1) == Some(&b':')
            {
                bail!(
                    "Go local replacement paths require an explicit build.command and qualified source inclusion"
                );
            }
        }
    }
    Ok(())
}

/// Run after pub get as well: transitive hooks are absent before materialization.
pub(crate) fn ensure_no_native_hooks(project_dir: &Path) -> anyhow::Result<()> {
    let config_path = project_dir.join(".dart_tool/package_config.json");
    let text = match std::fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("cannot read Dart package configuration"),
    };
    let config: serde_json::Value =
        serde_json::from_str(&text).context("invalid Dart package configuration")?;
    let base = url::Url::from_file_path(&config_path)
        .map_err(|_| anyhow::anyhow!("invalid Dart package configuration path"))?;
    for package in config
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .context("Dart package configuration lacks packages")?
    {
        let root = package
            .get("rootUri")
            .and_then(serde_json::Value::as_str)
            .context("Dart package configuration lacks rootUri")?;
        let root = base
            .join(root)?
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("unsupported Dart package root URI"))?;
        if root.join("hook/build.dart").exists() || root.join("hook/link.dart").exists() {
            bail!(
                "Dart native hooks require the qualified Linux Builder; use a Git/Builder deployment"
            );
        }
    }
    Ok(())
}

pub(crate) fn validate_output(
    output_dir: &Path,
    launch_cwd: &Path,
    recipe: NativeRecipe,
    entry: Option<&str>,
) -> anyhow::Result<NativeOutputEvidence> {
    if !output_dir.is_dir() {
        bail!("native recipe output is missing: {}", output_dir.display());
    }
    if matches!(recipe, NativeRecipe::FlutterWeb | NativeRecipe::HugoStatic) {
        if recipe == NativeRecipe::FlutterWeb && !output_dir.join("index.html").is_file() {
            bail!("STATIC recipe output lacks index.html");
        }
        return Ok(NativeOutputEvidence {
            entry: None,
            target: None,
            interpreter: None,
            libraries: Vec::new(),
            library_paths: Vec::new(),
        });
    }
    let entry = entry.context("native PROCESS requires an explicit executable output entry")?;
    validate_relative_path(entry)?;
    let root = output_dir.canonicalize()?;
    let binary_path = output_dir.join(entry);
    let binary = binary_path
        .canonicalize()
        .context("native executable entry is missing")?;
    if !binary.starts_with(&root) || !binary.is_file() {
        bail!("native executable entry escapes its artifact output directory");
    }
    let (requirements, _) =
        nrz_runtime_artifact::NativeExecutableRequirements::verify_artifact_closure(
            output_dir,
            &binary_path,
            launch_cwd,
        )?;
    Ok(NativeOutputEvidence {
        entry: Some(entry.into()),
        target: Some(NATIVE_RUNTIME_TARGET),
        interpreter: requirements.interpreter,
        libraries: requirements.libraries,
        library_paths: requirements.library_paths,
    })
}

fn validate_relative_path(path: &str) -> anyhow::Result<()> {
    if path.is_empty()
        || path.contains(['\0', '\\'])
        || path.as_bytes().get(1) == Some(&b':')
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        bail!("native entry must be a normalized relative artifact path");
    }
    Ok(())
}
