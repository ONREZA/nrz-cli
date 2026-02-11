use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::build;
use crate::cli::{BuildArgs, DeployArgs};

// --- API response types ---

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct DeploymentResponse {
    id: String,
    status: String,
    url: Option<String>,
    upload_urls: Option<UploadUrls>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct UploadUrls {
    server: String,
    assets: String,
    prerender: Option<String>,
}

/// Deploy build output to ONREZA platform.
///
/// 1. Run `nrz build` validation
/// 2. Authenticate (token from --token or NRZ_TOKEN)
/// 3. Create deployment via API
/// 4. Upload server bundle to S3
/// 5. Upload static assets to S3
/// 6. Upload prerendered pages to S3
/// 7. Finalize deployment (activate routes)
pub async fn run(args: DeployArgs) -> anyhow::Result<()> {
    let project_dir = std::path::Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    let token = args.token.as_deref().ok_or_else(|| {
        anyhow::anyhow!("deploy token required. Use --token or set NRZ_TOKEN env var")
    })?;

    // Step 1: Validate build output
    eprintln!(
        "  {} validating build output...",
        console::style("~").cyan().bold(),
    );
    build::run(BuildArgs {
        dir: project_dir.to_string_lossy().into_owned(),
        skip_validation: false,
    })
    .await?;

    // Step 2: Create deployment
    eprintln!(
        "  {} creating deployment...",
        console::style("~").cyan().bold(),
    );
    let _env = if args.prod { "production" } else { "preview" };
    let _token = token;

    // TODO: POST /api/projects/:id/deployments
    //   Headers: Authorization: Bearer {token}
    //   Body: { environment, manifest }
    //   Response: DeploymentResponse { id, upload_urls }

    // Step 3: Upload artifacts
    eprintln!(
        "  {} uploading artifacts...",
        console::style("~").cyan().bold(),
    );
    // TODO: Upload server bundle to upload_urls.server (presigned S3 URL)
    // TODO: Upload assets directory to upload_urls.assets (presigned S3 URL)
    // TODO: Upload prerender directory to upload_urls.prerender (if present)

    // Step 4: Signal upload complete
    // TODO: POST /api/deployments/:id/upload-complete

    // Step 5: Activate deployment
    // TODO: POST /api/deployments/:id/activate

    // Step 6: Poll for ready status
    // TODO: GET /api/deployments/:id — poll until status=ready or error
    //   Timeout after 120s

    eprintln!(
        "  {} deploy: API integration not yet implemented",
        console::style("!").yellow().bold(),
    );
    eprintln!("  build validation passed — deploy API calls are TODO");

    Ok(())
}
