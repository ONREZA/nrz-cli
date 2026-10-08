use super::fs::LocalFs;
use super::ssr::*;

#[test]
fn non_ssr_framework_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let fs = LocalFs::new(dir.path());
    assert!(analyze_ssr(&fs, "vite").is_none());
    assert!(analyze_ssr(&fs, "hono").is_none());
    assert!(analyze_ssr(&fs, "elysia").is_none());
    assert!(analyze_ssr(&fs, "other").is_none());
}

// ── Next.js ────────────────────────────────────────────────────

#[test]
fn nextjs_static_export() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = { output: 'export' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("export")));
}

#[test]
fn nextjs_standalone_mode() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.mjs"),
        "export default { output: 'standalone' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("standalone")));
}

#[test]
fn nextjs_middleware_detected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("middleware.ts"),
        "export function middleware() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("middleware")));
}

#[test]
fn nextjs_api_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("pages/api")).unwrap();
    std::fs::write(dir.path().join("pages/api/hello.ts"), "export default fn").unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("pages/api")));
}

#[test]
fn nextjs_route_handlers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/api")).unwrap();
    std::fs::write(dir.path().join("app/api/route.ts"), "export async fn GET").unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("route")));
}

#[test]
fn nextjs_gssp() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("pages")).unwrap();
    std::fs::write(
        dir.path().join("pages/index.tsx"),
        "export async function getServerSideProps() { return { props: {} } }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("getServerSideProps"))
    );
}

#[test]
fn nextjs_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    // Next.js defaults to SSR → not static compatible
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}

#[test]
fn nextjs_use_server_directive() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/actions")).unwrap();
    std::fs::write(
        dir.path().join("app/actions/submit.ts"),
        "\"use server\"\n\nexport async function submitForm() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("use server")));
}

#[test]
fn nextjs_revalidate() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/blog")).unwrap();
    std::fs::write(
        dir.path().join("app/blog/page.tsx"),
        "export const revalidate = 60;",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("revalidate")));
}

#[test]
fn nextjs_get_static_props() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("pages")).unwrap();
    std::fs::write(
        dir.path().join("pages/blog.tsx"),
        "export async function getStaticProps() { return { props: {} } }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    // getStaticProps is SSG — doesn't enable static compatibility on its own
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("getStaticProps"))
    );
}

#[test]
fn nextjs_generate_static_params() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/blog/[slug]")).unwrap();
    std::fs::write(
        dir.path().join("app/blog/[slug]/page.tsx"),
        "export function generateStaticParams() { return [] }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("generateStaticParams"))
    );
}

#[test]
fn nextjs_block_comment_ignored() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = {\n  /* output: 'standalone' */\n}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.ssr_features.iter().any(|f| f.contains("standalone")));
}

#[test]
fn nextjs_inline_comment_ignored() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = {\n  // output: 'standalone' // was active before\n  output: 'export', // deploy as static\n}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("export")));
    assert!(!result.ssr_features.iter().any(|f| f.contains("standalone")));
}

#[test]
fn inline_comment_respects_string_literals() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        r#"export default defineNuxtConfig({
  routeRules: {
    '/api/**': { proxy: 'http://localhost:3001/**' },
  }
})"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(result.ssr_features.iter().any(|f| f.contains("routeRules")));
}

// ── Next.js conflict scenarios ──────────────────────────────

#[test]
fn nextjs_export_with_middleware_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = { output: 'export' }",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("middleware.ts"),
        "export function middleware() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("export")));
    assert!(result.ssr_features.iter().any(|f| f.contains("middleware")));
}

// ── Nuxt ───────────────────────────────────────────────────────

#[test]
fn nuxt_ssr_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: false })",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(result.ssr_features.iter().any(|f| f.contains("ssr: false")));
    assert!(result.is_static_compatible);
}

#[test]
fn nuxt_server_api_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("server/api")).unwrap();
    std::fs::write(dir.path().join("server/api/hello.ts"), "export default").unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("server/api")));
}

#[test]
fn nuxt_server_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("server/routes")).unwrap();
    std::fs::write(dir.path().join("server/routes/feed.ts"), "export default").unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn nuxt_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}

#[test]
fn nuxt_server_middleware() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("server/middleware")).unwrap();
    std::fs::write(
        dir.path().join("server/middleware/auth.ts"),
        "export default",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("server/middleware"))
    );
}

#[test]
fn nuxt_route_rules_ssr() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        r#"export default defineNuxtConfig({
  routeRules: {
    '/api/**': { proxy: 'http://localhost:3001/**' },
    '/blog/**': { ssr: true },
  }
})"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("routeRules")));
}

#[test]
fn nuxt_nitro_preset_static_no_false_positive() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        r#"export default defineNuxtConfig({
  app: { head: { bodyAttrs: { class: 'static-page' } } }
})"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.ssr_features.iter().any(|f| f.contains("preset")));
}

