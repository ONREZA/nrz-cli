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

// ── strip_inline_comment edge cases (tested via analyze_ssr) ──

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
fn nextjs_env_fallback_standalone() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.js"),
        "module.exports = { output: process.env.NEXT_OUTPUT || 'standalone' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("standalone")));
}

#[test]
fn nextjs_env_nullish_coalescing_export() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("next.config.mjs"),
        "export default { output: process.env.NEXT_OUTPUT ?? 'export' }",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nextjs").unwrap();
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("export")));
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
fn nuxt_env_fallback_ssr_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        "export default defineNuxtConfig({ ssr: process.env.NUXT_SSR ?? false })",
    )
    .unwrap();
    let result = analyze_ssr(&LocalFs::new(dir.path()), "nuxt").unwrap();
    assert!(result.is_static_compatible);
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
