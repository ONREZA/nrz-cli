use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use uuid::Uuid;

use crate::cli::DeployArgs;

#[path = "edge_handoff_v2.rs"]
mod v2;

#[cfg(test)]
#[path = "legacy_handoff_test_support.rs"]
mod legacy_test_support;

pub(super) const EDGE_BUILD_HANDOFF_MODE_ENV: &str = "NRZ_EDGE_BUILD_HANDOFF";
#[cfg(test)]
pub(super) const EDGE_BUILD_HANDOFF_MODE_V1: &str = "V1";
pub(super) const EDGE_BUILD_HANDOFF_MODE_V2: &str = "V2";
const OUTPUT_DIRECTORY_ENV: &str = "ONREZA_OUTPUT_DIR";
const PLATFORM_RUNNER_ENV: &str = "NRZ_RUNNER";
const PLATFORM_RUNNER_VALUE: &str = "PLATFORM";

pub(super) struct EdgeBuildHandoffOutput {
    root: PathBuf,
    mode: &'static str,
    binding: Option<nrz_source_bundle::EdgeBuildBindingV2>,
}

pub(super) fn validate_resume_arguments(args: &DeployArgs) -> anyhow::Result<()> {
    if args.resume_deployment.is_none() {
        return Ok(());
    }

    let overrides = [
        (args.prod, "--prod"),
        (args.dry, "--dry"),
        (args.verify, "--verify"),
        (args.environment.is_some(), "--environment"),
        (args.project_id.is_some(), "--project-id"),
        (args.skip_build, "--skip-build"),
        (args.skip_install, "--skip-install"),
        (args.no_log_upload, "--no-log-upload"),
        (args.log_upload_debug, "--log-upload-debug"),
        (args.build_command.is_some(), "--build-command"),
        (args.skip_env_check, "--skip-env-check"),
        (args.compute.is_some(), "--compute"),
        (args.health_check_path.is_some(), "--health-check-path"),
        (args.app.is_some(), "--app"),
        (args.force_rules, "--force-rules"),
    ]
    .into_iter()
    .filter_map(|(present, flag)| present.then_some(flag))
    .collect::<Vec<_>>();

    if overrides.is_empty() {
        return Ok(());
    }
    Err(crate::output::coded_error(
        "INVALID_ARGUMENT",
        format!(
            "--resume-deployment uses immutable deployment-scoped settings and forbids overrides: {}",
            overrides.join(", ")
        ),
    ))
}

impl EdgeBuildHandoffOutput {
    pub(super) fn from_process_environment(
        resume_deployment_id: Option<Uuid>,
    ) -> anyhow::Result<Option<Self>> {
        let mode = std::env::var(EDGE_BUILD_HANDOFF_MODE_ENV).ok();
        if mode.as_deref() == Some("V1") {
            return Err(crate::output::coded_error(
                "CLI_UPDATE_REQUIRED",
                "Source V1 build handoff is no longer supported; use NRZ_EDGE_BUILD_HANDOFF=V2",
            ));
        }
        let output = std::env::var_os(OUTPUT_DIRECTORY_ENV).map(PathBuf::from);
        let platform_runner =
            std::env::var(PLATFORM_RUNNER_ENV).is_ok_and(|value| value == PLATFORM_RUNNER_VALUE);
        let mut output = Self::from_values(
            mode.as_deref(),
            output.as_deref(),
            platform_runner,
            resume_deployment_id,
        )?;
        if let Some(output) = &mut output
            && output.is_v2()
        {
            output.binding = Some(v2::binding_from_environment(
                resume_deployment_id.context("V2 handoff requires deployment")?,
            )?);
        }
        Ok(output)
    }

    pub(super) fn from_values(
        mode: Option<&str>,
        output: Option<&Path>,
        platform_runner: bool,
        resume_deployment_id: Option<Uuid>,
    ) -> anyhow::Result<Option<Self>> {
        let Some(mode) = mode else {
            ensure!(
                resume_deployment_id.is_none(),
                "--resume-deployment requires {EDGE_BUILD_HANDOFF_MODE_ENV}={EDGE_BUILD_HANDOFF_MODE_V2}"
            );
            return Ok(None);
        };
        if mode != "V1" && mode != EDGE_BUILD_HANDOFF_MODE_V2 {
            bail!("unsupported {EDGE_BUILD_HANDOFF_MODE_ENV} mode");
        }
        ensure!(
            platform_runner,
            "{EDGE_BUILD_HANDOFF_MODE_ENV} requires {PLATFORM_RUNNER_ENV}={PLATFORM_RUNNER_VALUE}"
        );
        ensure!(
            resume_deployment_id.is_some(),
            "{EDGE_BUILD_HANDOFF_MODE_ENV} requires --resume-deployment"
        );
        let output = output.context("Edge build handoff requires ONREZA_OUTPUT_DIR")?;
        ensure!(
            output.is_absolute(),
            "Edge build handoff output must be an absolute path"
        );
        Ok(Some(Self {
            root: output.to_path_buf(),
            mode: if mode == EDGE_BUILD_HANDOFF_MODE_V2 {
                EDGE_BUILD_HANDOFF_MODE_V2
            } else {
                "V1"
            },
            binding: None,
        }))
    }
}

fn publish_new_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt as _;
        let source = std::ffi::CString::new(source.as_os_str().as_bytes())?;
        let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())?;
        // Bundled musl lacks the renameat2 wrapper; NOREPLACE still atomically claims the name.
        if unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        } == 0
        {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        fs::hard_link(source, destination)
    }
}

fn write_new_synced_file(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let mut file = new_file(path)?;
    file.write_all(contents)
        .context("failed to write Edge build handoff descriptor")?;
    file.sync_all()
        .context("failed to sync Edge build handoff descriptor")
}

fn new_file(path: &Path) -> anyhow::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))
}

fn require_absent(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => bail!(
            "Edge build handoff output already exists: {}",
            path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect handoff output {}", path.display())),
    }
}

fn sync_directory(path: &Path) -> anyhow::Result<()> {
    File::open(path)
        .with_context(|| format!("failed to open handoff directory {}", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync handoff directory {}", path.display()))
}
