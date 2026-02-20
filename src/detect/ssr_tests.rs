use super::ssr::*;

#[test]
fn non_ssr_framework_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(analyze_ssr(dir.path(), "astro").is_none());
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
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
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
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
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
    assert!(result.is_static_compatible);
    assert!(result.ssr_features.is_empty());
}
