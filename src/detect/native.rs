//! Build-language detection; serving remains STATIC or an executable PROCESS.

use super::fs::Fs;
use super::types::{
    BuildInfo, ComputeType, DetectionMetadata, DetectionResult, PackageManagerInfo,
    PackageManagerType, RuntimeInfo, RuntimeType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRecipe {
    FlutterWeb,
    DartServer,
    GoServer,
    HugoStatic,
}

impl NativeRecipe {
    pub fn output_directory(self) -> &'static str {
        match self {
            Self::FlutterWeb => "build/web",
            Self::DartServer => "build/onreza-dart/bundle",
            Self::GoServer => "build/onreza-go",
            Self::HugoStatic => "public",
        }
    }
}

pub const NATIVE_RUNTIME_TARGET: &str = nrz_runtime_artifact::NATIVE_EXECUTION_TARGET;

pub fn native_recipe(framework: &str) -> Option<NativeRecipe> {
    match framework {
        "flutter" => Some(NativeRecipe::FlutterWeb),
        "dart" => Some(NativeRecipe::DartServer),
        "go" => Some(NativeRecipe::GoServer),
        "hugo" => Some(NativeRecipe::HugoStatic),
        _ => None,
    }
}

pub fn detect_native(fs: &dyn Fs) -> Option<DetectionResult> {
    // Hugo modules use go.mod; the generator owns the output contract.
    let recipe = if hugo_config(fs).is_some() {
        NativeRecipe::HugoStatic
    } else if let Some(pubspec) = pubspec(fs) {
        if is_flutter(&pubspec) {
            // A mobile-only Flutter project has no qualified web build input.
            if !fs.exists("web/index.html") {
                return None;
            }
            NativeRecipe::FlutterWeb
        } else if !dart_entries(fs).is_empty() {
            NativeRecipe::DartServer
        } else {
            return None;
        }
    } else if fs.exists("go.mod") && !go_main_packages(fs).is_empty() {
        NativeRecipe::GoServer
    } else {
        return None;
    };
    Some(detection(fs, recipe, false))
}

pub fn detect_configured_native(fs: &dyn Fs, framework: &str) -> Option<DetectionResult> {
    Some(detection(fs, native_recipe(framework)?, true))
}

fn pubspec(fs: &dyn Fs) -> Option<serde_yaml_ng::Value> {
    serde_yaml_ng::from_str(&fs.read_file("pubspec.yaml")?).ok()
}

fn is_flutter(pubspec: &serde_yaml_ng::Value) -> bool {
    pubspec
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("flutter"))
        .and_then(|flutter| flutter.get("sdk"))
        .and_then(serde_yaml_ng::Value::as_str)
        == Some("flutter")
}

pub fn dart_entries(fs: &dyn Fs) -> Vec<String> {
    fs.list_dir("bin")
        .into_iter()
        .map(|name| format!("bin/{name}"))
        .filter(|path| path.ends_with(".dart") && !fs.is_dir(path))
        .collect()
}

pub fn go_main_packages(fs: &dyn Fs) -> Vec<String> {
    let mut packages = Vec::new();
    if is_go_main_package(fs, "") {
        packages.push(".".to_string());
    }
    for name in fs.list_dir("cmd") {
        let path = format!("cmd/{name}");
        if fs.is_dir(&path) && is_go_main_package(fs, &path) {
            packages.push(format!("./cmd/{name}"));
        }
    }
    packages
}

fn is_go_main_package(fs: &dyn Fs, directory: &str) -> bool {
    fs.list_dir(directory).into_iter().any(|name| {
        if !name.ends_with(".go") || name.ends_with("_test.go") || name.starts_with(['.', '_']) {
            return false;
        }
        let path = if directory.is_empty() {
            name
        } else {
            format!("{directory}/{name}")
        };
        !fs.is_dir(&path)
            && fs.read_file(&path).is_some_and(|source| {
                let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
                let Some(("package", rest)) = go_identifier(source) else {
                    return false;
                };
                matches!(go_identifier(rest), Some(("main", _)))
            })
    })
}

