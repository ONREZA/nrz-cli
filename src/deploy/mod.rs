use crate::cli::DeployArgs;

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
    let _project_dir = std::path::Path::new(&args.dir).canonicalize()?;

    if args.token.is_none() {
        anyhow::bail!("deploy token required. Use --token or set NRZ_TOKEN env var");
    }

    // TODO: validate build output first
    // TODO: create deployment via API
    // TODO: upload artifacts to S3
    // TODO: finalize deployment

    eprintln!("nrz deploy: not yet implemented");
    Ok(())
}
