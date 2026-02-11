mod build;
mod cli;
mod deploy;
mod dev;
mod emulator;

use clap::Parser;
use cli::{Cli, Command};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Dev(args) => dev::run(args).await,
        Command::Build(args) => build::run(args).await,
        Command::Deploy(args) => deploy::run(args).await,
        Command::Db(args) => emulator::db::run(args).await,
        Command::Kv(args) => emulator::kv::run(args).await,
    }
}
