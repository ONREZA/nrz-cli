use super::*;

use std::ffi::OsString;
use std::process::Stdio;

use nrz::config::{IgnoredBuildBehavior, ProjectBuildSettings};

const GIT_DIFF_TIMEOUT: Duration = Duration::from_secs(30);
const USER_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const GIT_REPOSITORY_ENVIRONMENT_KEYS: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum IgnoredBuildOutcome {
    Continue { reason: String },
    Skip { reason: String },
}

pub(super) struct IgnoredBuildRequest<'a> {
    pub settings: &'a ProjectBuildSettings,
    pub environment_type: &'a str,
    pub project_dir: &'a Path,
    pub execution_env: &'a [(String, String)],
    pub json: bool,
    pub build_logs: Option<&'a BuildLogEmitter>,
}

#[derive(Clone, Copy)]
struct IgnoredBuildTimeouts {
    git_diff: Duration,
    user_command: Duration,
}

impl Default for IgnoredBuildTimeouts {
    fn default() -> Self {
        Self {
            git_diff: GIT_DIFF_TIMEOUT,
            user_command: USER_COMMAND_TIMEOUT,
        }
    }
}

pub(super) async fn evaluate(
    request: IgnoredBuildRequest<'_>,
) -> anyhow::Result<IgnoredBuildOutcome> {
    evaluate_with_timeouts(request, IgnoredBuildTimeouts::default()).await
}

async fn evaluate_with_timeouts(
    request: IgnoredBuildRequest<'_>,
    timeouts: IgnoredBuildTimeouts,
) -> anyhow::Result<IgnoredBuildOutcome> {
    let behavior = request.settings.ignored_build_behavior.ok_or_else(|| {
        output::coded_error(
            "IGNORED_BUILD_CONFIG_INVALID",
            "immutable build configuration does not include Ignored Build Step behavior",
        )
    })?;
    if behavior == IgnoredBuildBehavior::Automatic {
        return Ok(IgnoredBuildOutcome::Continue {
            reason: "Ignored Build Step is automatic; continuing build".to_string(),
        });
    }

    output::status(
        request.json,
        ">",
        "Running Ignored Build Step...",
        output::Phase::Deploy,
    );
    if let Some(build_logs) = request.build_logs {
        build_logs.info(BuildLogPhase::Init, "Running Ignored Build Step");
    }

    let outcome = match behavior {
        IgnoredBuildBehavior::Automatic => unreachable!(),
        IgnoredBuildBehavior::Never => IgnoredBuildOutcome::Skip {
            reason: "Ignored Build Step is configured to skip every build".to_string(),
        },
        IgnoredBuildBehavior::OnlyProduction => {
            environment_outcome(request.environment_type, "PRODUCTION")
        }
        IgnoredBuildBehavior::OnlyPreview => {
            environment_outcome(request.environment_type, "PREVIEW")
        }
        IgnoredBuildBehavior::OnlyChanges => {
            evaluate_git_diff(&request, None, timeouts.git_diff).await?
        }
        IgnoredBuildBehavior::ChangesInFolder => {
            let folder = validated_folder(request.settings.ignored_build_folder.as_deref())?;
            evaluate_git_diff(&request, Some(&folder), timeouts.git_diff).await?
        }
        IgnoredBuildBehavior::BashScript
        | IgnoredBuildBehavior::NodeScript
        | IgnoredBuildBehavior::Custom => {
            let command = request
                .settings
                .ignored_build_command
                .as_deref()
                .map(str::trim)
                .filter(|command| !command.is_empty())
                .ok_or_else(|| {
                    output::coded_error(
                        "IGNORED_BUILD_CONFIG_INVALID",
                        format!(
                            "{} requires a non-empty ignored build command",
                            behavior.as_str()
                        ),
                    )
                })?;
            let status = run_bounded_command(
                "bash",
                [OsString::from("-c"), OsString::from(command)],
                request.project_dir,
                request.execution_env,
                timeouts.user_command,
                false,
            )
            .await?;
            exit_code_outcome(status, false)?
        }
    };

    emit_outcome(&request, &outcome);
    Ok(outcome)
}

fn environment_outcome(current: &str, allowed: &str) -> IgnoredBuildOutcome {
    // CUSTOM deployments share preview runtime semantics, including
    // ONREZA_ENV=preview. Keep built-in policies aligned with that contract.
    let current_mode = if current.eq_ignore_ascii_case("CUSTOM") {
        "PREVIEW"
    } else {
        current
    };
    if current_mode.eq_ignore_ascii_case(allowed) {
        IgnoredBuildOutcome::Continue {
            reason: format!(
                "Ignored Build Step allows {} deployments; continuing build",
                allowed.to_ascii_lowercase()
            ),
        }
    } else {
        IgnoredBuildOutcome::Skip {
            reason: format!(
                "Ignored Build Step allows only {} deployments (current: {})",
                allowed.to_ascii_lowercase(),
                current_mode.to_ascii_lowercase()
            ),
        }
    }
}

