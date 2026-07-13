pub mod config;
pub mod config_handler;
pub mod db;
pub mod db_handler;
pub mod detect;
pub mod detect_handler;
pub mod domains;
pub mod domains_handler;
#[cfg(test)]
mod domains_handler_tests;
pub mod env;
pub mod env_handler;
#[cfg(test)]
mod env_handler_tests;
pub mod environment;
#[cfg(test)]
mod environment_tests;
pub mod functions;
pub mod functions_handler;
#[cfg(test)]
mod functions_handler_tests;
pub mod kv;
pub mod kv_handler;
pub mod preview;
pub mod projects;
pub mod projects_handler;
#[cfg(test)]
mod projects_handler_tests;
pub mod rules;
pub mod rules_handler;
#[cfg(test)]
mod rules_handler_tests;
pub mod workspace;
pub mod workspace_handler;

pub use config::ConfigArgs;
pub use db::DbArgs;
pub use detect::DetectArgs;
pub use domains::DomainsArgs;
pub use env::EnvArgs;
pub use functions::FunctionsArgs;
pub use kv::KvArgs;
pub use preview::PreviewArgs;
pub use projects::ProjectsArgs;
pub use rules::RulesArgs;
pub use workspace::WorkspaceArgs;

use clap::{ArgAction, Parser, Subcommand, builder::BoolishValueParser};

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
    #[arg(
        long,
        global = true,
        env = "NRZ_JSON",
        action = ArgAction::SetTrue,
        value_parser = BoolishValueParser::new()
    )]
    pub json: bool,

    /// Force human-readable output; suppresses --json/NRZ_JSON and auto-JSON in non-TTY environments
    #[arg(
        long,
        global = true,
        env = "NRZ_HUMAN",
        action = ArgAction::SetTrue,
        value_parser = BoolishValueParser::new()
    )]
    pub human: bool,

    /// API token for authentication
    #[arg(long, global = true, env = "NRZ_TOKEN")]
    pub token: Option<String>,

    /// Workspace slug to use
    #[arg(long, global = true, env = "NRZ_WORKSPACE")]
    pub workspace: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start local dev server with platform emulation (KV, Context)
    Dev(DevArgs),

    /// Validate build output and manifest
    Build(BuildArgs),

    /// Deploy to ONREZA platform
    Deploy(DeployArgs),

    /// Manage managed PostgreSQL databases
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

    /// Manage protected preview URL access
    Preview(PreviewArgs),

    /// Inspect effective CLI configuration
    Config(ConfigArgs),

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

    /// Initialize project scaffold (optionally create/link on platform)
    Init(InitArgs),

    /// Detect framework, package manager, and project features
    Detect(DetectArgs),

    /// Manage ONREZA Functions (policy check, ...)
    Functions(FunctionsArgs),

    /// Manage Edge Rules authored in onreza.rules.toml
    Rules(RulesArgs),
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
    /// Run a named command profile from [dev.aliases] in onreza.toml
    #[arg(short = 'a', long, conflicts_with = "command")]
    pub alias: Option<String>,

    /// Framework command to run (overrides [dev] command in onreza.toml)
    #[arg(long)]
    pub command: Option<String>,

    /// Port for the dev server (default: 4321, or from onreza.toml)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Enable Node.js inspector (attach a debugger on port 9229)
    #[arg(long)]
    pub inspect: bool,

    /// Enable Node.js inspector, breaking before user code starts
    #[arg(long, conflicts_with = "inspect")]
    pub inspect_brk: bool,

    /// Database branch to use for DATABASE_URL injection
    #[arg(long)]
    pub db_branch: Option<String>,

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

    /// Build and validate deploy plan without creating a deployment
    #[arg(long)]
    pub dry: bool,

    /// Verify the live deployment URL after deploy; preview deployments use a temporary bypass
    #[arg(long, conflicts_with_all = ["dry", "resume_deployment"])]
    pub verify: bool,

    /// Environment type (production or preview). Can be specified multiple times.
    #[arg(long, env = "NRZ_ENV")]
    pub env: Vec<String>,

    /// Project ID (skip interactive selection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Skip build step before deploying
    #[arg(long)]
    pub skip_build: bool,

    /// Skip dependency installation before build
    #[arg(long)]
    pub skip_install: bool,

    /// Disable centralized build-log upload for this deployment
    #[arg(long)]
    pub no_log_upload: bool,

    /// Include debug build-log events in centralized upload
    #[arg(long)]
    pub log_upload_debug: bool,

    /// Custom build command (overrides [build] command in onreza.toml)
    #[arg(long)]
    pub build_command: Option<String>,

    /// Skip environment variable validation against [env] declarations
    #[arg(long)]
    pub skip_env_check: bool,

    /// Resume an existing deployment (builder mode: skip project resolution and polling)
    #[arg(long, hide = true)]
    pub resume_deployment: Option<String>,

    /// Override compute type: static, process
    #[arg(long)]
    pub compute: Option<String>,

    /// Health check path for PROCESS deployments (e.g. "/health"). Use "none" for TCP only.
    #[arg(long)]
    pub health_check_path: Option<String>,

    /// Monorepo app/workspace to deploy (name, directory basename, or path)
    #[arg(long, alias = "filter")]
    pub app: Option<String>,

    /// Replace UI-authored Edge Rules when onreza.rules.toml intentionally owns them
    #[arg(long)]
    pub force_rules: bool,
}

#[derive(Parser)]
pub struct InitArgs {
    /// Project name (default: directory name)
    #[arg(long)]
    pub name: Option<String>,

    /// Skip framework/package manager detection
    #[arg(long)]
    pub skip_detection: bool,

    /// Create project on platform
    #[arg(long)]
    pub create: bool,

    /// Link existing project by ID
    #[arg(long)]
    pub project_id: Option<String>,

    /// Create only local onreza.toml/.onreza scaffold; never create or link a platform project
    #[arg(long, conflicts_with_all = ["create", "project_id"])]
    pub local: bool,
}
