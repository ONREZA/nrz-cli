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
        if !is_go_detection_filename(&name) {
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
                if !go_target_constraints(source) {
                    return false;
                }
                let Some(("package", rest)) = go_identifier(source) else {
                    return false;
                };
                matches!(go_identifier(rest), Some(("main", rest)) if !go_imports_c(rest))
            })
    })
}

// Remote content discovery covers exactly the package directories inspected above.
// Build constraints and package declarations still require the fetched contents.
pub(super) fn is_go_detection_source_path(path: &str) -> bool {
    let mut parts = path.split('/');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(name), None, None, None) => is_go_detection_filename(name),
        (Some("cmd"), Some(directory), Some(name), None) if !directory.is_empty() => {
            is_go_detection_filename(name)
        }
        _ => false,
    }
}

fn is_go_detection_filename(name: &str) -> bool {
    name.ends_with(".go")
        && !name.ends_with("_test.go")
        && !name.starts_with(['.', '_'])
        && go_target_filename(name)
}

// Match the automatic recipe, independent of the detector host or installed Go:
// linux/amd64, GOAMD64=v1, CGO_ENABLED=0, empty GOFLAGS and GOEXPERIMENT.
// Filename/header semantics follow the qualified Go SDK's go/build package.
fn go_target_filename(name: &str) -> bool {
    const OS: &[&str] = &[
        "aix",
        "android",
        "darwin",
        "dragonfly",
        "freebsd",
        "hurd",
        "illumos",
        "ios",
        "js",
        "linux",
        "nacl",
        "netbsd",
        "openbsd",
        "plan9",
        "solaris",
        "wasip1",
        "windows",
        "zos",
    ];
    const ARCH: &[&str] = &[
        "386",
        "amd64",
        "amd64p32",
        "arm",
        "armbe",
        "arm64",
        "arm64be",
        "loong64",
        "mips",
        "mipsle",
        "mips64",
        "mips64le",
        "mips64p32",
        "mips64p32le",
        "ppc",
        "ppc64",
        "ppc64le",
        "riscv",
        "riscv64",
        "s390",
        "s390x",
        "sparc",
        "sparc64",
        "wasm",
    ];
    let stem = name.split('.').next().unwrap_or(name);
    let Some((_, suffix)) = stem.split_once('_') else {
        return true;
    };
    let parts: Vec<_> = suffix.split('_').collect();
    let last = parts[parts.len() - 1];
    if parts.len() >= 2 && OS.contains(&parts[parts.len() - 2]) && ARCH.contains(&last) {
        return parts[parts.len() - 2] == "linux" && last == "amd64";
    }
    !(OS.contains(&last) || ARCH.contains(&last)) || matches!(last, "linux" | "amd64")
}

fn go_target_tag(tag: &str) -> bool {
    // Baseline tool tags observed in the pinned Go 1.27.1 linux/amd64 SDK.
    // Compiler upgrades must qualify these together with the automatic recipe.
    if matches!(
        tag,
        "linux"
            | "amd64"
            | "unix"
            | "gc"
            | "amd64.v1"
            | "goexperiment.regabiwrappers"
            | "goexperiment.regabiargs"
            | "goexperiment.dwarf5"
            | "goexperiment.randomizedheapbase64"
            | "goexperiment.greenteagc"
            | "goexperiment.jsonv2"
            | "goexperiment.sizespecializedmalloc"
    ) {
        return true;
    }
    static RELEASE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let release = *RELEASE.get_or_init(|| {
        let pins: serde_json::Value =
            serde_json::from_str(include_str!("../../assets/native-toolchains.json"))
                .expect("trusted native toolchain pins");
        pins["go"]
            .as_str()
            .expect("qualified Go version")
            .strip_prefix("1.")
            .expect("qualified Go 1.x version")
            .split('.')
            .next()
            .unwrap()
            .parse()
            .expect("qualified Go minor version")
    });
    tag.strip_prefix("go1.")
        .and_then(|minor| minor.parse::<usize>().ok())
        .is_some_and(|minor| minor > 0 && minor <= release && tag == format!("go1.{minor}"))
}

