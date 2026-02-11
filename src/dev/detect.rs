use std::path::Path;

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Framework {
    pub name: FrameworkName,
    pub dev_command: String,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameworkName {
    Astro,
    Nuxt,
    Nitro,
    SvelteKit,
}

/// Auto-detect framework from project dependencies.
pub fn detect_framework(project_dir: &Path) -> anyhow::Result<Framework> {
    let pkg_path = project_dir.join("package.json");
    let pkg_content =
        std::fs::read_to_string(&pkg_path).context("package.json not found in project dir")?;

    let pkg: serde_json::Value =
        serde_json::from_str(&pkg_content).context("invalid package.json")?;

    let has_dep = |name: &str| -> bool {
        pkg.get("dependencies")
            .and_then(|d| d.get(name))
            .is_some()
            || pkg
                .get("devDependencies")
                .and_then(|d| d.get(name))
                .is_some()
    };

    if has_dep("astro") {
        return Ok(Framework {
            name: FrameworkName::Astro,
            dev_command: "astro dev".into(),
        });
    }

    if has_dep("nuxt") {
        return Ok(Framework {
            name: FrameworkName::Nuxt,
            dev_command: "nuxt dev".into(),
        });
    }

    if has_dep("@sveltejs/kit") {
        return Ok(Framework {
            name: FrameworkName::SvelteKit,
            dev_command: "vite dev".into(),
        });
    }

    if has_dep("nitropack") {
        return Ok(Framework {
            name: FrameworkName::Nitro,
            dev_command: "nitro dev".into(),
        });
    }

    anyhow::bail!(
        "could not detect framework — expected astro, nuxt, @sveltejs/kit, or nitropack in dependencies"
    );
}
