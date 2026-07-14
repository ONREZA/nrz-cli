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

    /// Invoke an active ONREZA Function revision in the remote sandbox
    Invoke(Box<FunctionsInvokeArgs>),
}

#[derive(Parser)]
pub struct FunctionsCheckArgs {
    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,
}

#[derive(Parser)]
pub struct FunctionsInvokeArgs {
    /// Function name
    pub name: String,

    /// Path to project directory
    #[arg(default_value = ".")]
    pub dir: String,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Target environment (production, preview, development, custom name, or environment ID)
    #[arg(long)]
    pub environment: Option<String>,

    /// HTTP method for fetch-style invokes
    #[arg(long, value_parser = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"])]
    pub method: Option<String>,

    /// HTTP path for fetch-style invokes
    #[arg(long)]
    pub path: Option<String>,

    /// Raw query string, with or without leading '?'
    #[arg(long)]
    pub query_string: Option<String>,

    /// Host used to build the synthetic request URL
    #[arg(long)]
    pub host: Option<String>,

    /// HTTP header as 'Name: value'; repeat for multiple headers
    #[arg(long = "header", short = 'H')]
    pub headers: Vec<String>,

    /// JSON payload file to POST as application/json; use '-' to read stdin
    #[arg(long, conflicts_with_all = ["body", "body_base64"])]
    pub payload: Option<String>,

    /// Raw request body file; use '-' to read stdin
    #[arg(long, conflicts_with_all = ["payload", "body_base64"])]
    pub body: Option<String>,

    /// Raw request body as standard base64
    #[arg(long, conflicts_with_all = ["payload", "body"])]
    pub body_base64: Option<String>,

    /// Non-fetch event JSON file ({ "type": "manual|queue|scheduled", "event": ... }); use '-' to read stdin
    #[arg(long)]
    pub event: Option<String>,

    /// Debug options JSON file; use '-' to read stdin
    #[arg(long)]
    pub debug: Option<String>,
}
