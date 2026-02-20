use super::ssr::*;

#[test]
fn non_ssr_framework_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(analyze_ssr(dir.path(), "vite").is_none());
    assert!(analyze_ssr(dir.path(), "other").is_none());
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("middleware")));
}

#[test]
fn nextjs_api_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("pages/api")).unwrap();
    std::fs::write(dir.path().join("pages/api/hello.ts"), "export default fn").unwrap();
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("pages/api")));
}

#[test]
fn nextjs_route_handlers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app/api")).unwrap();
    std::fs::write(dir.path().join("app/api/route.ts"), "export async fn GET").unwrap();
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
    // getStaticProps is SSG — doesn't enable static compatibility on its own
    // (Next.js defaults to SSR, getStaticProps is just informational)
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
    // generateStaticParams is SSG — informational only
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
    // Block comment should be stripped — standalone not detected
    assert!(!result.ssr_features.iter().any(|f| f.contains("standalone")));
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
    let result = analyze_ssr(dir.path(), "nextjs").unwrap();
    // middleware overrides output: 'export' → not static compatible
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
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    assert!(result.ssr_features.iter().any(|f| f.contains("ssr: false")));
    // ssr: false with no server features → static compatible
    assert!(result.is_static_compatible);
}

#[test]
fn nuxt_server_api_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("server/api")).unwrap();
    std::fs::write(dir.path().join("server/api/hello.ts"), "export default").unwrap();
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("server/api")));
}

#[test]
fn nuxt_server_routes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("server/routes")).unwrap();
    std::fs::write(dir.path().join("server/routes/feed.ts"), "export default").unwrap();
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
}

#[test]
fn nuxt_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    // Nuxt defaults to SSR → not static compatible
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
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
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
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    assert!(!result.is_static_compatible);
    assert!(result.ssr_features.iter().any(|f| f.contains("routeRules")));
}

#[test]
fn nuxt_nitro_preset_static_no_false_positive() {
    let dir = tempfile::tempdir().unwrap();
    // "static" appears in a class name, not as a preset value
    std::fs::write(
        dir.path().join("nuxt.config.ts"),
        r#"export default defineNuxtConfig({
  app: { head: { bodyAttrs: { class: 'static-page' } } }
})"#,
    )
    .unwrap();
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    // Should NOT detect preset: 'static'
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
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    assert!(result.ssr_features.iter().any(|f| f.contains("preset")));
    // preset: 'static' → static compatible
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
    let result = analyze_ssr(dir.path(), "nuxt").unwrap();
    // server/api/ overrides ssr: false → not static compatible
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
    // SvelteKit defaults to SSR → not static compatible
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
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
    let result = analyze_ssr(dir.path(), "sveltekit").unwrap();
    // +server routes override adapter-static → not static compatible
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
    let result = analyze_ssr(dir.path(), "astro").unwrap();
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
    let result = analyze_ssr(dir.path(), "astro").unwrap();
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
    let result = analyze_ssr(dir.path(), "astro").unwrap();
    assert!(result.is_static_compatible);
}

#[test]
fn astro_clean_project() {
    let dir = tempfile::tempdir().unwrap();
    let result = analyze_ssr(dir.path(), "astro").unwrap();
    // Astro defaults to static
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
    let result = analyze_ssr(dir.path(), "astro").unwrap();
    assert!(!result.is_static_compatible);
    assert!(
        result
            .ssr_features
            .iter()
            .any(|f| f.contains("SSR adapter"))
    );
}