// Only the initial package clause determines discovery. Go compilation remains
// responsible for build constraints and the rest of the source's validity.
fn go_identifier(mut source: &str) -> Option<(&str, &str)> {
    loop {
        source = source.trim_start_matches([' ', '\t', '\r', '\n']);
        if let Some(comment) = source.strip_prefix("//") {
            source = comment.split_once('\n').map_or("", |(_, rest)| rest);
        } else if let Some(comment) = source.strip_prefix("/*") {
            source = comment.split_once("*/")?.1;
        } else {
            break;
        }
    }
    let first = source.chars().next()?;
    if first != '_' && !first.is_alphabetic() {
        return None;
    }
    let end = source
        .char_indices()
        .find(|(_, character)| *character != '_' && !character.is_alphanumeric())
        .map_or(source.len(), |(offset, _)| offset);
    Some(source.split_at(end))
}

fn hugo_config(fs: &dyn Fs) -> Option<&'static str> {
    [
        "hugo.toml",
        "hugo.yaml",
        "hugo.json",
        "config/_default/hugo.toml",
        "config/_default/hugo.yaml",
        "config/_default/hugo.json",
    ]
    .into_iter()
    .find(|path| fs.exists(path) && !fs.is_dir(path))
}

fn detection(fs: &dyn Fs, recipe: NativeRecipe, configured: bool) -> DetectionResult {
    let (framework, name, compute, runtime, manager, build, install, entry) = match recipe {
        NativeRecipe::FlutterWeb => (
            "flutter",
            "Flutter Web",
            ComputeType::Static,
            RuntimeType::Dart,
            Some(PackageManagerType::Pub),
            Some("flutter build web --release"),
            Some("flutter pub get --enforce-lockfile"),
            None,
        ),
        NativeRecipe::DartServer => (
            "dart",
            "Dart",
            ComputeType::Process,
            RuntimeType::Dart,
            Some(PackageManagerType::Pub),
            None,
            Some("dart pub get --enforce-lockfile"),
            unique(dart_entries(fs))
                .and_then(|entry| entry.strip_suffix(".dart").map(str::to_string)),
        ),
        NativeRecipe::GoServer => (
            "go",
            "Go",
            ComputeType::Process,
            RuntimeType::Go,
            Some(PackageManagerType::Go),
            None,
            None,
            Some("server".to_string()),
        ),
        NativeRecipe::HugoStatic => (
            "hugo",
            "Hugo",
            ComputeType::Static,
            RuntimeType::Go,
            None,
            Some("hugo --environment production --destination public --cleanDestinationDir"),
            None,
            None,
        ),
    };
    let config_files = if recipe == NativeRecipe::HugoStatic {
        hugo_config(fs).into_iter().map(str::to_string).collect()
    } else if recipe == NativeRecipe::GoServer {
        ["go.mod", "go.sum", "go.work"]
            .into_iter()
            .filter(|path| fs.exists(path))
            .map(str::to_string)
            .collect()
    } else {
        ["pubspec.yaml", "pubspec.lock"]
            .into_iter()
            .filter(|path| fs.exists(path))
            .map(str::to_string)
            .collect()
    };
    DetectionResult {
        framework: framework.into(),
        name: name.into(),
        version: None,
        suggested_compute: compute,
        metadata: DetectionMetadata {
            source_build_context: None,
            uses_typescript: None,
            config_files,
            runtime: RuntimeInfo {
                runtime_type: runtime,
                version: None,
            },
            package_manager: manager.map(|pm_type| PackageManagerInfo {
                pm_type,
                version: None,
                lockfile: if pm_type == PackageManagerType::Pub {
                    fs.exists("pubspec.lock")
                        .then(|| "pubspec.lock".to_string())
                } else {
                    fs.exists("go.sum").then(|| "go.sum".to_string())
                },
            }),
            build_info: Some(BuildInfo {
                build_command: build.map(str::to_string),
                install_command: install.map(str::to_string),
                output_dir: Some(recipe.output_directory().into()),
                entry_point: entry,
            }),
            monorepo: None,
            ssr_analysis: None,
            structure: Vec::new(),
        },
        reason: format!(
            "{} {name} build recipe",
            if configured { "Configured" } else { "Detected" }
        ),
    }
}

fn unique(mut entries: Vec<String>) -> Option<String> {
    (entries.len() == 1).then(|| entries.remove(0))
}
