use super::*;

// ── Build step ───────────────────────────────────────────────

/// Resolve build command. Priority: CLI flag > config > server > auto-detect.
pub(super) fn resolve_build_command(
    explicit: Option<&str>,
    project_dir: &Path,
    effective: &EffectiveProjectConfig,
) -> Option<String> {
    if let Some(cmd) = explicit {
        return Some(cmd.to_string());
    }
    if let Some(setting) = effective.build_command() {
        return setting.value().map(str::to_string);
    }
    // Only auto-detect if package.json has a "build" script
    let pkg = crate::detect::package_json::PackageJson::load(project_dir)?;
    if !pkg.scripts.contains_key("build") {
        return None;
    }
    let pm = crate::detect::detect_package_manager_name(project_dir);
    Some(format!("{pm} run build"))
}

/// Run a shell command while preserving terminal output and optionally shipping it.
///
/// In JSON mode: pipes stdout/stderr, wraps each line via `output::log_line()` on stderr.
/// In non-JSON mode without a remote sink: inherits stdio. With a sink it pipes
/// and tees the original bytes back to the matching terminal stream.
///
/// `child_stream` controls the `s` field for child stdout lines ("user" or "debug").
/// Child stderr always goes to "debug" stream with "warn" level.
pub(super) fn run_command_streaming(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    phase: output::Phase,
    child_stream: &str,
    extra_env: &[(String, String)],
    build_logs: Option<&BuildLogEmitter>,
) -> anyhow::Result<()> {
    #[cfg(unix)]
    let (shell, shell_args) = ("sh", ["-c", cmd]);
    #[cfg(windows)]
    let (shell, shell_args) = ("cmd", ["/C", cmd]);

    if !json && build_logs.is_none() {
        let status = std::process::Command::new(shell)
            .args(shell_args)
            .current_dir(project_dir)
            .envs(
                extra_env
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str())),
            )
            .status()
            .with_context(|| format!("failed to start command: {cmd}"))?;
        if !status.success() {
            match status.code() {
                Some(code) => {
                    return Err(output::coded_error(
                        format!("{}_EXIT_CODE", phase.as_str().to_uppercase()),
                        format!("{phase} command `{cmd}` failed with exit code {code}"),
                    ));
                }
                None => {
                    return Err(output::coded_error(
                        format!("{}_SIGNAL_KILLED", phase.as_str().to_uppercase()),
                        format!("{phase} process `{cmd}` was killed by signal"),
                    ));
                }
            }
        }
        return Ok(());
    }

    // Capture when JSON framing or centralized upload needs the child streams.
    let mut command = std::process::Command::new(shell);
    command
        .args(shell_args)
        .current_dir(project_dir)
        .envs(
            extra_env
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Run the build in its own process group. Build tools spawn long-lived
    // grandchildren (jest/SWC workers, an OG-image headless Chromium, dev
    // daemons) that inherit the stdout/stderr pipe write-ends. If one survives
    // the top-level command, the reader threads never observe EOF and the joins
    // after `wait()` block forever — which is how a *successful* build ends up
    // recorded as a 15-minute "build timeout". A dedicated group lets us reap
    // the orphans deterministically once the command itself has exited.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to start command: {cmd}"))?;

    #[cfg(unix)]
    let child_pid = child.id() as libc::pid_t;

    let stdout = child
        .stdout
        .take()
        .context("expected piped stdout on child process")?;
    let stderr = child
        .stderr
        .take()
        .context("expected piped stderr on child process")?;

    let phase_out = phase.to_string();
    let stream_out = child_stream.to_string();
    let build_stream_out = if child_stream == "debug" {
        BuildLogStream::Debug
    } else {
        BuildLogStream::User
    };
    let build_level_out = if child_stream == "debug" {
        BuildLogLevel::Debug
    } else {
        BuildLogLevel::Info
    };
    let build_logs_out = build_logs.cloned();
    let stdout_handle = std::thread::spawn(move || {
        forward_child_stream(ChildStreamRequest {
            reader: stdout,
            terminal: ChildTerminal::Stdout,
            json,
            frame_stream: stream_out,
            frame_level: "info",
            phase_name: phase_out,
            build_phase: build_log_phase(phase),
            build_stream: build_stream_out,
            build_level: build_level_out,
            origin: BuildLogOrigin::ChildStdout,
            build_logs: build_logs_out,
        });
    });

    let phase_err = phase.to_string();
    // Install stderr → "user" stream (errors visible to user), other phases follow child_stream
    let stream_err = if phase == output::Phase::Install {
        "user".to_string()
    } else {
        child_stream.to_string()
    };
    let build_stream_err = if stream_err == "debug" {
        BuildLogStream::Debug
    } else {
        BuildLogStream::User
    };
    let build_logs_err = build_logs.cloned();
    let stderr_handle = std::thread::spawn(move || {
        forward_child_stream(ChildStreamRequest {
            reader: stderr,
            terminal: ChildTerminal::Stderr,
            json,
            frame_stream: stream_err,
            frame_level: "warn",
            phase_name: phase_err,
            build_phase: build_log_phase(phase),
            build_stream: build_stream_err,
            build_level: BuildLogLevel::Warn,
            origin: BuildLogOrigin::ChildStderr,
            build_logs: build_logs_err,
        });
    });

    let status = child
        .wait()
        .with_context(|| format!("failed to wait for command: {cmd}"))?;

    // The command has exited; reap any orphaned grandchildren still holding the
    // pipe write-ends open in its process group. Without this the joins below
    // can hang until an external timeout. SIGKILL is safe here — survivors are
    // orphans of an already-terminated build. The group id stays allocated while
    // any member is alive, so signalling it after `wait()` reaped the leader
    // cannot hit a recycled pid.
    #[cfg(unix)]
    {
        // Negative pid targets the whole process group.
        unsafe { libc::kill(-child_pid, libc::SIGKILL) };
    }

    if let Err(e) = stdout_handle.join() {
        tracing::warn!("stdout reader thread panicked: {e:?}");
    }
    if let Err(e) = stderr_handle.join() {
        tracing::warn!("stderr reader thread panicked: {e:?}");
    }

    if !status.success() {
        match status.code() {
            Some(code) => {
                return Err(output::coded_error(
                    format!("{}_EXIT_CODE", phase.as_str().to_uppercase()),
                    format!("{phase} command failed with exit code {code}"),
                ));
            }
            None => {
                return Err(output::coded_error(
                    format!("{}_SIGNAL_KILLED", phase.as_str().to_uppercase()),
                    format!("{phase} process was killed by signal"),
                ));
            }
        }
    }
    Ok(())
}