fn validated_folder(folder: Option<&str>) -> anyhow::Result<String> {
    let folder = folder
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .ok_or_else(|| {
            output::coded_error(
                "IGNORED_BUILD_CONFIG_INVALID",
                "CHANGES_IN_FOLDER requires a non-empty relative folder",
            )
        })?;
    let path = Path::new(folder);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(output::coded_error(
            "IGNORED_BUILD_CONFIG_INVALID",
            "Ignored Build Step folder must stay within the project root",
        ));
    }
    let normalized = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            Component::CurDir => None,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => unreachable!(),
        })
        .collect::<Vec<_>>()
        .join("/");
    if normalized.is_empty() {
        return Err(output::coded_error(
            "IGNORED_BUILD_CONFIG_INVALID",
            "Ignored Build Step folder must name a directory within the repository",
        ));
    }
    Ok(normalized)
}

async fn evaluate_git_diff(
    request: &IgnoredBuildRequest<'_>,
    folder: Option<&str>,
    timeout: Duration,
) -> anyhow::Result<IgnoredBuildOutcome> {
    let mut args = vec![
        OsString::from("diff"),
        OsString::from("HEAD^"),
        OsString::from("HEAD"),
        OsString::from("--quiet"),
    ];
    if let Some(folder) = folder {
        args.push(OsString::from("--"));
        args.push(OsString::from(format!(":(literal){folder}")));
    }
    let status = run_bounded_command(
        "git",
        args,
        request.project_dir,
        request.execution_env,
        timeout,
        true,
    )
    .await?;
    exit_code_outcome(status, true)
}

fn exit_code_outcome(
    status: std::process::ExitStatus,
    git_diff: bool,
) -> anyhow::Result<IgnoredBuildOutcome> {
    match status.code() {
        Some(0) => Ok(IgnoredBuildOutcome::Skip {
            reason: if git_diff {
                "Ignored Build Step found no relevant changes".to_string()
            } else {
                "Ignored Build Step returned exit code 0".to_string()
            },
        }),
        Some(1) => Ok(IgnoredBuildOutcome::Continue {
            reason: if git_diff {
                "Ignored Build Step found relevant changes (exit code 1); continuing build"
                    .to_string()
            } else {
                "Ignored Build Step returned exit code 1; continuing build".to_string()
            },
        }),
        Some(128) if git_diff => Ok(IgnoredBuildOutcome::Continue {
            reason: "Ignored Build Step could not compare a parent commit (exit code 128); continuing build"
                .to_string(),
        }),
        Some(code) => Err(output::coded_error(
            "IGNORED_BUILD_STEP_FAILED",
            format!("Ignored Build Step failed with exit code {code}"),
        )),
        None => Err(output::coded_error(
            "IGNORED_BUILD_STEP_FAILED",
            "Ignored Build Step was terminated by a signal",
        )),
    }
}

async fn run_bounded_command(
    program: &str,
    args: impl IntoIterator<Item = OsString>,
    project_dir: &Path,
    execution_env: &[(String, String)],
    timeout: Duration,
    isolate_git_repository: bool,
) -> anyhow::Result<std::process::ExitStatus> {
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .current_dir(project_dir)
        .envs(
            execution_env
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    remove_private_cli_environment(command.as_std_mut());
    if isolate_git_repository {
        remove_git_repository_environment(command.as_std_mut());
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.as_std_mut().process_group(0);
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to start Ignored Build Step using {program}"))?;
    let child_id = child.id();
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(result) => result.context("failed to wait for Ignored Build Step"),
        Err(_) => {
            #[cfg(unix)]
            if let Some(child_id) = child_id {
                unsafe { libc::kill(-(child_id as libc::pid_t), libc::SIGKILL) };
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(output::coded_error(
                "IGNORED_BUILD_STEP_TIMEOUT",
                format!(
                    "Ignored Build Step exceeded its {} timeout",
                    format_timeout(timeout)
                ),
            ))
        }
    }
}

pub(super) fn remove_git_repository_environment(command: &mut std::process::Command) {
    for key in GIT_REPOSITORY_ENVIRONMENT_KEYS {
        command.env_remove(key);
    }
}

fn format_timeout(timeout: Duration) -> String {
    if timeout.subsec_nanos() == 0 {
        format!("{} second", timeout.as_secs())
    } else {
        format!("{} ms", timeout.as_millis())
    }
}

fn emit_outcome(request: &IgnoredBuildRequest<'_>, outcome: &IgnoredBuildOutcome) {
    let message = match outcome {
        IgnoredBuildOutcome::Continue { reason } | IgnoredBuildOutcome::Skip { reason } => reason,
    };
    output::success(request.json, message, output::Phase::Deploy);
    if let Some(build_logs) = request.build_logs {
        build_logs.info(BuildLogPhase::Init, message);
    }
}

#[cfg(test)]
pub(super) async fn evaluate_for_test(
    request: IgnoredBuildRequest<'_>,
    git_diff_timeout: Duration,
    user_command_timeout: Duration,
) -> anyhow::Result<IgnoredBuildOutcome> {
    evaluate_with_timeouts(
        request,
        IgnoredBuildTimeouts {
            git_diff: git_diff_timeout,
            user_command: user_command_timeout,
        },
    )
    .await
}