#[test]
fn nuxt_nitro_preset_static_correct() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        r#"export default defineNuxtConfig({
  nitro: { preset: 'static' }
})"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(result.ssr_features.iter().any(|f| f.contains("preset")));
    assert!(result.is_static_compatible);
}

// ── Nuxt conflict scenarios ─────────────────────────────────

#[test]
fn nuxt_ssr_false_with_server_api_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: false })",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("server/api")).unwrap();
    std::fs::write(dir.path().join("server/api/hello.ts"), "export default").unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
}

// ── SvelteKit ──────────────────────────────────────────────────

#[test]
fn sveltekit_adapter_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-static';\nexport default { kit: { adapter: adapter() } }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("adapter-static"))
    );
}

#[test]
fn sveltekit_server_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes/api")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/api/+server.ts"),
        "export async function GET() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("+server")));
}

#[test]
fn sveltekit_hooks_server() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/hooks.server.ts"),
        "export const handle = ...",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("hooks.server"))
    );
}

#[test]
fn sveltekit_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}

#[test]
fn sveltekit_page_server() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/+page.server.ts"),
        "export async function load() { return {} }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("+page.server"))
    );
}

#[test]
fn sveltekit_layout_server() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/+layout.server.ts"),
        "export async function load() { return {} }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("+layout.server"))
    );
}

#[test]
fn sveltekit_form_actions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes/login")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/login/+page.server.ts"),
        "export const actions = { default: async ({ request }) => {} }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("form actions"))
    );
}

#[test]
fn sveltekit_adapter_node() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-node';\nexport default { kit: { adapter: adapter() } }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("adapter-node"))
    );
}

#[test]
fn sveltekit_adapter_auto() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-auto';\nexport default { kit: { adapter: adapter() } }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("adapter-auto"))
    );
}

// ── SvelteKit conflict scenarios ────────────────────────────

#[test]
fn sveltekit_adapter_static_with_server_routes_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-static';\nexport default { kit: { adapter: adapter() } }",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes/api")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/api/+server.ts"),
        "export async function GET() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
}

// ── Astro ──────────────────────────────────────────────────────

#[test]
fn astro_output_server() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("astro.config.mjs"),
        "import { defineConfig } from 'astro/config';\nexport default defineConfig({ output: 'server' })",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "astro").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("output: 'server'"))
    );
}

#[test]
fn astro_output_hybrid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("astro.config.mjs"),
        "import { defineConfig } from 'astro/config';\nexport default defineConfig({ output: 'hybrid' })",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "astro").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("hybrid")));
}

#[test]
fn astro_default_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("astro.config.mjs"),
        "import { defineConfig } from 'astro/config';\nexport default defineConfig({})",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "astro").unwrap();
    assert!(result.is_static_compatible);
}

#[test]
fn astro_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "astro").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}

#[test]
fn astro_ssr_adapter_in_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("astro.config.mjs"),
        r#"import { defineConfig } from 'astro/config';
import node from '@astrojs/node';
export default defineConfig({ output: 'server', adapter: node({ mode: 'standalone' }) })"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "astro").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("SSR adapter"))
    );
}

// ── React Router v7 ────────────────────────────────────────────

#[test]
fn react_router_default_is_ssr() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn react_router_spa_mode() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("react-router.config.ts"),
        r#"import type { Config } from "@react-router/dev/config";
export default { ssr: false } satisfies Config;"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("SPA mode")));
}

#[test]
fn react_router_route_loaders() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        r#"export async function loader() { return { data: [] }; }"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders"))
    );
}

#[test]
fn react_router_route_actions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/login.tsx"),
        r#"export async function action({ request }) { }"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("route actions"))
    );
}

#[test]
fn react_router_entry_server() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app")).unwrap();
    std::fs::write(
        dir.path().join("app/entry.server.tsx"),
        "export default function handleRequest() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("entry.server"))
    );
}

#[test]
fn react_router_spa_with_loaders_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("react-router.config.ts"),
        r#"export default { ssr: false };"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        r#"export async function loader() { return {}; }"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(!result.is_static_compatible);
}

// ── Remix ──────────────────────────────────────────────────────

#[test]
fn remix_default_is_ssr() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn remix_spa_mode() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"import { vitePlugin as remix } from "@remix-run/dev";
export default defineConfig({
  plugins: [remix({ ssr: false })],
})"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("SPA mode")));
}

#[test]
fn remix_route_loaders() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/_index.tsx"),
        r#"export async function loader() { return json({}); }"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders"))
    );
}

#[test]
fn remix_route_actions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/login.tsx"),
        r#"export async function action({ request }) { }"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("route actions"))
    );
}

#[test]
fn remix_entry_server() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app")).unwrap();
    std::fs::write(
        dir.path().join("app/entry.server.tsx"),
        "export default function handleRequest() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("entry.server"))
    );
}

