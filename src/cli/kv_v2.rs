use clap::{Args, Subcommand};
use uuid::Uuid;

#[derive(Args)]
pub struct KvV2Args {
    #[command(subcommand)]
    pub command: KvV2Command,
}

#[derive(Subcommand)]
pub enum KvV2Command {
    /// List regions eligible for a new store or a store's empty branches
    Regions {
        #[arg(long)]
        store: Option<Uuid>,
    },
    /// Manage remote KV Stores
    Stores {
        #[command(subcommand)]
        command: StoresCommand,
    },
    /// Manage empty and copy-on-write branches
    Branches {
        /// Required Store ID (accepted before or after the subcommand)
        #[arg(long, global = true)]
        store: Option<Uuid>,
        #[command(subcommand)]
        command: BranchesCommand,
    },
    /// Read and write binary data (UTF-8 input by default; base64 JSON output)
    Data {
        /// Required Store ID
        #[arg(long, global = true)]
        store: Option<Uuid>,
        /// Required branch ID
        #[arg(long, global = true)]
        branch: Option<Uuid>,
        #[command(subcommand)]
        command: DataCommand,
    },
}

#[derive(Args)]
pub struct Page {
    #[arg(long)]
    pub after: Option<Uuid>,
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: u32,
}

#[derive(Args)]
pub struct NewBranch {
    pub name: String,
    #[arg(long)]
    pub region: String,
    /// Stable creation identity; reuse with the identical request after an unknown outcome
    #[arg(long)]
    pub id: Option<Uuid>,
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(i64).range(1..=3_600_000))]
    pub batch_window_ms: i64,
}

#[derive(Subcommand)]
pub enum StoresCommand {
    List(Page),
    Create {
        #[command(flatten)]
        configuration: NewBranch,
        #[arg(long)]
        main_branch_id: Option<Uuid>,
    },
    Info {
        id: Uuid,
    },
    Delete {
        id: Uuid,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum BranchesCommand {
    List(Page),
    /// Create an empty branch in an eligible region
    Create(NewBranch),
    Info {
        id: Uuid,
    },
    Delete {
        id: Uuid,
        #[arg(long)]
        force: bool,
    },
    /// Fork a source branch; PENDING is accepted, inspect readiness with info
    Fork {
        source: Uuid,
        name: String,
        #[arg(long)]
        id: Option<Uuid>,
    },
    /// Change batching; the response includes configurationEffectiveAt
    BatchWindow {
        id: Uuid,
        #[arg(value_parser = clap::value_parser!(i64).range(1..=3_600_000))]
        milliseconds: i64,
    },
}

#[derive(Args)]
pub struct Key {
    pub key: String,
    /// Inputs are canonical padded base64 instead of UTF-8
    #[arg(long)]
    pub base64: bool,
}

#[derive(Subcommand)]
pub enum DataCommand {
    Get(Key),
    /// Acknowledge buffered writes by default (100ms default branch window)
    Put {
        #[command(flatten)]
        key: Key,
        value: String,
        /// Acknowledge only after the write is durable
        #[arg(long)]
        durable: bool,
    },
    Delete {
        #[command(flatten)]
        key: Key,
        #[arg(long)]
        durable: bool,
    },
    Keys {
        #[arg(long, default_value = "")]
        prefix: String,
        #[arg(long)]
        after: Option<String>,
        #[arg(long)]
        base64: bool,
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(i64).range(1..=1024))]
        limit: i64,
        #[arg(long, default_value_t = 1_048_576, value_parser = clap::value_parser!(i64).range(1..=1_048_576))]
        max_bytes: i64,
    },
    /// Make prior buffered writes durable; does not delete data
    Flush,
    Stats,
}
