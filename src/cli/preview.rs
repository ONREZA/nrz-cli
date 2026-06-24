use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct PreviewArgs {
    #[command(subcommand)]
    pub command: PreviewCommand,
}

#[derive(Subcommand)]
pub enum PreviewCommand {
    /// Create a preview protection bypass secret for agents and curl
    Access(PreviewAccessArgs),

    /// Revoke a preview protection bypass secret
    Revoke(PreviewRevokeArgs),
}

#[derive(Parser)]
pub struct PreviewAccessArgs {
    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Preview URL to include in generated curl/browser snippets
    #[arg(long)]
    pub url: Option<String>,

    /// Human-readable note for this bypass secret
    #[arg(long, default_value = "nrz preview access", value_parser = validate_note)]
    pub note: String,

    /// Access lifetime (for example: 15m, 1h, 24h)
    #[arg(long, default_value = "1h")]
    pub ttl: String,
}

#[derive(Parser)]
pub struct PreviewRevokeArgs {
    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,

    /// Bypass secret ID returned by `nrz preview access`
    #[arg(long)]
    pub secret_id: String,
}

fn validate_note(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("note must not be empty".to_string());
    }
    if value.len() > 200 {
        return Err("note must be at most 200 bytes".to_string());
    }
    Ok(value.to_string())
}