enum ChildTerminal {
    Stdout,
    Stderr,
}

struct ChildStreamRequest<R> {
    reader: R,
    terminal: ChildTerminal,
    json: bool,
    frame_stream: String,
    frame_level: &'static str,
    phase_name: String,
    build_phase: BuildLogPhase,
    build_stream: BuildLogStream,
    build_level: BuildLogLevel,
    origin: BuildLogOrigin,
    build_logs: Option<BuildLogEmitter>,
}

fn forward_child_stream<R: std::io::Read>(request: ChildStreamRequest<R>) {
    use std::io::{BufRead, Write};

    let mut reader = std::io::BufReader::new(request.reader);
    let mut raw = Vec::new();
    loop {
        raw.clear();
        match reader.read_until(b'\n', &mut raw) {
            Ok(0) => break,
            Ok(_) => {
                if !request.json {
                    let write_result = match request.terminal {
                        ChildTerminal::Stdout => std::io::stdout().lock().write_all(&raw),
                        ChildTerminal::Stderr => std::io::stderr().lock().write_all(&raw),
                    };
                    if write_result.is_err() {
                        break;
                    }
                }
                let line = String::from_utf8_lossy(&raw);
                let line = line.trim_end_matches(['\r', '\n']);
                if line.is_empty() {
                    continue;
                }
                if request.json {
                    output::log_line(
                        &request.frame_stream,
                        request.frame_level,
                        &request.phase_name,
                        line,
                    );
                }
                if let Some(build_logs) = &request.build_logs {
                    build_logs.emit(
                        request.build_stream,
                        request.build_level,
                        request.build_phase,
                        request.origin,
                        line,
                    );
                }
            }
            Err(error) => {
                if request.json {
                    output::log_line(
                        "debug",
                        "warn",
                        &request.phase_name,
                        &format!("[nrz] failed to read child output: {error}"),
                    );
                }
                break;
            }
        }
    }
}

