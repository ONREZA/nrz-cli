use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct BuildEnvPatch {
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct BuildPreparation {
    pub(crate) env: Vec<BuildEnvPatch>,
    pub(crate) warnings: Vec<String>,
}

impl BuildPreparation {
    pub(crate) fn env_pairs(&self) -> Vec<(String, String)> {
        self.env
            .iter()
            .map(|patch| (patch.key.clone(), patch.value.clone()))
            .collect()
    }
}

pub(crate) fn prepare_build(project_dir: &Path) -> anyhow::Result<BuildPreparation> {
    let mut preparation = BuildPreparation::default();

    if is_nextjs_project(project_dir) {
        match crate::nextjs_adapter::prepare_build_adapter(project_dir) {
            Ok(Some(adapter)) => {
                preparation.env.push(BuildEnvPatch {
                    key: "NEXT_ADAPTER_PATH".to_string(),
                    value: adapter.path.to_string_lossy().into_owned(),
                    message: "Next.js detected, enabling ONREZA adapter (NEXT_ADAPTER_PATH)"
                        .to_string(),
                });
                preparation.env.push(BuildEnvPatch {
                    key: "ONREZA_NEXT_ADAPTER_VERSION".to_string(),
                    value: env!("CARGO_PKG_VERSION").to_string(),
                    message: String::new(),
                });
            }
            Ok(None) => {
                preparation.env.push(BuildEnvPatch {
                    key: "NEXT_PRIVATE_STANDALONE".to_string(),
                    value: "1".to_string(),
                    message:
                        "Next.js detected, enabling standalone output (NEXT_PRIVATE_STANDALONE=1)"
                            .to_string(),
                });
            }
            Err(err) => {
                preparation.warnings.push(format!(
                    "Could not prepare Next.js adapter: {err:#}. Falling back to standalone output."
                ));
                preparation.env.push(BuildEnvPatch {
                    key: "NEXT_PRIVATE_STANDALONE".to_string(),
                    value: "1".to_string(),
                    message:
                        "Next.js detected, enabling standalone output (NEXT_PRIVATE_STANDALONE=1)"
                            .to_string(),
                });
            }
        }
    }

    if is_sveltekit_with_adapter_auto(project_dir) {
        preparation.env.push(BuildEnvPatch {
            key: "GCP_BUILDPACKS".to_string(),
            value: "1".to_string(),
            message: "SvelteKit adapter-auto detected, enabling adapter-node (GCP_BUILDPACKS=1)"
                .to_string(),
        });
    }

    Ok(preparation)
}

pub(crate) fn clear_before_build(project_dir: &Path) -> anyhow::Result<()> {
    if is_nextjs_project(project_dir) {
        crate::nextjs_adapter::clear_descriptor(project_dir)?;
    }
    Ok(())
}

pub(crate) fn compute_aware_output_dirs(
    detection: &crate::detect::types::DetectionResult,
) -> Vec<&'static str> {
    match detection.framework.as_str() {
        "nextjs" | "blitzjs" | "payload" => {
            if let Some(ref ssr) = detection.metadata.ssr_analysis {
                if ssr.is_static_compatible {
                    return vec!["out"];
                }
                if ssr.has_standalone_output() {
                    return vec![".next/standalone", ".next"];
                }
            }
            vec![".next/standalone", ".next"]
        }
        "nuxt" => {
            if let Some(ref ssr) = detection.metadata.ssr_analysis
                && ssr.is_static_compatible
            {
                return vec![".output/public", ".output"];
            }
            vec![".output"]
        }
        "remix" | "react-router" => {
            if let Some(ref ssr) = detection.metadata.ssr_analysis
                && ssr.is_static_compatible
            {
                return vec!["build/client", "build"];
            }
            vec!["build"]
        }
        "hydrogen" => {
            if let Some(ref ssr) = detection.metadata.ssr_analysis
                && ssr.is_static_compatible
            {
                return vec!["dist/client", "build/client", "build"];
            }
            vec!["dist", "build"]
        }
        "tanstack-start" => vec![".output", "dist"],
        "static-html" => {
            if detection
                .metadata
                .build_info
                .as_ref()
                .and_then(|info| info.output_dir.as_deref())
                == Some(".")
            {
                vec!["."]
            } else {
                crate::detect::presets::PACKAGE_STATIC_OUTPUT_DIRS.to_vec()
            }
        }
        slug => crate::detect::presets::framework_output_dirs(slug).to_vec(),
    }
}

pub(crate) fn is_nextjs_project(project_dir: &Path) -> bool {
    let Some(pkg) = crate::detect::package_json::PackageJson::load(project_dir) else {
        return false;
    };
    pkg.has_dependency("next")
}

pub(crate) fn is_sveltekit_with_adapter_auto(project_dir: &Path) -> bool {
    let Some(pkg) = crate::detect::package_json::PackageJson::load(project_dir) else {
        return false;
    };
    if !pkg.has_dependency("@sveltejs/kit") {
        return false;
    }
    if pkg.has_dependency("@sveltejs/adapter-node")
        || pkg.has_dependency("@sveltejs/adapter-static")
        || pkg.has_dependency("@sveltejs/adapter-vercel")
        || pkg.has_dependency("@sveltejs/adapter-cloudflare")
        || pkg.has_dependency("@sveltejs/adapter-netlify")
    {
        return false;
    }
    let config_content = ["svelte.config.js", "svelte.config.ts"]
        .iter()
        .map(|name| project_dir.join(name))
        .find(|path| path.is_file())
        .and_then(|path| std::fs::read_to_string(path).ok());
    match config_content {
        Some(content) => content.contains("adapter-auto"),
        None => true,
    }
}