fn go_directive<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(prefix)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

fn go_target_constraints(source: &str) -> bool {
    let mut modern = None;
    let mut legacy_end = 0;
    let mut offset = 0;
    let mut ended = false;
    let mut block_comment = false;
    'header: for raw_line in source.split_inclusive('\n') {
        offset += raw_line.len();
        let mut line = raw_line.trim();
        if line.is_empty() && !ended {
            legacy_end = offset;
            continue;
        }
        if !line.starts_with("//") {
            ended = true;
        }
        if !block_comment
            && let Some(expression) = go_directive(line, "//go:build")
            && modern.replace(expression).is_some()
        {
            return false;
        }
        loop {
            if block_comment {
                let Some((_, rest)) = line.split_once("*/") else {
                    continue 'header;
                };
                block_comment = false;
                line = rest.trim();
            } else if line.starts_with("//") || line.is_empty() {
                continue 'header;
            } else if let Some(rest) = line.strip_prefix("/*") {
                block_comment = true;
                line = rest.trim();
            } else {
                break 'header;
            }
        }
    }
    if let Some(expression) = modern {
        return go_boolean_constraint(expression).unwrap_or(false);
    }
    source[..legacy_end].lines().all(|line| {
        let Some(comment) = line.trim().strip_prefix("//") else {
            return true;
        };
        let Some(expression) = go_directive(comment.trim(), "+build") else {
            return true;
        };
        // go/build ignores legacy expressions exceeding the old parser's limit.
        go_legacy_constraint(expression).unwrap_or(true)
    })
}

fn go_tag_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '.')
}

fn go_valid_tag(tag: &str) -> bool {
    static TAG: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    TAG.get_or_init(|| regex::Regex::new(r"^[\p{L}\p{Nd}_.]+$").unwrap())
        .is_match(tag)
}

fn go_legacy_constraint(expression: &str) -> Option<bool> {
    let mut size = 0;
    let mut result = false;
    for (clause_index, clause) in expression.split_whitespace().enumerate() {
        size += usize::from(clause_index > 0);
        let mut matches = true;
        for (index, literal) in clause.split(',').enumerate() {
            size += usize::from(index > 0);
            if size > 100 {
                return None;
            }
            let (negative, tag) = literal
                .strip_prefix('!')
                .map_or((false, literal), |tag| (true, tag));
            let value = go_valid_tag(tag) && go_target_tag(tag);
            matches &= if literal.starts_with("!!") || literal == "!" {
                false
            } else {
                value != negative
            };
        }
        result |= matches;
    }
    Some(result)
}

// Evaluate with explicit stacks so even deeply nested untrusted expressions do
// not consume the Rust call stack. Go allows at most 1000 expression terms.
fn go_boolean_constraint(mut expression: &str) -> Option<bool> {
    let mut values = Vec::new();
    let mut operators = Vec::new();
    let mut operand = true;
    let mut size = 0;
    loop {
        expression = expression.trim_start_matches([' ', '\t']);
        let Some(first) = expression.chars().next() else {
            break;
        };
        if operand {
            if first == '!' && operators.last() != Some(&'!') {
                operators.push('!');
                expression = &expression[1..];
                continue;
            }
            size += 1;
            if size > 1000 {
                return None;
            }
            if first == '(' {
                operators.push('(');
                expression = &expression[1..];
                continue;
            }
            let end = expression
                .char_indices()
                .find(|(_, ch)| !go_tag_character(*ch))
                .map_or(expression.len(), |(offset, _)| offset);
            if end == 0 || !go_valid_tag(&expression[..end]) {
                return None;
            }
            values.push(go_target_tag(&expression[..end]));
            expression = &expression[end..];
            operand = false;
        } else if first == ')' {
            while operators.last().is_some_and(|op| *op != '(') {
                go_reduce_operator(&mut values, operators.pop()?)?;
            }
            if operators.pop()? != '(' {
                return None;
            }
            expression = &expression[1..];
        } else {
            let (operator, rest) = if let Some(rest) = expression.strip_prefix("&&") {
                ('&', rest)
            } else {
                ('|', expression.strip_prefix("||")?)
            };
            while operators
                .last()
                .is_some_and(|op| *op == '&' || (*op == '|' && operator == '|'))
            {
                go_reduce_operator(&mut values, operators.pop()?)?;
            }
            operators.push(operator);
            expression = rest;
            operand = true;
        }
        if operators.last() == Some(&'!') {
            operators.pop();
            go_reduce_operator(&mut values, '!')?;
        }
    }
    if operand {
        return None;
    }
    while let Some(operator) = operators.pop() {
        go_reduce_operator(&mut values, operator)?;
    }
    (values.len() == 1).then(|| values[0])
}

