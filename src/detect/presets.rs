//! Framework preset definitions — compile-time data, 1:1 with TS/Go.

use super::types::{
    FrameworkDetectionRule, FrameworkDetector, FrameworkPreset, PresetCategory, RuntimeType,
};

use FrameworkDetector::{Content, ContentAny, Package, Path, RuntimePackage, RuntimeSignal};

pub const PACKAGE_STATIC_OUTPUT_DIRS: &[&str] = &[
    "dist",
    ".output",
    "build",
    "out",
    "_site",
    "www",
    ".vitepress/dist",
];

/// All framework presets sorted by detection priority.
pub static PRESETS: &[FrameworkPreset] = &[
    // Tier 0: Wrappers that also pull in a Tier 1 framework (must be checked first)
    FrameworkPreset {
        slug: "blitzjs",
        name: "Blitz.js",
        dependencies: &["@blitzjs/next"],
        output_directory: ".next",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 1,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "keystone",
        name: "Keystone",
        dependencies: &["@keystone-6/core"],
        output_directory: ".keystone",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 1,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "payload",
        name: "Payload CMS",
        dependencies: &["@payloadcms/next"],
        output_directory: ".next",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 1,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "hydrogen",
        name: "Hydrogen",
        dependencies: &["@shopify/hydrogen"],
        output_directory: "build",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 1,
        runtime: RuntimeType::Node,
    },
    // Tier 1: Highly specific frameworks (priority 1-6)
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
    // Tier 1b: Next-gen SSR meta-frameworks (priority 7-9)
    FrameworkPreset {
        slug: "solidstart",
        name: "SolidStart",
        dependencies: &["@solidjs/start"],
        output_directory: ".output",
        build_script: Some("build"),
        category: PresetCategory::Other,
        priority: 7,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "qwik",
        name: "Qwik City",
        dependencies: &["@builder.io/qwik-city", "@qwik.dev/router"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Other,
        priority: 8,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "analog",
        name: "Analog",
        dependencies: &["@analogjs/platform"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Other,
        priority: 9,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "tanstack-start",
        name: "TanStack Start",
        dependencies: &["@tanstack/react-start"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::React,
        priority: 9,
        runtime: RuntimeType::Node,
    },
    // Tier 1c: Mobile-first web frameworks (priority 9)
    FrameworkPreset {
        slug: "expo",
        name: "Expo",
        dependencies: &["expo"],
        output_directory: "dist",
        build_script: None,
        category: PresetCategory::React,
        priority: 9,
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
    // Tier 3c: Full-stack / CMS frameworks (priority 39-41)
    FrameworkPreset {
        slug: "redwoodjs",
        name: "RedwoodJS",
        dependencies: &["@redwoodjs/core"],
        output_directory: "api/dist",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 39,
        runtime: RuntimeType::Node,
    },
    FrameworkPreset {
        slug: "strapi",
        name: "Strapi",
        dependencies: &["@strapi/strapi"],
        output_directory: "dist",
        build_script: Some("build"),
        category: PresetCategory::Server,
        priority: 41,
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

const SERVER_ENTRY_FILES: &[&str] = &[
    "server.js",
    "server.mjs",
    "server.cjs",
    "server.ts",
    "server.mts",
    "server.cts",
    "app.js",
    "app.mjs",
    "app.cjs",
    "app.ts",
    "app.mts",
    "app.cts",
    "index.js",
    "index.mjs",
    "index.cjs",
    "index.ts",
    "index.mts",
    "index.cts",
    "main.js",
    "main.mjs",
    "main.cjs",
    "main.ts",
    "main.mts",
    "main.cts",
    "src/server.js",
    "src/server.mjs",
    "src/server.cjs",
    "src/server.ts",
    "src/server.mts",
    "src/server.cts",
    "src/app.js",
    "src/app.mjs",
    "src/app.cjs",
    "src/app.ts",
    "src/app.mts",
    "src/app.cts",
    "src/index.js",
    "src/index.mjs",
    "src/index.cjs",
    "src/index.ts",
    "src/index.mts",
    "src/index.cts",
    "src/main.js",
    "src/main.mjs",
    "src/main.cjs",
    "src/main.ts",
    "src/main.mts",
    "src/main.cts",
    "dist/server.js",
    "dist/server.mjs",
    "dist/server.cjs",
    "dist/index.js",
    "dist/index.mjs",
    "dist/index.cjs",
    "dist/main.js",
    "dist/main.mjs",
    "dist/main.cjs",
    "dist/src/main.js",
    "dist/src/main.mjs",
    "dist/src/main.cjs",
    "build/server.js",
    "build/server.mjs",
    "build/server.cjs",
    "build/index.js",
    "build/index.mjs",
    "build/index.cjs",
    "build/main.js",
    "build/main.mjs",
    "build/main.cjs",
];

const HONO_IMPORT: &str =
    r#"(?m)(?:from\s+["']hono["']|require\(\s*["']hono["']\s*\)|import\(\s*["']hono["']\s*\))"#;
const ELYSIA_IMPORT: &str = r#"(?m)(?:from\s+["']elysia["']|require\(\s*["']elysia["']\s*\)|import\(\s*["']elysia["']\s*\))"#;
const NESTJS_IMPORT: &str = r#"(?m)(?:from\s+["']@nestjs/(?:core|common|platform-[^"']+)["']|require\(\s*["']@nestjs/(?:core|common|platform-[^"']+)["']\s*\)|import\(\s*["']@nestjs/(?:core|common|platform-[^"']+)["']\s*\))"#;
const FASTIFY_IMPORT: &str = r#"(?m)(?:from\s+["']fastify["']|require\(\s*["']fastify["']\s*\)|import\(\s*["']fastify["']\s*\))"#;
const EXPRESS_IMPORT: &str = r#"(?m)(?:from\s+["']express["']|require\(\s*["']express["']\s*\)|import\(\s*["']express["']\s*\))"#;
const KOA_IMPORT: &str =
    r#"(?m)(?:from\s+["']koa["']|require\(\s*["']koa["']\s*\)|import\(\s*["']koa["']\s*\))"#;
const H3_IMPORT: &str =
    r#"(?m)(?:from\s+["']h3["']|require\(\s*["']h3["']\s*\)|import\(\s*["']h3["']\s*\))"#;
const NITRO_IMPORT: &str = r#"(?m)(?:from\s+["']nitropack["']|require\(\s*["']nitropack["']\s*\)|import\(\s*["']nitropack["']\s*\))"#;

/// Declarative framework detection rules sorted by preset priority.
pub static DETECTION_RULES: &[FrameworkDetectionRule] = &[
    FrameworkDetectionRule {
        slug: "blitzjs",
        every: &[Package("@blitzjs/next")],
        some: &[],
        supersedes: &["nextjs"],
    },
    FrameworkDetectionRule {
        slug: "keystone",
        every: &[Package("@keystone-6/core")],
        some: &[Path("keystone.ts"), Path("keystone.js"), RuntimeSignal],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "payload",
        every: &[Package("@payloadcms/next")],
        some: &[],
        supersedes: &["nextjs"],
    },
    FrameworkDetectionRule {
        slug: "hydrogen",
        every: &[Package("@shopify/hydrogen")],
        some: &[],
        supersedes: &["react-router", "remix", "vite"],
    },
    FrameworkDetectionRule {
        slug: "nextjs",
        every: &[Package("next")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "nuxt",
        every: &[Package("nuxt")],
        some: &[],
        supersedes: &["nitro", "h3", "vue", "vite"],
    },
    FrameworkDetectionRule {
        slug: "sveltekit",
        every: &[Package("@sveltejs/kit")],
        some: &[],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "react-router",
        every: &[Package("@react-router/dev")],
        some: &[],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "remix",
        every: &[],
        some: &[Package("@remix-run/react"), Package("@remix-run/dev")],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "gatsby",
        every: &[Package("gatsby")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "solidstart",
        every: &[Package("@solidjs/start")],
        some: &[],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "qwik",
        every: &[],
        some: &[
            Package("@builder.io/qwik-city"),
            Package("@qwik.dev/router"),
        ],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "analog",
        every: &[Package("@analogjs/platform")],
        some: &[],
        supersedes: &["angular", "vite"],
    },
    FrameworkDetectionRule {
        slug: "tanstack-start",
        every: &[Package("@tanstack/react-start")],
        some: &[],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "expo",
        every: &[Package("expo")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "cra",
        every: &[Package("react-scripts")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "vue",
        every: &[Package("@vue/cli-service")],
        some: &[],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "angular",
        every: &[Package("@angular/core")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "preact",
        every: &[Package("preact-cli")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "astro",
        every: &[Package("astro")],
        some: &[],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "docusaurus",
        every: &[Package("@docusaurus/core")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "vitepress",
        every: &[Package("vitepress")],
        some: &[],
        supersedes: &["vite"],
    },
    FrameworkDetectionRule {
        slug: "eleventy",
        every: &[Package("@11ty/eleventy")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "hexo",
        every: &[Package("hexo")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "parcel",
        every: &[Package("parcel")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "stencil",
        every: &[Package("@stencil/core")],
        some: &[],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "hono",
        every: &[Package("hono")],
        some: &[
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: HONO_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &["express"],
    },
    FrameworkDetectionRule {
        slug: "elysia",
        every: &[Package("elysia")],
        some: &[
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: ELYSIA_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &["express"],
    },
    FrameworkDetectionRule {
        slug: "nestjs",
        every: &[Package("@nestjs/core")],
        some: &[
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: NESTJS_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &["express", "fastify"],
    },
    FrameworkDetectionRule {
        slug: "fastify",
        every: &[Package("fastify")],
        some: &[
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: FASTIFY_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &["express"],
    },
    FrameworkDetectionRule {
        slug: "adonis",
        every: &[Package("@adonisjs/core")],
        some: &[
            Path("adonisrc.ts"),
            Path("adonisrc.js"),
            Path("bin/server.js"),
            RuntimeSignal,
        ],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "express",
        every: &[RuntimePackage("express")],
        some: &[
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: EXPRESS_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "koa",
        every: &[RuntimePackage("koa")],
        some: &[
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: KOA_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "h3",
        every: &[Package("h3")],
        some: &[
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: H3_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "nitro",
        every: &[Package("nitropack")],
        some: &[
            Path("nitro.config.ts"),
            Path("nitro.config.js"),
            Content {
                path: "nitro.config.ts",
                pattern: "defineNitroConfig",
            },
            ContentAny {
                paths: SERVER_ENTRY_FILES,
                pattern: NITRO_IMPORT,
            },
            RuntimeSignal,
        ],
        supersedes: &["h3"],
    },
    FrameworkDetectionRule {
        slug: "redwoodjs",
        every: &[Package("@redwoodjs/core")],
        some: &[Path("redwood.toml"), RuntimeSignal],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "strapi",
        every: &[Package("@strapi/strapi")],
        some: &[
            Path("config/server.ts"),
            Path("config/server.js"),
            RuntimeSignal,
        ],
        supersedes: &[],
    },
    FrameworkDetectionRule {
        slug: "vite",
        every: &[Package("vite")],
        some: &[],
        supersedes: &[],
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
        "expo" => &["dist"],
        "cra" => &["build"],
        "solidstart" => &[".output"],
        "qwik" => &["dist", "server"],
        "analog" => &["dist/analog", "dist"],
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
        "tanstack-start" => &[".output", "dist"],
        // Hydrogen has two layouts: default (Oxygen workers) emits dist/,
        // Express recipe emits build/. `dist` is tried first so the workers-runtime
        // detector can fail fast on Oxygen; `build` covers the Express recipe.
        "hydrogen" => &[
            "dist/client",
            "dist/server",
            "dist",
            "build/client",
            "build/server",
            "build",
        ],
        "nitro" => &[".output"],
        "blitzjs" => &[".next/standalone", ".next"],
        "keystone" => &[".keystone"],
        "redwoodjs" => &["api/dist", "web/dist"],
        "payload" => &[".next/standalone", ".next"],
        "strapi" => &["dist", "build"],
        "static-html" => &["."],
        _ => &[],
    }
}

/// Check if a framework slug is an SSR-capable framework.
#[allow(dead_code)]
pub fn is_ssr_framework(slug: &str) -> bool {
    matches!(
        slug,
        "nextjs"
            | "nuxt"
            | "sveltekit"
            | "astro"
            | "remix"
            | "react-router"
            | "solidstart"
            | "qwik"
            | "analog"
            | "blitzjs"
            | "payload"
            | "tanstack-start"
            | "hydrogen"
    )
}

/// Check if a framework slug is a server-only framework (always PROCESS, no SSR analysis).
pub fn is_server_framework(slug: &str) -> bool {
    matches!(
        slug,
        "hono"
            | "elysia"
            | "nestjs"
            | "fastify"
            | "adonis"
            | "express"
            | "koa"
            | "h3"
            | "nitro"
            | "keystone"
            | "redwoodjs"
            | "strapi"
    )
}

/// Check if a framework slug is a Next.js wrapper (uses Next.js internals).
#[allow(dead_code)]
pub fn is_nextjs_wrapper(slug: &str) -> bool {
    matches!(slug, "blitzjs" | "payload")
}

/// Get presets that have dependencies (used for detection), sorted by priority.
#[allow(dead_code)]
pub fn detection_presets() -> impl Iterator<Item = &'static FrameworkPreset> {
    PRESETS.iter().filter(|p| !p.dependencies.is_empty())
}

/// Get declarative detection rules sorted by preset priority.
pub fn detection_rules() -> impl Iterator<Item = &'static FrameworkDetectionRule> {
    DETECTION_RULES.iter()
}
