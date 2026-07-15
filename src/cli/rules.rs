use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct RulesArgs {
    #[command(subcommand)]
    pub command: RulesCommand,
}

#[derive(Subcommand)]
pub enum RulesCommand {
    /// Pull active Edge Rules into onreza.rules.toml
    Pull(RulesPullArgs),

    /// Publish onreza.rules.toml without running a deployment
    Publish(RulesPublishArgs),

    /// Show active Edge Rules metadata for an environment
    Status(RulesStatusArgs),

    /// Validate local onreza.rules.toml
    Check(RulesCheckArgs),
}

#[derive(Parser)]
pub struct RulesPullArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Target environment (production, preview, development, custom name, or environment ID)
    #[arg(long)]
    pub environment: Option<String>,

    /// Overwrite onreza.rules.toml without prompting
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser)]
pub struct RulesPublishArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Target environment (production, preview, development, custom name, or environment ID)
    #[arg(long)]
    pub environment: Option<String>,

    /// Replace UI-authored Edge Rules when onreza.rules.toml intentionally owns them
    #[arg(long)]
    pub force_rules: bool,
}

#[derive(Parser)]
pub struct RulesStatusArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Target environment (production, preview, development, custom name, or environment ID)
    #[arg(long)]
    pub environment: Option<String>,
}

#[derive(Parser)]
pub struct RulesCheckArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,
}
