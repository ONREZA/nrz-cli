use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct FunctionsArgs {
    #[command(subcommand)]
    pub command: FunctionsCommand,
}

#[derive(Subcommand)]
pub enum FunctionsCommand {
    /// Run the ONREZA Functions policy check against the local source bundle
    Check(FunctionsCheckArgs),
}

#[derive(Parser)]
pub struct FunctionsCheckArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,
}
