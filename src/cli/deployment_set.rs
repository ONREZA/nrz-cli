use clap::Args;
use uuid::Uuid;

#[derive(Args)]
pub struct DeploymentSetArgs {
    #[arg(long)]
    pub project_id: Uuid,
    /// Exact Environment ID or name
    #[arg(long)]
    pub environment: String,
    /// Replace every active Service, including independent Git and local Sources
    #[arg(
        long,
        conflicts_with = "service_ids",
        required_unless_present = "service_ids"
    )]
    pub all: bool,
    /// Service to include; repeat to select a set
    #[arg(long = "service-id", required_unless_present = "all")]
    pub service_ids: Vec<Uuid>,
    /// Source checkout mapping SERVICE_UUID=PATH; repeat for local Sources
    #[arg(long = "local")]
    pub local: Vec<String>,
    /// Reuse the exact stored command after an unknown admission response
    #[arg(long)]
    pub idempotency_key: Option<Uuid>,
    #[arg(long)]
    pub expected_generation: Option<String>,
    /// A timeout preserves the admitted operation and uploaded Sources
    #[arg(long, default_value_t = 1800, value_parser = clap::value_parser!(u32).range(1..))]
    pub wait_timeout: u32,
}
