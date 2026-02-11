//! Auth stub for ONREZA platform.
//!
//! Device flow:
//! 1. POST /api/auth/device → { device_code, user_code, verification_uri }
//! 2. Open browser to verification_uri
//! 3. Poll POST /api/auth/device/token until approved
//! 4. Save token to ~/.config/nrz/credentials.json

pub async fn login() -> anyhow::Result<()> {
    // TODO: POST /api/auth/device to get device_code + verification URL
    // TODO: Open browser with `open` crate or xdg-open
    // TODO: Poll POST /api/auth/device/token until approved or timeout
    // TODO: Save token to credentials_path()

    eprintln!(
        "  {} login: API integration not yet implemented",
        console::style("!").yellow().bold(),
    );
    Ok(())
}

pub async fn whoami() -> anyhow::Result<()> {
    let creds_path = credentials_path();
    if !creds_path.exists() {
        eprintln!("not logged in. Run `nrz login` first.");
        return Ok(());
    }

    // TODO: GET /api/auth/me with saved token
    // TODO: Print user info (email, team, etc.)

    eprintln!(
        "  {} whoami: API integration not yet implemented",
        console::style("!").yellow().bold(),
    );
    eprintln!("  credentials file: {}", creds_path.display());
    Ok(())
}

fn credentials_path() -> std::path::PathBuf {
    dirs_home()
        .join(".config")
        .join("nrz")
        .join("credentials.json")
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}