fn build_log_phase(phase: output::Phase) -> BuildLogPhase {
    match phase {
        output::Phase::Install => BuildLogPhase::Install,
        output::Phase::Build => BuildLogPhase::Build,
        _ => BuildLogPhase::Deploy,
    }
}

pub(super) fn run_install_step(
    project_dir: &Path,
    json: bool,
    effective: &EffectiveProjectConfig,
    build_logs: Option<&BuildLogEmitter>,
) -> anyhow::Result<()> {
    let Some(cmd) = resolve_install_command(project_dir, effective) else {
        return Ok(());
    };
    let (cmd, install_env) = prepare_install_command(&cmd, project_dir, json);

    output::status(
        json,
        ">",
        format!("Installing dependencies: {cmd}"),
        output::Phase::Deploy,
    );
    if let Some(build_logs) = build_logs {
        build_logs.info(BuildLogPhase::Install, "Installing dependencies");
    }
    // Install child output → debug stream (npm noise), nrz markers go through output::status/success
    run_command_streaming(
        &cmd,
        project_dir,
        json,
        output::Phase::Install,
        "debug",
        &install_env,
        build_logs,
    )?;
    output::success(json, "Dependencies installed", output::Phase::Deploy);
    if let Some(build_logs) = build_logs {
        build_logs.info(BuildLogPhase::Install, "Dependencies installed");
    }
    Ok(())
}

pub(super) fn resolve_install_command(
    project_dir: &Path,
    effective: &EffectiveProjectConfig,
) -> Option<String> {
    // Priority: effective config command > auto-detect from package manager.
    // PRESET server commands are filtered out while building EffectiveProjectConfig.
    if let Some(setting) = effective.install_command() {
        return setting.value().map(str::to_string);
    }
    if !project_dir.join("package.json").exists() {
        return None;
    }

    let local_fs = crate::detect::fs::LocalFs::new(project_dir);
    let pkg = crate::detect::package_json::PackageJson::load_from_fs(&local_fs);
    let pm_info = crate::detect::package_manager::detect_package_manager(&local_fs, pkg.as_ref());
    match pm_info {
        Some(info) => {
            Some(crate::detect::package_manager::install_command(info.pm_type).to_string())
        }
        None => Some("npm install".to_string()),
    }
}

pub(super) fn prepare_install_command(
    cmd: &str,
    project_dir: &Path,
    json: bool,
) -> (String, Vec<(String, String)>) {
    prepare_install_command_with_sandbox(cmd, project_dir, json, running_in_onreza_build_sandbox())
}

pub(super) fn prepare_install_command_with_sandbox(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    running_in_sandbox: bool,
) -> (String, Vec<(String, String)>) {
    if !should_apply_pnpm_build_scripts_compat(cmd, project_dir, running_in_sandbox) {
        return (cmd.to_string(), Vec::new());
    }

    output::status(
        json,
        "~",
        "pnpm install in build sandbox: allowing dependency build scripts (no project pnpm build policy found)",
        output::Phase::Deploy,
    );

    (cmd.to_string(), pnpm_build_scripts_compat_env())
}

