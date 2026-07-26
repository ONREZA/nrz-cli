pub mod inject;
mod process;

#[cfg(test)]
mod inject_tests;

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use serde::Serialize;

use crate::api::ApiClient;
use crate::auth;
use crate::cli::DevArgs;
use crate::output;
use nrz::config::{self, ProjectConfig};
use nrz::emulator::kv::KvStore;
use nrz::emulator::server::EmulatorServer;

/// Start local dev server with platform emulation.
///
/// 1. Resolve dev command (--alias > --command > config)
/// 2. Ensure data directory exists
/// 3. Generate JS bootstrap (globalThis.ONREZA)
/// 4. Fetch DATABASE_URL from kaiki (if project is linked and user is authenticated)
/// 5. Create KV store + emulator server
/// 6. Start emulator in background, wait for readiness
/// 7. Build NODE_OPTIONS (bootstrap + optional inspector)
/// 8. Spawn dev command as child process with injected env vars
/// 9. Cleanup on exit
pub async fn run(
    args: DevArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = std::path::Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    // 1. Resolve dev command: --alias > --command > [dev] command
    let dev_command = if let Some(ref name) = args.alias {
        config
            .dev_alias_command(name)
            .with_context(|| {
                format!("alias '{name}' not found — define it in [dev.aliases] in onreza.toml")
            })?
            .to_string()
    } else if let Some(ref cmd) = args.command {
        cmd.clone()
    } else if let Some(ref cmd) = config.dev.command {
        cmd.clone()
    } else {
        anyhow::bail!(
            "no dev command specified — set [dev] command in onreza.toml, use --command, or define aliases in [dev.aliases]"
        );
    };

    // 2. Ensure data directory
    prepare_data_dir(&project_dir, config.data_dir_relative())?;

    // 3. Generate bootstrap script
    let port = args.port.unwrap_or(config.dev_port());
    let emulator_port = port
        .checked_add(1)
        .context("dev port 65535 leaves no port available for the local emulator")?;

    // 4. Resolve one platform execution context unless explicitly local-only.
    let extra_env = if args.local {
        std::collections::HashMap::new()
    } else {
        let project_id = config::resolve_project_id(None, config)?;
        let token = auth::resolve_token(token, workspace.or(config.project.workspace.as_deref()))?;
        let client = ApiClient::authenticated(&token)?;
        let source_ref = args.db_branch.as_deref().or(config.db_branch());
        let context = crate::execution_context::resolve_for_mutation(
            &client,
            &project_id,
            &project_dir,
            args.environment.as_deref(),
            source_ref,
        )
        .await?;
        let materialized =
            crate::execution_context::materialize_desired(&client, &context, source_ref, "DEV")
                .await?;
        crate::execution_context::warn_local_dotenv_drift(&project_dir, false)?;
        output::status(
            json,
            "~",
            format!(
                "environment {} ({})",
                materialized.context.environment_name, materialized.context.environment_id,
            ),
            output::Phase::Dev,
        );
        crate::execution_context::execution_environment(&materialized)
    };

    // 5. Create the authenticated bootstrap immediately before starting the emulator.
    let kv = KvStore::new();
    let host = config.dev_host();
    let server = EmulatorServer::new(kv, emulator_port, host)?;
    let emulator_token = server.token().to_string();
    let emulator_url = server.client_url();
    let bootstrap = inject::generate_bootstrap(&emulator_url, &emulator_token)?;
    let bootstrap_file = write_bootstrap(&bootstrap)?;

    // 6. Start emulator server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            tracing::error!(%e, "emulator server error");
        }
    });

    // 7. Wait for emulator to be ready
    if let Err(error) = wait_for_emulator(&emulator_url, &emulator_token).await {
        server_handle.abort();
        return Err(error);
    }

    output::status(
        json,
        "~",
        format!("emulator ready on port {emulator_port}"),
        output::Phase::Dev,
    );

    // 8. Build extra NODE_OPTIONS (--inspect / --inspect-brk)
    let inspect_flag = if args.inspect_brk {
        Some("--inspect-brk")
    } else if args.inspect {
        Some("--inspect")
    } else {
        None
    };

    output::status(
        json,
        ">",
        format!("starting: {dev_command}"),
        output::Phase::Dev,
    );

    // 9. Spawn dev server (blocks until exit or Ctrl+C)
    let exit = process::spawn_dev_server(
        &project_dir,
        &dev_command,
        bootstrap_file.path(),
        inspect_flag,
        &extra_env,
        json,
    )
    .await?;

    // 10. Cleanup
    server_handle.abort();

    if json {
        output::json_output(&DevResult {
            status: exit.as_str(),
            command: &dev_command,
            project_dir: project_dir.to_string_lossy(),
        });
    }

    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DevResult<'a> {
    status: &'static str,
    command: &'a str,
    project_dir: std::borrow::Cow<'a, str>,
}

struct BootstrapFile {
    directory: PathBuf,
    path: PathBuf,
}

impl BootstrapFile {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for BootstrapFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.directory);
    }
}

fn write_bootstrap(bootstrap: &str) -> anyhow::Result<BootstrapFile> {
    let directory = std::env::temp_dir().join(format!("nrz-dev-{}", uuid::Uuid::now_v7().simple()));
    let mut directory_options = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory_options.mode(0o700);
    }
    directory_options.create(&directory).with_context(|| {
        format!(
            "failed to create bootstrap directory: {}",
            directory.display()
        )
    })?;
    let path = directory.join("bootstrap.mjs");
    let write_result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .with_context(|| format!("failed to create bootstrap: {}", path.display()))?;
        file.write_all(bootstrap.as_bytes())
            .with_context(|| format!("failed to write bootstrap: {}", path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync bootstrap: {}", path.display()))
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&directory);
    }
    write_result?;
    Ok(BootstrapFile { directory, path })
}

fn prepare_data_dir(project_dir: &Path, configured: &str) -> anyhow::Result<PathBuf> {
    let relative = Path::new(configured);
    if configured.trim().is_empty() || relative.as_os_str() == "." {
        anyhow::bail!("dev.data_dir must name a directory inside the project");
    }

    let mut target = project_dir.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => target.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!(
                    "dev.data_dir must be a relative path inside the project: {configured}"
                );
            }
        }

        match std::fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                anyhow::bail!(
                    "dev.data_dir must not traverse symbolic links: {}",
                    target.display()
                );
            }
            Ok(metadata) if !metadata.is_dir() => {
                anyhow::bail!(
                    "dev.data_dir component is not a directory: {}",
                    target.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&target).with_context(|| {
                    format!("failed to create data directory: {}", target.display())
                })?;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to inspect data directory: {}", target.display())
                });
            }
        }
    }

    if target == project_dir {
        anyhow::bail!("dev.data_dir must name a directory inside the project");
    }

    let canonical = target
        .canonicalize()
        .with_context(|| format!("failed to resolve data directory: {}", target.display()))?;
    if !canonical.starts_with(project_dir) {
        anyhow::bail!(
            "dev.data_dir resolves outside the project: {}",
            canonical.display()
        );
    }

    Ok(canonical)
}

async fn wait_for_emulator(base_url: &str, token: &str) -> anyhow::Result<()> {
    let url = format!("{base_url}/__nrz/health");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        match client
            .get(&url)
            .header(nrz::emulator::server::EMULATOR_TOKEN_HEADER, token)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    }
    anyhow::bail!("emulator server failed to start at {base_url}")
}
