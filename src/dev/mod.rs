mod detect;
mod inject;
mod process;

use crate::cli::DevArgs;

/// Start local dev server with platform emulation.
///
/// 1. Detect framework (Astro, Nuxt, Nitro, SvelteKit)
/// 2. Start emulator (KV, DB, Context)
/// 3. Generate JS bootstrap that sets globalThis.ONREZA
/// 4. Spawn framework dev command as child process
/// 5. Forward signals, handle graceful shutdown
pub async fn run(args: DevArgs) -> anyhow::Result<()> {
    let project_dir = std::path::Path::new(&args.dir).canonicalize()?;

    let framework = detect::detect_framework(&project_dir)?;
    tracing::info!(?framework, "detected framework");

    // TODO: start emulator services
    // TODO: generate ONREZA bootstrap script
    // TODO: spawn framework dev server
    // TODO: watch for config changes

    eprintln!("nrz dev: not yet implemented");
    Ok(())
}
