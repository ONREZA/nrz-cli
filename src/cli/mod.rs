pub mod db;
pub mod db_handler;
#[cfg(test)]
mod db_handler_tests;
pub mod db_migrate_handler;
#[cfg(test)]
mod db_migrate_handler_tests;
pub mod domains;
pub mod domains_handler;
pub mod env;
pub mod env_handler;
pub mod kv;
pub mod kv_handler;
pub mod projects;
pub mod projects_handler;
pub mod workspace;
pub mod workspace_handler;

pub use db::DbArgs;
pub use domains::DomainsArgs;
pub use env::EnvArgs;
pub use kv::KvArgs;
pub use projects::ProjectsArgs;
pub use workspace::WorkspaceArgs;

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

    /// Machine-readable JSON output
    #[arg(long, global = true, env = "NRZ_JSON")]
    pub json: bool,

    /// API token for authentication
    #[arg(long, global = true, env = "NRZ_TOKEN")]
    pub token: Option<String>,

    /// Workspace slug to use
    #[arg(long, global = true, env = "NRZ_WORKSPACE")]
    pub workspace: Option<String>,
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

    /// Log out from ONREZA platform
    Logout(LogoutArgs),

    /// Link this directory to an ONREZA project
    Link(LinkArgs),

    /// Upgrade nrz to the latest version
    Upgrade(crate::upgrade::UpgradeArgs),

    /// Manage projects
    Projects(ProjectsArgs),

    /// List deployments for a project
    Deployments(DeploymentsArgs),

    /// View runtime logs
    Logs(LogsArgs),

    /// Manage environment variables
    Env(EnvArgs),

    /// Manage custom domains
    Domains(DomainsArgs),

    /// Rollback a deployment
    Rollback(RollbackArgs),

    /// Manage workspaces
    Workspace(WorkspaceArgs),

    /// Initialize an existing project on ONREZA platform
    Init(InitArgs),
}

#[derive(Parser)]
pub struct LogoutArgs {
    /// Log out from all workspaces
    #[arg(long)]
    pub all: bool,
}

#[derive(Parser)]
pub struct DeploymentsArgs {
    /// Maximum number of deployments to list
    #[arg(long, default_value = "10", value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: u32,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Parser)]
pub struct LogsArgs {
    /// Filter by deployment ID
    #[arg(long)]
    pub deployment_id: Option<String>,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Maximum number of log entries
    #[arg(long, default_value = "50", value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,

    /// Search filter
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Parser)]
pub struct RollbackArgs {
    /// Deployment ID to rollback (default: current live)
    #[arg(long)]
    pub deployment_id: Option<String>,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Parser)]
pub struct LinkArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Project ID (skip interactive selection)
    #[arg(long)]
    pub project_id: Option<String>,
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

    /// Production deployment
    #[arg(long)]
    pub prod: bool,

    /// Project ID (skip interactive selection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Skip database migrations during deploy
    #[arg(long)]
    pub skip_migrations: bool,
}

#[derive(Parser)]
pub struct InitArgs {
    /// Project name (default: directory name)
    #[arg(long)]
    pub name: Option<String>,

    /// Skip framework/package manager detection
    #[arg(long)]
    pub skip_detection: bool,
}
