//! CLI args for `nrz detect`.

use clap::Parser;

#[derive(Parser)]
pub struct DetectArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Output only the framework slug
    #[arg(long)]
    pub slug_only: bool,

    /// Save detected framework to onreza.toml
    #[arg(long)]
    pub save: bool,

    /// Read JSON manifest from stdin (remote detection)
    #[arg(long, hide = true)]
    pub stdin: bool,

    /// Output the list of files needed for remote detection
    #[arg(long, hide = true)]
    pub needed_files: bool,
}
