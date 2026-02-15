mod api;
mod auth;
mod build;
mod cli;
mod deploy;
mod deployments;
mod dev;
mod link;
mod logs;
mod output;
mod projects;
mod rollback;
mod upgrade;

use std::io::IsTerminal;

use clap::Parser;
use cli::{Cli, Command};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let json = cli.json || !std::io::stdout().is_terminal();
    let token = cli.token.clone();

    let result = run_command(cli.command, json, token.as_deref()).await;

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

async fn run_command(command: Command, json: bool, token: Option<&str>) -> anyhow::Result<()> {
    match command {
        Command::Dev(args) => dev::run(args).await,
        Command::Build(args) => build::run(args, json).await,
        Command::Deploy(args) => deploy::run(args, json, token).await,
        Command::Db(args) => cli::db_handler::run(args, json).await,
        Command::Kv(args) => cli::kv_handler::run(args, json).await,
        Command::Login => auth::login(json, token).await,
        Command::Whoami => auth::whoami(json, token).await,
        Command::Logout => auth::logout(json).await,
        Command::Link(args) => link::run(args, json, token).await,
        Command::Upgrade(args) => upgrade::run(args).await,
        Command::Projects(args) => projects::run(args, json, token).await,
        Command::Deployments(args) => deployments::run(args, json, token).await,
        Command::Logs(args) => logs::run(args, json, token).await,
        Command::Env(args) => cli::env_handler::run(args, json, token).await,
        Command::Domains(args) => cli::domains_handler::run(args, json, token).await,
        Command::Rollback(args) => rollback::run(args, json, token).await,
    }
}