#[test]
fn remix_spa_mode_with_loaders_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"import { vitePlugin as remix } from "@remix-run/dev";
export default defineConfig({
  plugins: [remix({ ssr: false })],
})"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/_index.tsx"),
        r#"export async function loader() { return json({}); }"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn remix_block_comment_ignored() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"export default defineConfig({
  plugins: [remix({ /* ssr: false */ })],
})"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(!result.is_static_compatible);
}

// ── file_has_exported_symbol edge cases ─────────────────────

#[test]
fn exported_symbol_in_line_comment_not_matched() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        "// TODO: export function loader() {}\nexport default function Home() { return <div/>; }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(
        !result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders")),
        "commented-out loader should not be detected"
    );
}

#[test]
fn exported_symbol_in_block_comment_not_matched() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        "/*\n * export async function loader() { return {}; }\n */\nexport default function Home() { return <div/>; }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(
        !result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders")),
        "loader inside block comment should not be detected"
    );
}

#[test]
fn symbol_without_export_not_matched() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        "function loader() { return {}; }\nexport default function Home() { return <div/>; }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(
        !result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders")),
        "non-exported loader should not be detected"
    );
}

#[test]
fn export_const_loader_matched() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        "export const loader = async () => { return {}; };",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders"))
    );
}

#[test]
fn re_export_loader_matched() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        "export { loader } from './home.server';",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders"))
    );
}

#[test]
fn substring_loader_not_matched() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        "export const preloader = () => {};\nexport default function Home() { return <div/>; }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(
        !result
            .ssr_features
            .iter()
            .any(|f| f.contains("route loaders")),
        "preloader should not be matched as loader"
    );
}

#[test]
fn substring_action_not_matched() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/routes")).unwrap();
    std::fs::write(
        dir.path().join("app/routes/home.tsx"),
        "export const reactionState = {};\nexport default function Home() { return <div/>; }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "react-router").unwrap();
    assert!(
        !result
            .ssr_features
            .iter()
            .any(|f| f.contains("route actions")),
        "reactionState should not be matched as action"
    );
}

// ── SolidStart ──────────────────────────────────────────────────

#[test]
fn solidstart_default_is_ssr() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "solidstart").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn solidstart_ssr_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.config.ts"),
        r#"import { defineConfig } from "@solidjs/start/config";
export default defineConfig({ ssr: false });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "solidstart").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("ssr: false")));
}

#[test]
fn solidstart_api_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes/api")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/api/hello.ts"),
        "export function GET() { return new Response('ok'); }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "solidstart").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("src/routes/api"))
    );
}

#[test]
fn solidstart_use_server() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/lib")).unwrap();
    std::fs::write(
        dir.path().join("src/lib/actions.ts"),
        "\"use server\";\nexport async function submitForm() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "solidstart").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("use server")));
}

#[test]
fn solidstart_ssr_false_with_api_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.config.ts"),
        r#"import { defineConfig } from "@solidjs/start/config";
export default defineConfig({ ssr: false });"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes/api")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/api/data.ts"),
        "export function GET() {}",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "solidstart").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn solidstart_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "solidstart").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}

#[test]
fn solidstart_block_comment_ignored() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.config.ts"),
        r#"import { defineConfig } from "@solidjs/start/config";
export default defineConfig({ /* ssr: false */ });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "solidstart").unwrap();
    assert!(!result.is_static_compatible);
    assert!(!result.ssr_features.iter().any(|f| f.contains("ssr: false")));
}

// ── Qwik City ───────────────────────────────────────────────────

#[test]
fn qwik_default_is_ssr() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn qwik_static_adaptor() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"import { qwikCity } from "@builder.io/qwik-city/vite";
import staticAdapter from "@builder.io/qwik-city/adaptors/static/vite";
export default defineConfig({ plugins: [qwikCity(), staticAdapter()] });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("static adaptor"))
    );
}

#[test]
fn qwik_static_adaptor_new_package() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"import { qwikRouter } from "@qwik.dev/router/vite";
import staticAdapter from "@qwik.dev/router/adaptors/static/vite";
export default defineConfig({ plugins: [qwikRouter(), staticAdapter()] });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(result.is_static_compatible);
}

#[test]
fn qwik_route_loader() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/index.tsx"),
        r#"import { routeLoader$ } from "@builder.io/qwik-city";
export const useData = routeLoader$(() => { return { items: [] }; });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("routeLoader$"))
    );
}

#[test]
fn qwik_route_action() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes/login")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/login/index.tsx"),
        r#"import { routeAction$ } from "@builder.io/qwik-city";
export const useLogin = routeAction$((data) => { });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("routeAction$"))
    );
}

#[test]
fn qwik_server_function() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/lib")).unwrap();
    std::fs::write(
        dir.path().join("src/lib/api.ts"),
        r#"import { server$ } from "@builder.io/qwik-city";
export const fetchData = server$(() => { });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("server$ functions"))
    );
}

