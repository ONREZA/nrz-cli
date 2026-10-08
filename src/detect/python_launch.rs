//! Pure Python launch inference; filesystem materialization belongs to deploy.

use anyhow::bail;

use super::fs::Fs;
use super::python::{
    PYTHON_ENTRY_CANDIDATES, dependency_plan, framework, framework_evidence_complete,
};

/// Generated bootstrap path consumed by Python launch packaging and artifact classification.
pub const PYTHON_BOOTSTRAP_ENTRY: &str = ".onreza/python/launch.py";

/// Authored Python launch selectors resolved independently of source materialization.
pub struct PythonLaunchRequest<'a> {
    pub entry: Option<&'a str>,
    pub module: Option<&'a str>,
    pub application: Option<&'a str>,
    pub server: Option<&'a str>,
    pub args: &'a [String],
}

/// Resolved entry and arguments consumed by the Python bootstrap materializer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PythonLaunch {
    pub entry: String,
    pub args: Vec<String>,
}

/// Validate Python launch intent and infer its server from the selected framework.
pub fn resolve_launch_for_framework(
    fs: &dyn Fs,
    request: PythonLaunchRequest<'_>,
    framework_override: Option<&str>,
) -> anyhow::Result<Option<PythonLaunch>> {
    dependency_plan(fs)?;
    if request.args.len() > 64
        || request
            .args
            .iter()
            .any(|arg| arg.len() > 4096 || arg.contains('\0'))
    {
        bail!("Python launch arguments exceed the immutable launch contract");
    }
    if [
        request.entry.is_some(),
        request.module.is_some(),
        request.application.is_some(),
    ]
    .into_iter()
    .filter(|set| *set)
    .count()
        > 1
    {
        bail!("Python entry, module and application are mutually exclusive");
    }
    if let Some(entry) = request.entry {
        if entry == PYTHON_BOOTSTRAP_ENTRY {
            bail!(
                "Python bootstrap entry is reserved; select deploy.module or application instead"
            );
        }
        if request.server.is_some() {
            bail!("Python server requires an application, not a script entry");
        }
        return Ok(Some(PythonLaunch {
            entry: entry.to_string(),
            args: request.args.to_vec(),
        }));
    }
    if let Some(module) = request.module {
        if request.server.is_some() {
            bail!("Python server requires an application, not a module launch");
        }
        validate_module(module)?;
        return Ok(Some(bootstrap_launch("MODULE", module, request.args)?));
    }
    // An authored callable owns its invocation; dependencies only infer a
    // server when no launch declaration exists.
    if request.application.is_none()
        && request.server.is_none()
        && let Some(application) = console_application(fs)?
    {
        return Ok(Some(bootstrap_launch(
            "CALLABLE",
            &application,
            request.args,
        )?));
    }
    let framework_override = super::accepted_framework_override(framework_override);
    let detected = match framework_override.as_deref() {
        Some(framework) => framework,
        None if request.server.is_some()
            && !fs.exists("manage.py")
            && !framework_evidence_complete(fs)? =>
        {
            "python"
        }
        None => framework(fs)?,
    };
    let server = request
        .server
        .map(parse_server)
        .transpose()?
        .or(match detected {
            "fastapi" | "starlette" => Some("ASGI"),
            "django" | "flask" => Some("WSGI"),
            _ => None,
        });
    if let Some(application) = request.application {
        validate_application(application)?;
        let server = server.unwrap_or("CALLABLE");
        if server == "WSGI"
            && application
                .split_once(':')
                .is_some_and(|(_, attribute)| attribute.contains('.'))
        {
            bail!(
                "Python WSGI application requires a simple callable name (module:app or module:create_app()); Gunicorn cannot import dotted attributes"
            );
        }
        return Ok(Some(bootstrap_launch(server, application, request.args)?));
    }
    if let Some(server) = server {
        let application = if detected == "django" {
            let suffix = if server == "ASGI" { "asgi" } else { "wsgi" };
            let mut applications = Vec::new();
            for base in ["", "src/"] {
                for child in fs.list_dir(base.trim_end_matches('/')) {
                    let package = format!("{base}{child}");
                    if fs.exists(&format!("{package}/{suffix}.py")) {
                        validate_module(&child)?;
                        applications.push(format!("{child}.{suffix}:application"));
                    }
                }
            }
            unique_application(applications)?
        } else {
            let applications = PYTHON_ENTRY_CANDIDATES
                .iter()
                .filter(|entry| fs.exists(entry) && !fs.is_dir(entry))
                .map(|entry| {
                    entry
                        .trim_start_matches("src/")
                        .trim_end_matches(".py")
                        .replace('/', ".")
                        + ":app"
                })
                .collect();
            unique_application(applications)?
        };
        return Ok(Some(bootstrap_launch(server, &application, request.args)?));
    }
    let entries: Vec<_> = PYTHON_ENTRY_CANDIDATES
        .iter()
        .filter(|entry| fs.exists(entry) && !fs.is_dir(entry))
        .collect();
    match entries.as_slice() {
        [] => Ok(None),
        [entry] => Ok(Some(PythonLaunch {
            entry: (*entry).to_string(),
            args: request.args.to_vec(),
        })),
        _ => bail!(
            "multiple Python entry candidates found; set deploy.entry, module or application explicitly"
        ),
    }
}

