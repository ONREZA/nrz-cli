use std::io::{BufRead, IsTerminal, Write};

use nrz::config::{self, ProjectConfig};

use crate::{api::ApiClient, auth};

pub(crate) fn project_client(
    token: Option<&str>,
    workspace: Option<&str>,
    project_id: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<(ApiClient, String)> {
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    let project_id = config::resolve_project_id(project_id, config)?;
    Ok((client, project_id))
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;

/// Destructive commands require explicit confirmation in both human and machine modes.
pub(crate) fn confirm_deletion(
    entity: &str,
    id: &str,
    force: bool,
    json: bool,
) -> anyhow::Result<()> {
    if force {
        return Ok(());
    }
    if json || !std::io::stdin().is_terminal() {
        anyhow::bail!("--force is required in non-interactive mode");
    }
    eprint!(
        "  {} ",
        console::style(format!(
            "Type {entity} ID ({}) to confirm deletion:",
            crate::output::terminal_line(id)
        ))
        .bold()
    );
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    if line.trim() != id {
        anyhow::bail!("confirmation did not match, aborting.");
    }
    Ok(())
}
