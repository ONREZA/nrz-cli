use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Explain the effective build/deploy configuration
    Explain(ConfigExplainArgs),
}

#[derive(Parser)]
pub struct ConfigExplainArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Monorepo app/workspace to explain (name, directory basename, or path)
    #[arg(long, alias = "filter")]
    pub app: Option<String>,

    /// Project ID to use for server-backed settings
    #[arg(long)]
    pub project_id: Option<String>,

    /// Explain local onreza.toml config only; do not fetch server project settings
    #[arg(long)]
    pub local: bool,
}