#[test]
fn qwik_static_with_route_loader_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"import staticAdapter from "@builder.io/qwik-city/adaptors/static/vite";
export default defineConfig({ plugins: [staticAdapter()] });"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/index.tsx"),
        r#"import { routeLoader$ } from "@builder.io/qwik-city";
export const useData = routeLoader$(() => ({}));"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn qwik_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "qwik").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}

// ── Analog ──────────────────────────────────────────────────────

#[test]
fn analog_default_is_ssr() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "analog").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn analog_ssr_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"import { defineConfig } from "vite";
import analog from "@analogjs/platform";
export default defineConfig({ plugins: [analog({ ssr: false })] });"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "analog").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("ssr: false")));
}

#[test]
fn analog_server_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/server/routes")).unwrap();
    std::fs::write(
        dir.path().join("src/server/routes/hello.ts"),
        "export default defineEventHandler(() => ({ message: 'hello' }));",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "analog").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("src/server/routes"))
    );
}

#[test]
fn analog_server_api() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/server/api")).unwrap();
    std::fs::write(
        dir.path().join("src/server/api/data.ts"),
        "export default defineEventHandler(() => []);",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "analog").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("src/server/api"))
    );
}

#[test]
fn analog_ssr_false_with_server_routes_is_not_static() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"import analog from "@analogjs/platform";
export default defineConfig({ plugins: [analog({ ssr: false })] });"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src/server/routes")).unwrap();
    std::fs::write(
        dir.path().join("src/server/routes/api.ts"),
        "export default defineEventHandler(() => ({}));",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "analog").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn analog_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "analog").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}

// ── Config comments and quoting (tested via analyze_ssr) ──

#[test]
fn nextjs_inline_comment_with_escaped_quotes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        r#"module.exports = { output: "export" } // deploy as static"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(result.is_static_compatible);
}

#[test]
fn nuxt_backtick_string_with_slashes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({\n  devServer: { url: `http://localhost:3000` },\n  ssr: false\n})",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(result.is_static_compatible);
}

#[test]
fn nextjs_no_comment_line_unmodified() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.mjs"),
        "export default { output: 'standalone' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("standalone")));
}

// ── P3.4: Improved SSR analysis ─────────────────────────────────

#[test]
fn sveltekit_ts_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("svelte.config.ts"),
        r#"import adapter from '@sveltejs/adapter-static';
export default { kit: { adapter: adapter() } };"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("adapter-static"))
    );
}

#[test]
fn sveltekit_ts_config_node_adapter() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("svelte.config.ts"),
        r#"import adapter from '@sveltejs/adapter-node';
export default { kit: { adapter: adapter() } };"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "sveltekit").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("adapter-node"))
    );
}

#[test]
fn remix_legacy_config_ssr_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("remix.config.js"),
        r#"/** @type {import('@remix-run/dev').AppConfig} */
module.exports = { ssr: false };"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("legacy remix.config"))
    );
}

#[test]
fn remix_vite_config_takes_precedence_over_legacy() {
    let dir = tempfile::tempdir().unwrap();
    // Vite config with ssr: false
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"export default defineConfig({ plugins: [remix({ ssr: false })] });"#,
    )
    .unwrap();
    // Legacy config also present — vite should take precedence
    std::fs::write(
        dir.path().join("remix.config.js"),
        r#"module.exports = {};"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("SPA mode")));
}

// ── Next.js wrappers (Blitz.js, Payload CMS) ──────────────────

#[test]
fn blitzjs_uses_nextjs_ssr_analysis() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = { output: 'standalone' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "blitzjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.has_standalone_output());
}

#[test]
fn payload_uses_nextjs_ssr_analysis() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.mjs"),
        "export default { output: 'standalone' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "payload").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.has_standalone_output());
}

#[test]
fn payload_no_config_defaults_to_ssr() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "payload").unwrap();
    assert!(!result.is_static_compatible);
}

// ── TanStack Start (Vinxi) SSR analysis ───────────────────────

#[test]
fn tanstack_start_defaults_to_ssr() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "tanstack-start").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn tanstack_start_detects_server_functions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/routes")).unwrap();
    std::fs::write(
        dir.path().join("src/routes/index.tsx"),
        "import { createServerFn } from '@tanstack/react-start'",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "tanstack-start").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("server functions"))
    );
}

// ── Hydrogen SSR analysis ─────────────────────────────────────

#[test]
fn hydrogen_uses_react_router_ssr_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "hydrogen").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn remix_vite_exists_ignores_legacy_ssr_false() {
    let dir = tempfile::tempdir().unwrap();
    // Vite config present but without ssr: false (SSR enabled)
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"export default defineConfig({ plugins: [remix()] });"#,
    )
    .unwrap();
    // Legacy config has ssr: false — should be IGNORED since vite config exists
    std::fs::write(
        dir.path().join("remix.config.js"),
        r#"module.exports = { ssr: false };"#,
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "remix").unwrap();
    assert!(
        !result.is_static_compatible,
        "legacy remix.config.js should be ignored when vite config exists"
    );
}

