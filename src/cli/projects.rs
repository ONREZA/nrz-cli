use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct ProjectsArgs {
    #[command(subcommand)]
    pub command: ProjectsCommand,
}

#[derive(Subcommand)]
pub enum ProjectsCommand {
    /// List projects
    List {
        /// Maximum number of projects to list
        #[arg(long, default_value = "20", value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
    },

    /// Create a new project
    Create {
        /// Project slug name (lowercase, hyphens)
        #[arg(long)]
        name: String,

        /// Display name (defaults to name)
        #[arg(long)]
        display_name: Option<String>,

        /// Git repository URL
        #[arg(long)]
        git_url: Option<String>,

        /// Git branch (default: main)
        #[arg(long)]
        branch: Option<String>,

        /// Framework preset (e.g. astro, nuxt, sveltekit)
        #[arg(long)]
        framework: Option<String>,

        /// Install command
        #[arg(long)]
        install_command: Option<String>,

        /// Build command
        #[arg(long)]
        build_command: Option<String>,

        /// Output directory
        #[arg(long)]
        output_directory: Option<String>,

        /// Link project to current directory after creation
        #[arg(long)]
        link: bool,
    },

    /// Show project details
    Info {
        /// Project ID
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        id: String,
    },

    /// Update project settings
    Update {
        /// Project ID
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        id: String,

        /// Display name
        #[arg(long)]
        display_name: Option<String>,

        /// Git repository URL
        #[arg(long)]
        git_url: Option<String>,

        /// Git branch
        #[arg(long)]
        branch: Option<String>,

        /// Framework preset
        #[arg(long)]
        framework: Option<String>,

        /// Install command
        #[arg(long)]
        install_command: Option<String>,

        /// Build command
        #[arg(long)]
        build_command: Option<String>,

        /// Output directory
        #[arg(long)]
        output_directory: Option<String>,

        /// Root directory
        #[arg(long)]
        root_directory: Option<String>,

        /// Node.js version
        #[arg(long)]
        node_version: Option<String>,
    },

    /// Delete a project
    Delete {
        /// Project ID
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        id: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}
