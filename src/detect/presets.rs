//! Framework preset definitions — compile-time data, 1:1 with TS/Go.

use super::types::{FrameworkPreset, PresetCategory, RuntimeType};

/// All framework presets sorted by detection priority.
pub static PRESETS: &[FrameworkPreset] = &[
    // Tier 1: Highly specific frameworks (priority 1-4)
    FrameworkPreset {
        slug: "nextjs",
        name: "Next.js",
        dependencies: &["next"],
        output_directory: ".next",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 1,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "nuxt",
        name: "Nuxt.js",
        dependencies: &["nuxt"],
        output_directory: ".output",
        build_script: Some("build"),
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
        slug: "react-router",
        name: "React Router",
        dependencies: &["@react-router/dev"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 4,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "remix",
        name: "Remix",
        dependencies: &["@remix-run/react", "@remix-run/dev"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 5,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "gatsby",
        name: "Gatsby",
        dependencies: &["gatsby"],
        output_directory: "public",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 6,
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
    // Tier 3b: Server frameworks (priority 30-39)
    FrameworkPreset {
        slug: "hono",
        name: "Hono",
        dependencies: &["hono"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 30,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "elysia",
        name: "Elysia",
        dependencies: &["elysia"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 31,
        runtime: RuntimeType::Bun,
    },
    FrameworkPreset {
        slug: "nestjs",
        name: "NestJS",
        dependencies: &["@nestjs/core"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 32,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "fastify",
        name: "Fastify",
        dependencies: &["fastify"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 33,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "adonis",
        name: "AdonisJS",
        dependencies: &["@adonisjs/core"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 34,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "express",
        name: "Express",
        dependencies: &["express"],
        output_directory: ".",
        build_script: None,
        category: PresetCategory::Server,
        priority: 35,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "koa",
        name: "Koa",
        dependencies: &["koa"],
        output_directory: ".",
        build_script: None,
        category: PresetCategory::Server,
        priority: 36,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "h3",
        name: "H3",
        dependencies: &["h3"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 37,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "nitro",
        name: "Nitro",
        dependencies: &["nitropack"],
        output_directory: ".output",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 38,
        runtime: RuntimeType::Node,
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

/// Static HTML preset — separate from detection presets since it has no
/// dependency markers. Used directly by `detect()` when the static HTML
/// fallback triggers (index.html found, no package.json).
pub static STATIC_HTML_PRESET: FrameworkPreset = FrameworkPreset {
    slug: "static-html",
    name: "Static HTML",
    dependencies: &[],
    output_directory: ".",
    build_script: None,
    category: PresetCategory::Static,
    priority: 28,
    runtime: RuntimeType::Static,
};

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
    &STATIC_HTML_PRESET
}

/// Additional output directories to search for a given framework slug,
/// beyond the config defaults in `output_dirs()`.
/// Used by `build::detect_output_dir` to search framework-specific paths
/// in addition to config defaults.
pub fn framework_output_dirs(slug: &str) -> &'static [&'static str] {
    match slug {
        "nextjs" => &[".next/standalone", ".next", "out"],
        "nuxt" => &[".output"],
        "sveltekit" => &["build"],
        "react-router" => &["build/client", "build/server", "build"],
        "remix" => &["build/client", "build/server", "build"],
        "gatsby" => &["public"],
        "cra" => &["build"],
        "astro" => &["dist"],
        "vite" => &["dist"],
        "vue" => &["dist"],
        "angular" => &["dist"],
        "preact" => &["build"],
        "docusaurus" => &["build"],
        "vitepress" => &[".vitepress/dist"],
        "eleventy" => &["_site"],
        "hexo" => &["public"],
        "parcel" => &["dist"],
        "stencil" => &["www"],
        "hono" => &["dist"],
        "elysia" => &["dist"],
        "nestjs" => &["dist"],
        "fastify" => &["dist", "."],
        "adonis" => &["build"],
        "express" => &["."],
        "koa" => &["."],
        "h3" => &["dist", "."],
        "nitro" => &[".output"],
        "static-html" => &["."],
        _ => &[],
    }
}

/// Check if a framework slug is an SSR-capable framework.
#[allow(dead_code)]
pub fn is_ssr_framework(slug: &str) -> bool {
    matches!(
        slug,
        "nextjs" | "nuxt" | "sveltekit" | "astro" | "remix" | "react-router"
    )
}

/// Check if a framework slug is a server-only framework (always PROCESS, no SSR analysis).
pub fn is_server_framework(slug: &str) -> bool {
    matches!(
        slug,
        "hono" | "elysia" | "nestjs" | "fastify" | "adonis" | "express" | "koa" | "h3" | "nitro"
    )
}

/// Get presets that have dependencies (used for detection), sorted by priority.
pub fn detection_presets() -> impl Iterator<Item = &'static FrameworkPreset> {
    PRESETS.iter().filter(|p| !p.dependencies.is_empty())
}