#[test]
fn nextjs_env_fallback_standalone_is_unknown() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = { output: process.env.NEXT_OUTPUT || 'standalone' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(!result.ssr_features.iter().any(|f| f.contains("standalone")));
}

#[test]
fn nextjs_env_nullish_coalescing_export_is_unknown() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.mjs"),
        "export default { output: process.env.NEXT_OUTPUT ?? 'export' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(!result.ssr_features.iter().any(|f| f.contains("export")));
}

#[test]
fn nextjs_backtick_quoted_standalone() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = { output: `standalone` }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("standalone")));
}

#[test]
fn nuxt_env_fallback_ssr_false_is_unknown() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: process.env.NUXT_SSR ?? false })",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn nuxt_nonliteral_expressions_require_ssr() {
    for (expression, is_static) in [
        ("(process.env.NUXT_SSR ?? false)", false),
        ("((process.env.NUXT_SSR ?? false))", false),
        ("process.env.NUXT_SSR??false", false),
        ("process.env.NUXT_SSR||false", false),
        ("Boolean({ enabled: true, other: false })??false", false),
        ("Boolean([true, false])||false", false),
        ("Boolean(flag||false)", false),
        ("Boolean(flag??false)", false),
        ("process.env.NUXT_SSR | false", false),
        ("process.env.NUXT_SSR ? false : true", false),
        ("flag)", false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("nuxt.config.ts"),
            format!("export default defineNuxtConfig({{ ssr: {expression} }})"),
        )
        .unwrap();

        let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();

        assert_eq!(
            result.is_static_compatible, is_static,
            "expression: {expression}"
        );
    }
}

#[test]
fn ssr_settings_match_properties_and_complete_literals() {
    let mut failures = Vec::new();
    for (framework, properties, is_static) in [
        ("nuxt", "css: ['ssr.css'], ssr: false", true),
        ("nuxt", "no_ssr: false, ssr: true", false),
        ("nuxt", "note: 'ssr: false ', ssr: true", false),
        ("nuxt", "note: `🦊 ssr: false `, ssr: true", false),
        ("nuxt", "ssr: false || true", false),
        ("nuxt", "ssr: flag || false || true", false),
        ("nuxt", "ssr: false as const", true),
        ("nuxt", "ssr: flag ?? false as const", false),
        ("nuxt", "\"ssr\": false", true),
        ("nuxt", "'ssr': false", true),
        ("nuxt", "ssr: (false)", true),
        ("nuxt", "ssr: (false as const)", true),
        ("nuxt", "ssr: (false) as const", true),
        ("nuxt", "ssr: false // ignored || true\n", true),
        ("nuxt", "ssr: false /* 🦊 */ || true", false),
        ("nextjs", "output: 'export' + 'foo'", false),
        ("nextjs", "'output': 'export' as const", true),
        ("astro", "output: 'server' as const", false),
        ("astro", "\"output\": ('server' as const)", false),
    ] {
        let config_name = if framework == "nextjs" {
            "next"
        } else {
            framework
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(format!("{config_name}.config.ts")),
            format!("export default {{ {properties} }}"),
        )
        .unwrap();

        let result = analyze_ssr(&LocalFs::new(dir.path()), framework).unwrap();
        if result.is_static_compatible != is_static {
            failures.push(properties);
        }
    }
    assert!(failures.is_empty(), "incorrect SSR settings: {failures:?}");
}

#[test]
fn ssr_config_scans_multiline_properties_without_reading_strings_as_code() {
    let mut failures = Vec::new();
    for (properties, is_static) in [
        ("note: `hello\nssr: false\nbye`, ssr: true", false),
        ("note: '${flag}', ssr: false", true),
        ("note: `$flag`, ssr: false", true),
        ("ssr:\nfalse", true),
        ("ssr: false\n || true", false),
        ("ssr /* 🦊 */ : // comment\n false", true),
        ("\"ssr\" // comment\n : /* comment */ false", true),
        ("ssr: false /* comment */ as  const", true),
        ("ssr: false\tas\tconst", true),
        ("ssr: false as boolean", true),
        ("ssr: false as unknown as boolean", true),
        ("ssr: false as unknown as boolean || true", false),
        ("ssr: false satisfies boolean", true),
        ("ssr: false as boolean || true", false),
        ("ssr: // note\u{2028} false", true),
        ("ssr: // note\u{2029} false", true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("nuxt.config.ts"),
            format!("export default defineNuxtConfig({{ {properties} }})"),
        )
        .unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
        if result.is_static_compatible != is_static {
            failures.push(properties);
        }
    }
    assert!(failures.is_empty(), "incorrect SSR settings: {failures:?}");
}

