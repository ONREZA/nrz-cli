use clap::{Args, Parser, Subcommand};

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

        /// Git branch (defaults to the repository primary branch)
        #[arg(long)]
        branch: Option<String>,

        /// Framework preset (e.g. astro, nuxt, sveltekit)
        #[arg(long)]
        framework: Option<String>,

        #[command(flatten)]
        build: ProjectBuildOptions,

        /// Link project to current directory after creation
        #[arg(long)]
        link: bool,
    },

    /// Show project details
    Info {
        /// Select one Service in a multi-Service Product
        #[arg(long)]
        service_id: Option<uuid::Uuid>,
        /// Project ID
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        id: String,
    },

    /// Update project settings
    Update {
        /// Select one Service in a multi-Service Product
        #[arg(long)]
        service_id: Option<uuid::Uuid>,
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

        #[command(flatten)]
        build: ProjectBuildOptions,

        /// Root directory
        #[arg(long)]
        root_directory: Option<String>,

        /// Node.js version
        #[arg(long)]
        node_version: Option<String>,
    },

    /// Delete an empty Product (remove Services, databases and KV data first)
    Delete {
        /// Project ID
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        id: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
pub struct ProjectBuildOptions {
    /// Package manager compatible with the selected framework
    #[arg(long, value_parser = ["npm", "yarn", "pnpm", "bun", "pip", "uv", "poetry", "pub", "go"])]
    pub package_manager: Option<String>,

    /// Install command
    #[arg(long)]
    pub install_command: Option<String>,

    /// Build command
    #[arg(long)]
    pub build_command: Option<String>,

    /// Output directory
    #[arg(long)]
    pub output_directory: Option<String>,
}
