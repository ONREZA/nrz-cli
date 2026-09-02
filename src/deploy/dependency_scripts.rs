use super::*;

const ALLOW_DEPENDENCY_SCRIPTS_ENV: &str = "NRZ_BUILD_ALLOW_DEPENDENCY_SCRIPTS";
const ZERO_UNTRUSTED_DEPENDENCIES: &[u8] = b"Found 0 untrusted dependencies with scripts.";

struct MetadataSnapshot {
    path: PathBuf,
    contents: Vec<u8>,
}

pub(super) fn run_bun_dependency_scripts_if_needed(
    install_command: &str,
    project_dir: &Path,
    json: bool,
    install_env: &[(String, String)],
    build_logs: Option<&BuildLogEmitter>,
) -> anyhow::Result<()> {
    if !allow_dependency_scripts() || !is_bun_install_command(install_command) {
        return Ok(());
    }
    run_bun_dependency_scripts(project_dir, json, install_env, build_logs)
}

fn allow_dependency_scripts() -> bool {
    std::env::var(ALLOW_DEPENDENCY_SCRIPTS_ENV)
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

pub(super) fn is_bun_install_command(command: &str) -> bool {
    let first_command = command.split([';', '&', '|']).next().unwrap_or_default();
    let mut tokens = first_command.split_whitespace();
    let Some(executable) = tokens
        .find(|token| !is_shell_assignment(token))
        .map(command_token_basename)
    else {
        return false;
    };
    if executable != "bun" {
        return false;
    }
    matches!(tokens.next(), Some("install" | "i"))
        && !tokens.any(|token| token == "--ignore-scripts")
}

pub(super) fn run_bun_dependency_scripts(
    project_dir: &Path,
    json: bool,
    install_env: &[(String, String)],
    build_logs: Option<&BuildLogEmitter>,
) -> anyhow::Result<()> {
    let output = bun_untrusted_output(project_dir, install_env)?;
    if contains_bytes(&output.stdout, ZERO_UNTRUSTED_DEPENDENCIES)
        || contains_bytes(&output.stderr, ZERO_UNTRUSTED_DEPENDENCIES)
    {
        return Ok(());
    }

    let snapshots = snapshot_bun_metadata(project_dir)?;
    output::status(
        json,
        ">",
        "Running blocked Bun dependency scripts",
        output::Phase::Deploy,
    );
    if let Some(build_logs) = build_logs {
        build_logs.info(
            BuildLogPhase::Install,
            "Running blocked Bun dependency scripts",
        );
    }

    let trust_result = run_command_streaming(
        "bun pm trust --all",
        project_dir,
        json,
        output::Phase::Install,
        "debug",
        install_env,
        build_logs,
    );
    restore_bun_metadata(&snapshots)?;
    trust_result?;

    output::success(
        json,
        "Bun dependency scripts completed",
        output::Phase::Deploy,
    );
    if let Some(build_logs) = build_logs {
        build_logs.info(BuildLogPhase::Install, "Bun dependency scripts completed");
    }
    Ok(())
}

fn bun_untrusted_output(
    project_dir: &Path,
    install_env: &[(String, String)],
) -> anyhow::Result<std::process::Output> {
    let mut command = std::process::Command::new("bun");
    command
        .args(["pm", "untrusted"])
        .current_dir(project_dir)
        .envs(
            install_env
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
    remove_private_cli_environment(&mut command);
    let output = command
        .output()
        .context("failed to inspect blocked Bun dependency scripts")?;
    if !output.status.success() {
        return Err(output::coded_error(
            "INSTALL_EXIT_CODE",
            format!(
                "failed to inspect blocked Bun dependency scripts with exit code {}",
                output.status.code().unwrap_or(1)
            ),
        ));
    }
    Ok(output)
}

fn snapshot_bun_metadata(project_dir: &Path) -> anyhow::Result<Vec<MetadataSnapshot>> {
    let root = bun_project_root(project_dir);
    ["package.json", "bun.lock", "bun.lockb"]
        .into_iter()
        .filter_map(|name| {
            let path = root.join(name);
            path.exists().then_some(path)
        })
        .map(|path| {
            let contents = std::fs::read(&path)
                .with_context(|| format!("failed to snapshot {}", path.display()))?;
            Ok(MetadataSnapshot { path, contents })
        })
        .collect()
}

fn restore_bun_metadata(snapshots: &[MetadataSnapshot]) -> anyhow::Result<()> {
    for snapshot in snapshots {
        std::fs::write(&snapshot.path, &snapshot.contents)
            .with_context(|| format!("failed to restore {}", snapshot.path.display()))?;
    }
    Ok(())
}

fn bun_project_root(project_dir: &Path) -> &Path {
    project_dir
        .ancestors()
        .find(|directory| {
            directory.join("bun.lock").exists() || directory.join("bun.lockb").exists()
        })
        .unwrap_or(project_dir)
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
