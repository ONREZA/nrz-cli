use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceCommand,
}

#[derive(Subcommand)]
pub enum WorkspaceCommand {
    /// List all workspaces
    List,

    /// Switch default workspace
    Switch {
        /// Workspace slug to switch to
        slug: String,
    },
}
