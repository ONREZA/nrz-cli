//! Resolve application intent before install/build; never execute package scripts.

use anyhow::{Context, bail};
use nrz_source_bundle::{ApplicationRuntimeDeclaration, ApplicationRuntimeFamily};
use serde::Deserialize;

use super::fs::Fs;

#[derive(Debug, Default, Deserialize)]
struct RuntimeProject {
    #[serde(default)]
    deploy: RuntimeDeploy,
}

#[derive(Debug, Default, Deserialize)]
struct RuntimeDeploy {
    runtime: Option<ApplicationRuntimeFamily>,
    #[serde(alias = "entrypoint")]
    entry: Option<String>,
    args: Option<Vec<String>>,
}

pub fn resolve_application_runtime(
    fs: &dyn Fs,
    framework: &str,
) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    let config = fs
        .read_file("onreza.toml")
        .map(|text| toml::from_str::<RuntimeProject>(&text))
        .transpose()
        .context("invalid application runtime declaration in onreza.toml")?
        .unwrap_or_default()
        .deploy;
    resolve_application_runtime_with_config(
        fs,
        framework,
        config.runtime,
        config.entry,
        config.args,
    )
}

pub fn resolve_application_runtime_with_config(
    fs: &dyn Fs,
    framework: &str,
    runtime: Option<ApplicationRuntimeFamily>,
    entry: Option<String>,
    args: Option<Vec<String>>,
) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    let config = RuntimeDeploy {
        runtime,
        entry,
        args,
    };
    let package = fs
        .read_file("package.json")
        .map(|text| serde_json::from_str::<super::package_json::PackageJson>(&text))
        .transpose()
        .context("invalid package.json while resolving application runtime")?;
    let script = package
        .as_ref()
        .and_then(|package| package.scripts.get("start"));
    if config.runtime.is_some()
        && script
            .and_then(|script| {
                direct_launcher_family(script.split_whitespace().next().unwrap_or(""))
            })
            .is_some_and(|family| Some(family) != config.runtime)
    {
        bail!("[deploy] runtime conflicts with scripts.start launcher");
    }
    let complete = config.runtime.is_some() && config.entry.is_some() && config.args.is_some();
    let start = if complete {
        None
    } else {
        script
            .map(|script| direct_start(script))
            .transpose()?
            .flatten()
    };
    if config.runtime.is_some()
        && start
            .as_ref()
            .is_some_and(|start| Some(start.family) != config.runtime)
    {
        bail!("[deploy] runtime conflicts with scripts.start launcher");
    }
    let family = config
        .runtime
        .or_else(|| start.as_ref().map(|start| start.family));
    let Some(family) = family else {
        if config.args.is_some() {
            bail!("[deploy] args requires an explicit runtime or direct bun/node start");
        }
        return Ok(None);
    };
    if framework == "python" || (framework == "elysia" && family != ApplicationRuntimeFamily::Bun) {
        bail!("application runtime conflicts with framework {framework}");
    }
    let entry = config
        .entry
        .or_else(|| start.as_ref().and_then(|start| start.entry.clone()))
        .map(|entry| normalize_application_entry(&entry))
        .transpose()?;
    let args = config
        .args
        .unwrap_or_else(|| start.map_or_else(Vec::new, |start| start.args));
    let declaration = ApplicationRuntimeDeclaration {
        family,
        entry,
        args,
    };
    declaration
        .intent()
        .validate()
        .map_err(anyhow::Error::msg)?;
    Ok(Some(declaration))
}

pub fn normalize_application_entry(entry: &str) -> anyhow::Result<String> {
    if entry.len() > 4096 {
        bail!("application entry exceeds 4096 UTF-8 bytes");
    }
    let entry = entry.trim().replace('\\', "/");
    let entry = entry.strip_prefix("./").unwrap_or(&entry);
    let mut words = entry.split_whitespace();
    let first = words.next().unwrap_or("");
    if entry.contains(':')
        || (words.next().is_some()
            && matches!(
                first,
                "bun" | "node" | "npm" | "pnpm" | "yarn" | "python" | "python3"
            ))
    {
        bail!("application entry must be a relative file path, not a command or URL");
    }
    nrz_source_bundle::normalize_source_path(entry).map_err(anyhow::Error::msg)
}

fn direct_start(script: &str) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    let mut words = script.split_whitespace();
    let Some(executor) = words.next() else {
        return Ok(None);
    };
    let family = match direct_launcher_family(executor) {
        Some(family) => family,
        None => {
            if words.any(|word| matches!(word.trim_matches(['\'', '"']), "bun" | "node")) {
                bail!(
                    "ambiguous scripts.start launcher; declare [deploy] runtime, entry and args explicitly"
                );
            }
            return Ok(None);
        }
    };
    if script.chars().any(|character| {
        matches!(
            character,
            '\'' | '"'
                | '`'
                | '$'
                | '|'
                | '&'
                | ';'
                | '<'
                | '>'
                | '('
                | ')'
                | '\n'
                | '\r'
                | '*'
                | '?'
                | '['
                | ']'
                | '\\'
        )
    }) {
        bail!(
            "unsupported scripts.start shell syntax; declare [deploy] runtime, entry and args explicitly"
        );
    }
    let mut entry = words
        .next()
        .context("scripts.start requires an application file")?;
    if family == ApplicationRuntimeFamily::Bun && entry == "run" {
        entry = words
            .next()
            .context("bun run requires an application file")?;
    }
    if entry.starts_with('-') || !super::looks_like_script_path_token(entry) {
        bail!(
            "unsupported scripts.start interpreter flags or script alias; declare [deploy] runtime, entry and args explicitly"
        );
    }
    Ok(Some(ApplicationRuntimeDeclaration {
        family,
        entry: Some(normalize_application_entry(entry)?),
        args: words.map(str::to_string).collect(),
    }))
}

fn direct_launcher_family(executor: &str) -> Option<ApplicationRuntimeFamily> {
    match executor.trim_matches(['\'', '"']) {
        "bun" => Some(ApplicationRuntimeFamily::Bun),
        "node" => Some(ApplicationRuntimeFamily::Node),
        _ => None,
    }
}
