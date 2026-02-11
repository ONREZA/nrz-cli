use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub adapter: AdapterInfo,
    pub framework: FrameworkInfo,
    pub server: ServerConfig,
    pub assets: AssetsConfig,
    pub routes: Vec<Route>,
    pub prerender: Option<PrerenderConfig>,
    pub features: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AdapterInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct FrameworkInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub entry: String,
    pub export: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetsConfig {
    pub directory: String,
    pub prefix: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RouteType {
    Static,
    Ssr,
    Api,
    Isr,
    EdgeFunction,
}

#[derive(Debug, Deserialize)]
pub struct Route {
    pub pattern: String,
    #[serde(rename = "type")]
    pub route_type: RouteType,
    pub priority: Option<i32>,
    pub revalidate: Option<u64>,
    pub methods: Option<Vec<String>>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct PrerenderConfig {
    pub directory: String,
    pub pages: std::collections::HashMap<String, PrerenderPage>,
}

#[derive(Debug, Deserialize)]
pub struct PrerenderPage {
    pub html: String,
    pub data: Option<String>,
}

pub fn load_and_validate(path: &Path) -> anyhow::Result<Manifest> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let manifest: Manifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    // Version check
    if manifest.version != 1 {
        anyhow::bail!(
            "unsupported manifest version: {}. Upgrade your adapter.",
            manifest.version
        );
    }

    // Adapter name check
    if !manifest.adapter.name.starts_with("@onreza/") {
        anyhow::bail!(
            "unknown adapter: {}. Use an official @onreza/* adapter.",
            manifest.adapter.name
        );
    }

    // Export format check
    if manifest.server.export != "fetch" {
        anyhow::bail!(
            "unsupported export format: {}. Expected \"fetch\".",
            manifest.server.export
        );
    }

    // Routes check
    if manifest.routes.is_empty() {
        anyhow::bail!("no routes defined in manifest");
    }

    // ISR routes must have revalidate
    for route in &manifest.routes {
        if route.route_type == RouteType::Isr {
            match route.revalidate {
                Some(0) | None => {
                    anyhow::bail!("ISR route '{}' must have revalidate > 0", route.pattern);
                }
                _ => {}
            }
        }
    }

    Ok(manifest)
}

pub fn verify_files(output_dir: &Path, manifest: &Manifest) -> anyhow::Result<()> {
    // Server entry exists
    let entry_path = output_dir.join(&manifest.server.entry);
    if !entry_path.exists() {
        anyhow::bail!("server entry not found: {}", entry_path.display());
    }

    // Assets directory exists
    let assets_path = output_dir.join(&manifest.assets.directory);
    if !assets_path.is_dir() {
        anyhow::bail!("assets directory not found: {}", assets_path.display());
    }

    // Prerender directory if declared
    if let Some(prerender) = &manifest.prerender {
        let prerender_path = output_dir.join(&prerender.directory);
        if !prerender_path.is_dir() {
            anyhow::bail!(
                "prerender directory not found: {}",
                prerender_path.display()
            );
        }
    }

    Ok(())
}