#[test]
fn nuxt_quoted_delimiters_do_not_make_dynamic_values_static() {
    for expression in [
        r#"(process.env["),} || ??"] ?? false)"#,
        r#"((process.env['\'),} || ??'] || false))"#,
        r#"(process.env[`\`),} || ??`] ?? false)"#,
        r#"(process.env["\"),} || ??"] || false)"#,
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("nuxt.config.ts"),
            format!("export default defineNuxtConfig({{ ssr: {expression}, featureFlag: true }})"),
        )
        .unwrap();

        let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();

        assert!(!result.is_static_compatible, "expression: {expression}");
    }
}

#[test]
fn nuxt_unknown_or_ambiguous_exported_config_requires_ssr() {
    let deep_config = format!(
        "export default {{ ssr: {}false{} }}",
        "(".repeat(512),
        ")".repeat(512)
    );
    for config in [
        "const note = `${flag}`; export default { ssr: false }",
        "export default custom({ ssr: false })",
        "export default { ssr: 'false' }",
        "export default { ssr: `false` }",
        "export default { ssr: false, ssr: false }",
        "export default { ssr: false, ssr: flag }",
        "export default { ssr: false, ...options }",
        "export default { ssr: true, feature: { ssr: false } }",
        "export default { ssr: flag, nitro: { preset: 'static' } }",
        "export default { ssr: false, nitro: { preset: flag } }",
        "export default { ssr: false, nitro: { ...options } }",
        "import adapter from '@sveltejs/adapter-static'; export default { ssr: adapter(), nitro: { preset: 'static' } }",
        "import adapter from '@sveltejs/adapter-static'; export default { ssr: false, nitro: { preset: adapter() } }",
        "export default { note: /, ssr: false, / }",
        "export default { ssr: false as any in { false: true } }",
        "export default { ssr: false as any < true }",
        "export default { ssr: false as any >= false }",
        "export default { ssr: false as any ^ true }",
        "export default { ssr: false as App.1 }",
        "export default { ssr: false as Array<boolean true }",
        "export default { ssr: false satisfies boolean ? true : false }",
        "export default { ssr: (false as any).constructor(1) }",
        "export default { ssr: false,, }",
        "export default { ssr: false, other: }",
        "export default { ssr: false, other: 1; }",
        "export default { ssr: false",
        "export default defineNuxtConfig({ ssr: false }, options)",
        "export default defineNuxtConfig(defineNuxtConfig({ ssr: false }))",
        "const config = { ssr: false }; export default config",
        "const example = { ssr: false }; export default { ssr: flag }",
        "if (flag) module.exports = { ssr: false }",
        "while (false) module.exports = { ssr: false }",
        "holder.module.exports = { ssr: false }",
        "const module = {}; module.exports = { ssr: false }",
        "import module from '@sveltejs/adapter-static'; module.exports = { ssr: false }",
        "const { module } = holder; module.exports = { ssr: false }",
        "const { defineNuxtConfig } = holder; export default defineNuxtConfig({ ssr: false })",
        "const { defineConfig } = holder; export default defineConfig({ ssr: false })",
        "const note = 1, defineConfig = options => ({ ssr: true }); export default defineConfig({ ssr: false })",
        "defineNuxtConfig = options => ({ ssr: true }); export default defineNuxtConfig({ ssr: false })",
        "import { defineConfig } from 'unrecognized-package'; export default defineConfig({ ssr: false })",
        "import * as defineConfig from 'vite'; export default defineConfig({ ssr: false })",
        "import { mergeConfig as defineConfig } from 'vite'; export default defineConfig({ ssr: false })",
        "import defineConfig from 'vite'; export default defineConfig({ ssr: false })",
        "import { type defineConfig as defineConfig } from 'vite'; export default defineConfig({ ssr: false })",
        "import { fake as defineNuxtConfig } from 'nuxt/config'; export default defineNuxtConfig({ ssr: false })",
        "import { defineNuxtConfig } from 'other-module'; export default defineNuxtConfig({ ssr: false })",
        "import 'unrelated'; const defineNuxtConfig = config => ({ ssr: true }); export default defineNuxtConfig({ ssr: false })",
        "const ssr = true; export default { ssr }",
        "module.exports = { ssr: false }; module.exports.ssr = true",
        "module.exports = { ssr: false }; Object.assign(module.exports, { ssr: true })",
    ].into_iter().chain(std::iter::once(deep_config.as_str())) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("nuxt.config.ts"), config).unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
        assert!(!result.is_static_compatible, "config: {config}");
    }
}

#[test]
fn local_plugin_binding_is_not_trusted_as_framework_configuration() {
    for (framework, plugin) in [("analog", "analog"), ("remix", "remix")] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vite.config.ts"),
            format!("const {plugin} = options => ({{ ssr: true }}); export default defineConfig({{ plugins: [{plugin}({{ ssr: false }})] }})"),
        )
        .unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), framework).unwrap();
        assert!(!result.is_static_compatible, "framework: {framework}");
    }
}