pub(super) fn pnpm_build_scripts_compat_env() -> Vec<(String, String)> {
    PNPM_BUILD_SCRIPT_COMPAT_ENV
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

pub(super) fn should_apply_pnpm_build_scripts_compat(
    cmd: &str,
    project_dir: &Path,
    running_in_sandbox: bool,
) -> bool {
    running_in_sandbox
        && is_pnpm_install_command(cmd)
        && !has_explicit_pnpm_build_policy(project_dir)
}

pub(super) fn running_in_onreza_build_sandbox() -> bool {
    running_in_onreza_build_sandbox_from_env(|key| std::env::var(key).ok())
}

pub(super) fn running_in_onreza_build_sandbox_from_env(
    mut env: impl FnMut(&str) -> Option<String>,
) -> bool {
    env_value_is_truthy(env("NRZ_BUILD_SANDBOX").as_deref())
        || (env_value_is_truthy(env("ONREZA").as_deref())
            && env_value_is_truthy(env("CI").as_deref()))
}

pub(super) fn env_value_is_truthy(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

pub(super) fn is_pnpm_install_command(cmd: &str) -> bool {
    let tokens = shell_command_tokens(cmd);
    let Some(pnpm_index) = tokens.iter().position(|token| is_pnpm_command_token(token)) else {
        return false;
    };

    let mut skip_next = false;
    for token in tokens.iter().skip(pnpm_index + 1) {
        if is_shell_command_separator(token) {
            break;
        }
        if skip_next {
            skip_next = false;
            continue;
        }
        if pnpm_option_takes_value(token) {
            skip_next = !token.contains('=');
            continue;
        }
        if token.starts_with('-') {
            continue;
        }
        return matches!(token.as_str(), "install" | "i");
    }
    false
}

pub(super) fn shell_command_tokens(cmd: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = cmd.chars().peekable();

    while let Some(ch) = chars.next() {
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '&' | '|' if chars.peek() == Some(&ch) => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                chars.next();
                tokens.push(format!("{ch}{ch}"));
            }
            ';' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                tokens.push(";".to_string());
            }
            ch if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

pub(super) fn is_pnpm_command_token(token: &str) -> bool {
    let command = token
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(token)
        .trim_matches(|ch| ch == '"' || ch == '\'');
    command == "pnpm" || command.starts_with("pnpm@")
}

pub(super) fn is_shell_command_separator(token: &str) -> bool {
    matches!(token, "&&" | "||" | ";")
}

pub(super) fn pnpm_option_takes_value(token: &str) -> bool {
    matches!(
        token,
        "-C" | "--dir"
            | "--filter"
            | "--workspace-dir"
            | "--store-dir"
            | "--config"
            | "--package-import-method"
            | "--network-concurrency"
            | "--fetch-retries"
            | "--fetch-retry-factor"
            | "--fetch-retry-mintimeout"
            | "--fetch-retry-maxtimeout"
    ) || token.starts_with("--filter=")
        || token.starts_with("--dir=")
        || token.starts_with("--workspace-dir=")
        || token.starts_with("--store-dir=")
        || token.starts_with("--config.")
}

pub(super) fn has_explicit_pnpm_build_policy(project_dir: &Path) -> bool {
    for dir in project_dir.ancestors() {
        for file in [
            "pnpm-workspace.yaml",
            "pnpm-workspace.yml",
            ".npmrc",
            ".pnpmrc",
            "package.json",
        ] {
            let path = dir.join(file);
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            if file_contains_pnpm_build_policy(file, &contents) {
                return true;
            }
        }
    }
    false
}

pub(super) fn file_contains_pnpm_build_policy(file_name: &str, contents: &str) -> bool {
    if matches!(file_name, ".npmrc" | ".pnpmrc") {
        return rc_file_contains_pnpm_build_policy(contents);
    }

    if file_name == "package.json" {
        return package_json_contains_pnpm_build_policy(contents);
    }

    yaml_file_contains_pnpm_build_policy(contents)
}

pub(super) fn rc_file_contains_pnpm_build_policy(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            return false;
        }
        let Some((key, value)) = parse_rc_config_setting(line) else {
            return false;
        };
        pnpm_build_policy_setting_blocks_compat(key, value)
    })
}

