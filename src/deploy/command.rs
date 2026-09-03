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

pub(super) fn is_recursive_deploy_command(command: &str) -> bool {
    command.split([';', '&', '|']).any(|segment| {
        let mut tokens = segment.split_whitespace();
        let Some(first) = tokens
            .find(|token| !is_shell_assignment(token))
            .map(command_token_basename)
        else {
            return false;
        };
        if first == "nrz" || first.starts_with("nrz@") {
            return tokens.next().is_some_and(|token| token == "deploy");
        }
        if first != "npx" && first != "bunx" {
            return false;
        }
        let remaining = tokens.map(command_token_basename).collect::<Vec<_>>();
        remaining
            .windows(2)
            .any(|pair| (pair[0] == "nrz" || pair[0].starts_with("nrz@")) && pair[1] == "deploy")
    })
}

pub(super) fn is_shell_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

pub(super) fn command_token_basename(token: &str) -> String {
    token
        .trim_matches(['\'', '"'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(token)
        .to_ascii_lowercase()
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
        let mut command = std::process::Command::new(shell);
        command.args(shell_args).current_dir(project_dir).envs(
            extra_env
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
        remove_private_cli_environment(&mut command);
        let status = command
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
    remove_private_cli_environment(&mut command);

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
    execution_env: &[(String, String)],
    build_logs: Option<&BuildLogEmitter>,
) -> anyhow::Result<()> {
    let is_python =
        crate::detect::detect_with_framework_override(project_dir, effective.framework_override())
            .metadata
            .runtime
            .runtime_type
            == crate::detect::types::RuntimeType::Python;
    if is_python {
        let target = project_dir.join(crate::artifact::PYTHON_SITE_PACKAGES_ROOT);
        match std::fs::remove_dir_all(&target) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to clean generated Python dependencies at {}",
                        target.display()
                    )
                });
            }
        }
    }
    let Some(cmd) = resolve_install_command(project_dir, effective) else {
        return Ok(());
    };
    if is_recursive_deploy_command(&cmd) {
        return Err(output::coded_error(
            "INVALID_CONFIG",
            "`nrz deploy` cannot be used as an install command because the platform build runner already executes `nrz deploy`; use the package-manager install command, for example `npm ci`",
        ));
    }
    let (cmd, install_env) = prepare_install_command(&cmd, project_dir, json);
    let install_env = merge_command_environment(execution_env, &install_env);

    output::status(
        json,
        ">",
        format!("Installing dependencies: {cmd}"),
        output::Phase::Deploy,
    );
    if let Some(build_logs) = build_logs {
        build_logs.info(
            BuildLogPhase::Install,
            &format!("Installing dependencies: {cmd}"),
        );
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
    super::dependency_scripts::run_bun_dependency_scripts_if_needed(
        &cmd,
        project_dir,
        json,
        &install_env,
        build_logs,
    )?;
    output::success(json, "Dependencies installed", output::Phase::Deploy);
    if let Some(build_logs) = build_logs {
        build_logs.info(BuildLogPhase::Install, "Dependencies installed");
    }
    Ok(())
}

pub(super) fn merge_command_environment(
    base: &[(String, String)],
    overrides: &[(String, String)],
) -> Vec<(String, String)> {
    let mut merged = std::collections::BTreeMap::new();
    for (key, value) in base.iter().chain(overrides) {
        merged.insert(key.clone(), value.clone());
    }
    merged.into_iter().collect()
}

pub(super) fn remove_private_cli_environment(command: &mut std::process::Command) {
    for key in crate::execution_context::private_cli_environment_keys() {
        command.env_remove(key);
    }
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
    if crate::detect::detect_with_framework_override(project_dir, effective.framework_override())
        .metadata
        .runtime
        .runtime_type
        == crate::detect::types::RuntimeType::Python
    {
        let fs = crate::detect::fs::LocalFs::new(project_dir);
        return crate::detect::python::dependency_manifest(&fs)
            .map(crate::detect::python::install_command);
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
    _project_dir: &Path,
    _json: bool,
) -> (String, Vec<(String, String)>) {
    (cmd.to_string(), Vec::new())
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
    if is_recursive_deploy_command(cmd) {
        return Err(output::coded_error(
            "INVALID_CONFIG",
            "`nrz deploy` cannot be used as a build command because the platform build runner already executes `nrz deploy`; use the application build script, for example `npm run build`",
        ));
    }

    output::status(json, ">", format!("Building: {cmd}"), output::Phase::Deploy);
    if let Some(build_logs) = build_logs {
        build_logs.info(BuildLogPhase::Build, &format!("Building: {cmd}"));
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
        build_logs.info(BuildLogPhase::Build, "Build completed");
    }
    Ok(())
}