pub(crate) fn has_declared_console_scripts(fs: &dyn Fs) -> bool {
    console_scripts(fs).is_ok_and(|scripts| scripts.is_some_and(|scripts| !scripts.is_empty()))
}

fn console_scripts(fs: &dyn Fs) -> anyhow::Result<Option<toml::Table>> {
    let Some(text) = fs.read_file("pyproject.toml") else {
        return Ok(None);
    };
    let project: toml::Value = toml::from_str(&text)?;
    Ok(project
        .get("project")
        .and_then(|value| value.get("scripts"))
        .or_else(|| {
            project
                .get("tool")
                .and_then(|value| value.get("poetry"))
                .and_then(|value| value.get("scripts"))
        })
        .and_then(toml::Value::as_table)
        .cloned())
}

fn console_application(fs: &dyn Fs) -> anyhow::Result<Option<String>> {
    let Some(scripts) = console_scripts(fs)? else {
        return Ok(None);
    };
    if scripts.len() > 1 {
        bail!(
            "multiple declared Python console scripts found; set deploy.application = \"module:callable\" explicitly"
        )
    }
    let Some(reference) = scripts.values().next() else {
        return Ok(None);
    };
    let application = reference
        .as_str()
        .or_else(|| reference.get("reference").and_then(toml::Value::as_str))
        .ok_or_else(|| {
            anyhow::anyhow!("Python console script requires a module:callable reference")
        })?;
    if reference
        .get("type")
        .and_then(toml::Value::as_str)
        .is_some_and(|kind| kind != "console")
    {
        bail!("non-console Poetry scripts require an explicit deploy.entry or module");
    }
    validate_application(application)?;
    Ok(Some(application.to_string()))
}

fn parse_server(server: &str) -> anyhow::Result<&'static str> {
    match server {
        "asgi" | "uvicorn" => Ok("ASGI"),
        "wsgi" | "gunicorn" => Ok("WSGI"),
        _ => bail!("unsupported Python server {server}; use asgi/uvicorn or wsgi/gunicorn"),
    }
}

fn unique_application(applications: Vec<String>) -> anyhow::Result<String> {
    match applications.as_slice() {
        [application] => Ok(application.clone()),
        [] => bail!(
            "Python server application is not declared; set deploy.application = \"module:callable\""
        ),
        _ => bail!("multiple Python server applications found; set deploy.application explicitly"),
    }
}

fn validate_module(module: &str) -> anyhow::Result<()> {
    if module.len() > 4096
        || !module.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
    {
        bail!("Python module must be a dotted import name");
    }
    Ok(())
}

fn validate_application(application: &str) -> anyhow::Result<()> {
    let (module, attribute) = application
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Python application must be module:callable"))?;
    validate_module(module)?;
    validate_module(attribute.strip_suffix("()").unwrap_or(attribute))?;
    Ok(())
}

fn bootstrap_launch(mode: &str, target: &str, args: &[String]) -> anyhow::Result<PythonLaunch> {
    if args.len() > 62 {
        bail!(
            "Python module/server arguments exceed the immutable launch contract (62 user arguments)"
        );
    }
    Ok(PythonLaunch {
        entry: PYTHON_BOOTSTRAP_ENTRY.into(),
        args: [mode.to_string(), target.to_string()]
            .into_iter()
            .chain(args.iter().cloned())
            .collect(),
    })
}