pub(super) fn yaml_file_contains_pnpm_build_policy(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            return false;
        }
        let Some((key, value)) = parse_yaml_config_setting(line) else {
            return false;
        };
        pnpm_build_policy_setting_blocks_compat(key, value)
    })
}

pub(super) fn package_json_contains_pnpm_build_policy(contents: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) else {
        return false;
    };
    value
        .get("pnpm")
        .and_then(|pnpm| pnpm.as_object())
        .is_some_and(|pnpm| {
            pnpm.iter().any(|(key, value)| {
                let value = json_config_scalar(value);
                pnpm_build_policy_setting_blocks_compat(key, value.as_deref())
            })
        })
}

pub(super) fn parse_rc_config_setting(line: &str) -> Option<(&str, Option<&str>)> {
    if let Some((key, value)) = line.split_once('=') {
        return Some((clean_config_key(key)?, Some(value.trim())));
    }

    let mut parts = line.splitn(2, char::is_whitespace);
    let key = clean_config_key(parts.next()?)?;
    Some((key, parts.next().map(str::trim)))
}

pub(super) fn parse_yaml_config_setting(line: &str) -> Option<(&str, Option<&str>)> {
    let (key, value) = line.trim_start().split_once(':')?;
    let key = clean_config_key(key)?;
    let value = value.trim();
    Some((key, (!value.is_empty()).then_some(value)))
}

pub(super) fn clean_config_key(key: &str) -> Option<&str> {
    let key = key.trim().trim_matches(|ch| ch == '"' || ch == '\'').trim();
    (!key.is_empty()).then_some(key)
}

pub(super) fn json_config_scalar(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

pub(super) fn pnpm_build_policy_setting_blocks_compat(key: &str, value: Option<&str>) -> bool {
    match normalize_pnpm_build_policy_key(key).as_str() {
        "allowbuilds"
        | "dangerouslyallowallbuilds"
        | "onlybuiltdependencies"
        | "onlybuiltdependenciesfile"
        | "ignoredbuiltdependencies"
        | "neverbuiltdependencies" => true,
        "ignoredepscripts" | "strictdepbuilds" | "ignorescripts" => {
            config_bool_value(value).unwrap_or(true)
        }
        _ => false,
    }
}

pub(super) fn config_bool_value(value: Option<&str>) -> Option<bool> {
    let value = value?
        .split(['#', ';'])
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(|ch| ch == '"' || ch == '\'')
        .to_ascii_lowercase();
    match value.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub(super) fn normalize_pnpm_build_policy_key(key: &str) -> String {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect();
    normalized
        .strip_prefix("pnpm")
        .unwrap_or(&normalized)
        .to_string()
}

pub(super) fn run_build_step(
    cmd: &str,
    project_dir: &Path,
    json: bool,
    extra_env: &[(String, String)],
    build_logs: Option<&BuildLogEmitter>,
) -> anyhow::Result<()> {
    if cmd.trim().is_empty() {
        return Err(output::coded_error(
            "INVALID_CONFIG",
            "empty build command".to_string(),
        ));
    }

    output::status(json, ">", format!("Building: {cmd}"), output::Phase::Deploy);
    if let Some(build_logs) = build_logs {
        build_logs.info(BuildLogPhase::Build, "Build command started");
    }
    // Build child output → user stream (webpack/vite output is useful)
    run_command_streaming(
        cmd,
        project_dir,
        json,
        output::Phase::Build,
        "user",
        extra_env,
        build_logs,
    )?;
    output::success(json, "Build completed", output::Phase::Deploy);
    if let Some(build_logs) = build_logs {
        build_logs.info(BuildLogPhase::Build, "Build command completed");
    }
    Ok(())
}
