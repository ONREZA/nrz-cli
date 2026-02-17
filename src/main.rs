mod api;
mod auth;
mod build;
mod cli;
mod deploy;
mod deployments;
mod dev;
mod init;
mod link;
mod logs;
mod migrations;
mod output;
mod rollback;
mod upgrade;

use std::io::IsTerminal;

use clap::Parser;
use cli::{Cli, Command};
use nrz::config::ProjectConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let json = cli.json || !std::io::stdout().is_terminal();
    let token = cli.token.clone();
    let workspace = cli.workspace.clone();
    let env = cli.env.clone();

    let project_dir = std::env::current_dir().unwrap_or_default();
    let config = match nrz::config::load(&project_dir) {
        Ok(c) => c,
        Err(e) => {
            if json {
                output::json_error(&e);
            } else {
                eprintln!("Error: {e:#}");
            }
            std::process::exit(1);
        }
    };

    let result = run_command(
        cli.command,
        json,
        token.as_deref(),
        workspace.as_deref(),
        env.as_deref(),
        &config,
    )
    .await;

    if let Err(ref e) = result {
        if json {
            output::json_error(e);
            std::process::exit(1);
        } else {
            eprintln!("Error: {e:#}");
            std::process::exit(1);
        }
    }
}

async fn run_command(
    command: Command,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    match command {
        Command::Dev(args) => dev::run(args, config).await,
        Command::Build(args) => build::run(args, json, config).await,
        Command::Deploy(args) => deploy::run(args, json, token, workspace, config).await,
        Command::Db(args) => cli::db_handler::run(args, json, token, workspace, env, config).await,
        Command::Kv(args) => cli::kv_handler::run(args, json).await,
        Command::Login => auth::login(json, token).await,
        Command::Whoami => auth::whoami(json, token, workspace).await,
        Command::Logout(args) => auth::logout(json, workspace, args.all).await,
        Command::Link(args) => link::run(args, json, token, workspace, config).await,
        Command::Upgrade(args) => upgrade::run(args).await,
        Command::Projects(args) => cli::projects_handler::run(args, json, token, workspace).await,
        Command::Deployments(args) => deployments::run(args, json, token, workspace, config).await,
        Command::Logs(args) => logs::run(args, json, token, workspace, config).await,
        Command::Env(args) => cli::env_handler::run(args, json, token, workspace, config).await,
        Command::Domains(args) => {
            cli::domains_handler::run(args, json, token, workspace, config).await
        }
        Command::Rollback(args) => rollback::run(args, json, token, workspace, config).await,
        Command::Workspace(args) => cli::workspace_handler::run(args, json).await,
        Command::Init(args) => init::run(args, json, token, workspace, config).await,
    }
}
