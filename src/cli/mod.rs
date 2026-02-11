pub mod db;
pub mod db_handler;
pub mod kv;
pub mod kv_handler;

pub use db::DbArgs;
pub use kv::KvArgs;

use clap::{Parser, Subcommand};

/// ONREZA platform CLI
#[derive(Parser)]
#[command(
    name = "nrz",
    version,
    about = "ONREZA platform CLI — dev, build, deploy"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start local dev server with platform emulation (KV, DB, Context)
    Dev(DevArgs),

    /// Validate build output and manifest
    Build(BuildArgs),

    /// Deploy to ONREZA platform
    Deploy(DeployArgs),

    /// Manage local D1-compatible SQLite database
    Db(DbArgs),

    /// Manage local KV store
    Kv(KvArgs),

    /// Log in to ONREZA platform
    Login,

    /// Show current user info
    Whoami,

    /// Upgrade nrz to the latest version
    Upgrade(crate::upgrade::UpgradeArgs),
}

#[derive(Parser)]
pub struct DevArgs {
    /// Framework command to run (default: auto-detect)
    #[arg(long)]
    pub command: Option<String>,

    /// Port for the dev server
    #[arg(short, long, default_value = "4321")]
    pub port: u16,

    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,
}

#[derive(Parser)]
pub struct BuildArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Skip manifest validation
    #[arg(long)]
    pub skip_validation: bool,
}

#[derive(Parser)]
pub struct DeployArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Deploy token (or NRZ_TOKEN env var)
    #[arg(long, env = "NRZ_TOKEN")]
    pub token: Option<String>,

    /// Production deployment
    #[arg(long)]
    pub prod: bool,
}
