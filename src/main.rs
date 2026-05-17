mod api;
mod auth;
mod build;
mod cli;
mod deploy;
mod deployments;
#[cfg(test)]
mod deployments_tests;
mod detect;
mod detect_sync;
mod dev;
mod init;
mod link;
mod logs;
mod output;
#[cfg(test)]
mod output_tests;
mod rollback;
mod upgrade;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::Parser;
use cli::{Cli, Command};
use nrz::config::ProjectConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let json = !cli.human && (cli.json || !std::io::stdout().is_terminal());
    let token = cli.token.clone();
    let workspace = cli.workspace.clone();
    let env = cli.env;

    let config_dir = config_dir_for_command(&cli.command);
    let config = match nrz::config::load(&config_dir) {
        Ok(c) => c,
        Err(e) => {
            let coded = output::coded_error("INVALID_CONFIG", format!("{e:#}"));
            emit_terminal_error(json, &coded);
            std::process::exit(1);
        }
    };

    let result = run_command(
        cli.command,
        json,
        token.as_deref(),
        workspace.as_deref(),
        &env,
        &config,
    )
    .await;

    if let Err(ref e) = result {
        emit_terminal_error(json, e);
        std::process::exit(1);
    }
}

fn config_dir_for_command(command: &Command) -> PathBuf {
    match command {
        Command::Dev(args) => Path::new(&args.dir).to_path_buf(),
        Command::Build(args) => Path::new(&args.dir).to_path_buf(),
        Command::Deploy(args) => Path::new(&args.dir).to_path_buf(),
        Command::Link(args) => Path::new(&args.dir).to_path_buf(),
        Command::Detect(args) => Path::new(&args.dir).to_path_buf(),
        _ => std::env::current_dir().unwrap_or_default(),
    }
}

fn emit_terminal_error(json: bool, err: &anyhow::Error) {
    if !json {
        eprintln!("Error: {err:#}");
        return;
    }
    let message = format!("{err:#}");
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<output::CodedError>());
    match coded {
        Some(c) => output::log_error_structured("error", &message, &c.code, None),
        None => output::log_line("user", "error", "error", &message),
    }
}

async fn run_command(
    command: Command,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: &[String],
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    match command {
        Command::Dev(args) => dev::run(args, config).await,
        Command::Build(args) => build::run(args, json, config).await.map(|_| ()),
        Command::Deploy(args) => deploy::run(args, json, token, workspace, config).await,
        Command::Db(args) => cli::db_handler::run(args, json, token, workspace, config).await,
        Command::Kv(args) => cli::kv_handler::run(args, json).await,
        Command::Login => auth::login(json, token).await,
        Command::Whoami => auth::whoami(json, token, workspace).await,
        Command::Logout(args) => auth::logout(json, workspace, args.all).await,
        Command::Link(args) => link::run(args, json, token, workspace, config).await,
        Command::Upgrade(args) => upgrade::run(args).await,
        Command::Projects(args) => cli::projects_handler::run(args, json, token, workspace).await,
        Command::Deployments(args) => deployments::run(args, json, token, workspace, config).await,
        Command::Logs(args) => logs::run(args, json, token, workspace, config).await,
        Command::Env(args) => {
            cli::env_handler::run(args, json, token, workspace, env, config).await
        }
        Command::Domains(args) => {
            cli::domains_handler::run(args, json, token, workspace, config).await
        }
        Command::Rollback(args) => rollback::run(args, json, token, workspace, config).await,
        Command::Workspace(args) => cli::workspace_handler::run(args, json).await,
        Command::Init(args) => init::run(args, json, token, workspace, config).await,
        Command::Detect(args) => Ok(cli::detect_handler::run(args, json)?),
    }
}
