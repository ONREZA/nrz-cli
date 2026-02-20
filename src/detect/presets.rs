//! Framework preset definitions — compile-time data, 1:1 with TS/Go.

use super::types::{FrameworkPreset, PresetCategory, RuntimeType};

/// All framework presets sorted by detection priority.
pub static PRESETS: &[FrameworkPreset] = &[
    // Tier 1: Highly specific frameworks (priority 1-4)
    FrameworkPreset {
        slug: "nextjs",
        name: "Next.js",
        dependencies: &["next"],
        output_directory: "out",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 1,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "nuxt",
        name: "Nuxt.js",
        dependencies: &["nuxt"],
        output_directory: ".output/public",
        build_script: Some("generate"),
        category: PresetCategory::Vue,
        priority: 2,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "sveltekit",
        name: "SvelteKit",
        dependencies: &["@sveltejs/kit"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::Svelte,
        priority: 3,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "gatsby",
        name: "Gatsby",
        dependencies: &["gatsby"],
        output_directory: "public",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 4,
        runtime: RuntimeType::Node,
    },
    // Tier 2: CLI-based frameworks (priority 10-13)
    FrameworkPreset {
        slug: "cra",
        name: "Create React App",
        dependencies: &["react-scripts"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 10,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "vue",
        name: "Vue CLI",
        dependencies: &["@vue/cli-service"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Vue,
        priority: 11,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "angular",
        name: "Angular",
        dependencies: &["@angular/core"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Other,
        priority: 12,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "preact",
        name: "Preact CLI",
        dependencies: &["preact-cli"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 13,
        runtime: RuntimeType::Node,
    },
    // Tier 3: Static site generators (priority 20-26)
    FrameworkPreset {
        slug: "astro",
        name: "Astro",
        dependencies: &["astro"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Static,
        priority: 20,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "docusaurus",
        name: "Docusaurus",
        dependencies: &["@docusaurus/core"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::Static,
        priority: 21,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "vitepress",
        name: "VitePress",
        dependencies: &["vitepress"],
        output_directory: ".vitepress/dist",
        build_script: Some("docs:build"),
        category: PresetCategory::Static,
        priority: 22,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "eleventy",
        name: "Eleventy",
        dependencies: &["@11ty/eleventy"],
        output_directory: "_site",
        build_script: Some("build"),
        category: PresetCategory::Static,
        priority: 23,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "hexo",
        name: "Hexo",
        dependencies: &["hexo"],
        output_directory: "public",
        build_script: Some("build"),
        category: PresetCategory::Static,
        priority: 24,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "parcel",
        name: "Parcel",
        dependencies: &["parcel"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Static,
        priority: 25,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "stencil",
        name: "Stencil",
        dependencies: &["@stencil/core"],
        output_directory: "www",
        build_script: Some("build"),
        category: PresetCategory::Other,
        priority: 26,
        runtime: RuntimeType::Node,
    },
    // Tier 3.6: Plain static HTML (no build)
    FrameworkPreset {
        slug: "static-html",
        name: "Static HTML",
        dependencies: &[],
        output_directory: ".",
        build_script: None,
        category: PresetCategory::Static,
        priority: 28,
        runtime: RuntimeType::Static,
    },
    // Tier 4: Generic catch-all
    FrameworkPreset {
        slug: "vite",
        name: "Vite",
        dependencies: &["vite"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 100,
        runtime: RuntimeType::Node,
    },
    // Default preset
    FrameworkPreset {
        slug: "other",
        name: "Other",
        dependencies: &[],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Other,
        priority: 1000,
        runtime: RuntimeType::Node,
    },
];

/// Get a preset by slug, or `None`.
pub fn get_preset_by_slug(slug: &str) -> Option<&'static FrameworkPreset> {
    PRESETS.iter().find(|p| p.slug == slug)
}

/// Get the default "other" preset.
pub fn get_default_preset() -> &'static FrameworkPreset {
    get_preset_by_slug("other").expect("'other' preset must exist")
}

/// Get the static-html preset.
pub fn get_static_html_preset() -> &'static FrameworkPreset {
    get_preset_by_slug("static-html").expect("'static-html' preset must exist")
}

/// Check if a framework slug is an SSR-capable framework.
#[allow(dead_code)]
pub fn is_ssr_framework(slug: &str) -> bool {
    matches!(slug, "nextjs" | "nuxt" | "sveltekit")
}

/// Get presets that have dependencies (used for detection), sorted by priority.
pub fn detection_presets() -> impl Iterator<Item = &'static FrameworkPreset> {
    PRESETS.iter().filter(|p| !p.dependencies.is_empty())
}