#[test]
fn nuxt_imports_and_unrelated_shorthand_preserve_static_settings() {
    for config in [
        "import { defineNuxtConfig } from 'nuxt/config'; export default defineNuxtConfig({ ssr: false })",
        "import { defineNuxtConfig as ignored } from 'other-module'; import { defineNuxtConfig } from 'nuxt/config'; export default defineNuxtConfig({ ssr: false })",
        "import 'unrelated'; export default defineNuxtConfig({ ssr: false })",
        "type as = boolean; export default { ssr: (false as as) as boolean }",
        "const other = 0; export default { ssr: false, other }",
        "const other = 0; export default { other, ssr: false }",
        "export default { nitro: { preset: 'static' }, routeRules: { '/': { headers: { hello: 'x' } } } }",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("nuxt.config.ts"), config).unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
        assert!(result.is_static_compatible, "config: {config}");
    }
}

#[test]
fn nuxt_config_nesting_accepts_128_levels_and_declines_129() {
    for depth in [128, 129] {
        let unary = "keyof ".repeat(depth - 1);
        let compound = "keyof ".repeat(depth - 2);
        for (form, config) in [
            (
                "balanced arrays",
                format!(
                    "export default {{ ssr: false, other: {}0{} }}",
                    "[".repeat(depth - 1),
                    "]".repeat(depth - 1)
                ),
            ),
            (
                "unary type",
                format!("export default {{ ssr: false as unknown as {unary}any }}"),
            ),
            (
                "grouped type",
                format!("export default {{ ssr: false as unknown as {compound}(any) }}"),
            ),
            (
                "tuple type",
                format!("export default {{ ssr: false as unknown as {compound}[any] }}"),
            ),
            (
                "generic type",
                format!("export default {{ ssr: false as unknown as {compound}Array<any> }}"),
            ),
            (
                "indexed type",
                format!(
                    "type App = {{ 0: boolean }}; export default {{ ssr: false as unknown as {compound}App[0] }}"
                ),
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("nuxt.config.ts"), config).unwrap();
            let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
            assert_eq!(
                result.is_static_compatible,
                depth == 128,
                "{form}: depth {depth}"
            );
        }
    }
}

#[test]
fn framework_plugin_settings_require_an_owned_literal_array() {
    for (framework, prefix, plugins, is_static) in [
        ("remix", "", "[remix({ ssr: false }),]", true),
        (
            "remix",
            "",
            "[remix({ ssr: false, appDirectory: 'app' })]",
            true,
        ),
        (
            "remix",
            "const extras = { disabled: false };",
            "[extras.disabled, remix({ ssr: false })]",
            true,
        ),
        ("remix", "", "foo([remix({ ssr: false })])", false),
        ("remix", "", "(remix({ ssr: false }))", false),
        (
            "remix",
            "",
            "[...runtimePlugins, remix({ ssr: false })]",
            false,
        ),
        (
            "analog",
            "",
            "[...runtimePlugins, analog({ ssr: false })]",
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vite.config.ts"),
            format!("{prefix} export default defineConfig({{ plugins: {plugins} }})"),
        )
        .unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), framework).unwrap();
        assert_eq!(
            result.is_static_compatible, is_static,
            "{framework} plugins: {plugins}"
        );
    }
}

#[test]
fn nuxt_type_assertions_keep_complete_boolean_literals_static() {
    for expression in [
        "(false as const) as boolean",
        "false as []",
        "false as unknown as []",
        "false as boolean | true",
        "false satisfies Config['ssr']",
        "false as Namespace.Type",
        "false as Array<boolean>",
        "false as [boolean, 'ssr']",
        "false as (boolean & Unknown)",
        "false as readonly boolean[]",
        "false as keyof Config",
        "false as typeof flag",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("nuxt.config.ts"),
            format!("export default {{ ssr: {expression} }}"),
        )
        .unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
        assert!(result.is_static_compatible, "expression: {expression}");
    }
}

#[test]
fn unreadable_preferred_config_requires_ssr_without_selecting_lower_priority_static_config() {
    for (framework, preferred, fallback) in [
        ("astro", "astro.config.mjs", None),
        (
            "astro",
            "astro.config.mjs",
            Some(("astro.config.js", "export default { output: 'static' }")),
        ),
        (
            "remix",
            "vite.config.ts",
            Some(("remix.config.js", "module.exports = { ssr: false }")),
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(preferred), [0xff]).unwrap();
        if let Some((file, content)) = fallback {
            std::fs::write(dir.path().join(file), content).unwrap();
        }
        let result = analyze_ssr(&LocalFs::new(dir.path()), framework).unwrap();
        assert!(!result.is_static_compatible, "{framework}: {fallback:?}");
    }
}