fn go_reduce_operator(values: &mut Vec<bool>, operator: char) -> Option<()> {
    let right = values.pop()?;
    let value = match operator {
        '!' => !right,
        '&' => values.pop()? & right,
        '|' => values.pop()? | right,
        _ => return None,
    };
    values.push(value);
    Some(())
}

fn go_skip_trivia(mut source: &str) -> Option<&str> {
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
    Some(source)
}

// The initial package clause and imports are enough for discovery; compilation
// remains responsible for the complete source's validity and a main function.
fn go_identifier(source: &str) -> Option<(&str, &str)> {
    let source = go_skip_trivia(source)?;
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

fn go_imports_c(mut source: &str) -> bool {
    loop {
        let Some(rest) = go_skip_trivia(source) else {
            return false;
        };
        source = rest.trim_start_matches(';');
        let Some(("import", rest)) = go_identifier(source) else {
            return false;
        };
        let Some(rest) = go_skip_trivia(rest) else {
            return false;
        };
        let grouped = rest.starts_with('(');
        source = if grouped { &rest[1..] } else { rest };
        loop {
            let Some(rest) = go_skip_trivia(source) else {
                return false;
            };
            source = rest.trim_start_matches(';');
            let Some(rest) = go_skip_trivia(source) else {
                return false;
            };
            source = rest;
            if grouped && let Some(rest) = source.strip_prefix(')') {
                source = rest;
                break;
            }
            if let Some(rest) = source.strip_prefix('.') {
                source = rest;
            } else if let Some((_, rest)) = go_identifier(source) {
                source = rest;
            }
            let Some((path, rest)) = go_import_string(source) else {
                return false;
            };
            if path == "C" {
                return true;
            }
            source = rest;
            if !grouped {
                break;
            }
        }
    }
}

fn go_import_string(source: &str) -> Option<(String, &str)> {
    let source = go_skip_trivia(source)?;
    if let Some(raw) = source.strip_prefix('`') {
        let (value, rest) = raw.split_once('`')?;
        return Some((value.replace('\r', ""), rest));
    }
    let mut chars = source.strip_prefix('"')?.char_indices();
    let mut value = String::new();
    while let Some((offset, character)) = chars.next() {
        match character {
            '"' => return Some((value, &source[offset + 2..])),
            '\n' | '\r' => return None,
            '\\' => {
                let (_, escaped) = chars.next()?;
                let (digits, radix) = match escaped {
                    'x' => (2, 16),
                    'u' => (4, 16),
                    'U' => (8, 16),
                    '0'..='7' => (3, 8),
                    'a' => {
                        value.push('\x07');
                        continue;
                    }
                    'b' => {
                        value.push('\x08');
                        continue;
                    }
                    'f' => {
                        value.push('\x0c');
                        continue;
                    }
                    'n' => {
                        value.push('\n');
                        continue;
                    }
                    'r' => {
                        value.push('\r');
                        continue;
                    }
                    't' => {
                        value.push('\t');
                        continue;
                    }
                    'v' => {
                        value.push('\x0b');
                        continue;
                    }
                    '\\' | '"' => {
                        value.push(escaped);
                        continue;
                    }
                    _ => return None,
                };
                let mut code = if radix == 8 {
                    escaped.to_digit(radix)?
                } else {
                    0
                };
                for _ in usize::from(radix == 8)..digits {
                    code = code * radix + chars.next()?.1.to_digit(radix)?;
                }
                value.push(char::from_u32(code)?);
            }
            _ => value.push(character),
        }
    }
    None
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