#[test]
fn astro_only_defaults_to_static_when_exported_output_is_absent_or_literal_static() {
    for (config, is_static) in [
        ("export default {}", true),
        ("export default []", false),
        ("export default defineConfig({ output: 'static' })", true),
        ("export default { output: 'server' as  string }", false),
        ("export default { output: mode ?? 'static' }", false),
        (
            "export default { // note\u{2028} output: 'server'\n }",
            false,
        ),
        (
            "export default { // note\u{2029} output: 'server'\n }",
            false,
        ),
        ("export default { ...options }", false),
        ("export default { ['output']: 'server' }", false),
        (r#"export default { 'out\u0070ut': 'server' }"#, false),
        (r#"export default { \u006futput: 'server' }"#, false),
        (
            "const config = { output: 'static' }; export default config",
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("astro.config.ts"), config).unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), "astro").unwrap();
        assert_eq!(result.is_static_compatible, is_static, "config: {config}");
    }
}

#[test]
fn unsupported_templates_do_not_reintroduce_commented_static_adapters() {
    for (framework, file, config) in [
        (
            "sveltekit",
            "svelte.config.js",
            "/* adapter-static */ import adapter from '@sveltejs/adapter-node'; const note = `hello ${flag}`; export default { kit: { adapter: adapter() } };",
        ),
        (
            "qwik",
            "vite.config.ts",
            "/* @builder.io/qwik-city/static */ const note = `hello ${flag}`; export default defineConfig({ plugins: [] });",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(file), config).unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), framework).unwrap();
        assert!(!result.is_static_compatible, "config: {config}");
    }
}

#[test]
fn static_adapter_imports_require_an_applied_adapter_in_exported_config() {
    for (framework, file, config, is_static) in [
        (
            "sveltekit",
            "svelte.config.js",
            "import from from '@sveltejs/adapter-static'; export default { kit: { adapter: from() } };",
            true,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import module from '@sveltejs/adapter-static'; export default { kit: { adapter: module() } };",
            true,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import staticAdapter from '@sveltejs/adapter-static'; import node from '@sveltejs/adapter-node'; export default { kit: { adapter: node() } };",
            false,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import staticAdapter from '@sveltejs/adapter-static'; export default { kit: {} };",
            false,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import customStatic from '@sveltejs/adapter-static'; export default { kit: { adapter: customStatic() } };",
            true,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import { default as customStatic } from '@sveltejs/adapter-static'; export default { kit: { adapter: customStatic() } };",
            true,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import adapter from '@sveltejs/adapter-static'; export default { kit: { adapter: flag ? adapter() : node() } };",
            false,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import adapter from '@sveltejs/adapter-static'; export default { kit: { adapter: adapter[0] } };",
            false,
        ),
        (
            "sveltekit",
            "svelte.config.js",
            "import adapter from '@sveltejs/adapter-static'; export default { kit: { adapter: adapter() || fallback } };",
            false,
        ),
        (
            "qwik",
            "vite.config.ts",
            "import staticAdapter from '@builder.io/qwik-city/adaptors/static/vite'; import { qwikCity } from '@builder.io/qwik-city/vite'; export default defineConfig({ plugins: [qwikCity()] });",
            false,
        ),
        (
            "qwik",
            "vite.config.ts",
            "import adapter from '@qwik.dev/router/adaptors/static/vite'; export default defineConfig({ plugins: [adapter()] });",
            true,
        ),
        (
            "qwik",
            "vite.config.ts",
            "import { default as adapter } from '@qwik.dev/router/adaptors/static/vite'; export default defineConfig({ plugins: [adapter()] });",
            true,
        ),
        (
            "qwik",
            "vite.config.ts",
            "import adapter from '@qwik.dev/router/adaptors/static/vite'; export default defineConfig({ plugins: [flag ? adapter() : other()] });",
            false,
        ),
        (
            "qwik",
            "vite.config.ts",
            "import adapter from '@qwik.dev/router/adaptors/static/vite'; export default defineConfig({ plugins: [...options, adapter()] });",
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(file), config).unwrap();
        let result = analyze_ssr(&LocalFs::new(dir.path()), framework).unwrap();
        assert_eq!(result.is_static_compatible, is_static, "config: {config}");
    }
}

#[test]
fn nuxt_grouped_nested_operator_is_not_a_direct_fallback() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: (process.env.NUXT_SSR ?? (featureFlag || false)), featureFlag: true })",
    )
    .unwrap();

    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();

    assert!(!result.is_static_compatible);
}

#[test]
fn nuxt_unrelated_false_property_is_not_ssr_fallback() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: process.env.NUXT_SSR, featureFlag: false })",
    )
    .unwrap();

    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();

    assert!(!result.is_static_compatible);
}

#[test]
fn nuxt_nested_fallback_does_not_consume_the_next_property() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: process.env.NUXT_SSR ?? Boolean({ enabled: true }.enabled), featureFlag: false })",
    )
    .unwrap();

    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();

    assert!(!result.is_static_compatible);
}

#[test]
fn nuxt_nested_operator_is_not_treated_as_direct_fallback() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: process.env.NUXT_SSR ?? (featureFlag || false), featureFlag: true })",
    )
    .unwrap();

    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();

    assert!(!result.is_static_compatible);
}
