use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use assert_cmd::Command as AssertCommand;

const PNG_1X1: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4, 0,
    0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 252, 255, 31, 0, 3, 3, 2, 0,
    239, 191, 167, 222, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn conformance_enabled() -> bool {
    std::env::var("NRZ_NEXTJS_CONFORMANCE")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn write_bytes(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn run_checked(label: &str, command: &mut ProcessCommand) {
    let output = command.output().unwrap_or_else(|error| {
        panic!("failed to start {label}: {error}");
    });
    assert!(
        output.status.success(),
        "{label} failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn npm_install(project: &Path) {
    run_checked(
        "npm install",
        ProcessCommand::new("npm")
            .arg("install")
            .arg("--no-audit")
            .arg("--no-fund")
            .arg("--loglevel=error")
            .current_dir(project),
    );
}

fn run_next_build_with_adapter(project: &Path, adapter_path: &Path) {
    run_checked(
        "next build with adapter",
        ProcessCommand::new("npm")
            .arg("run")
            .arg("build")
            .env("CI", "1")
            .env("NEXT_TELEMETRY_DISABLED", "1")
            .env("NEXT_ADAPTER_PATH", adapter_path)
            .env("ONREZA_NEXT_ADAPTER_VERSION", env!("CARGO_PKG_VERSION"))
            .current_dir(project),
    );
}

fn run_next_build_legacy(project: &Path) {
    run_checked(
        "next build legacy standalone",
        ProcessCommand::new("npm")
            .arg("run")
            .arg("build")
            .env("CI", "1")
            .env("NEXT_TELEMETRY_DISABLED", "1")
            .env("NEXT_PRIVATE_STANDALONE", "1")
            .env_remove("NEXT_ADAPTER_PATH")
            .env_remove("ONREZA_NEXT_ADAPTER_VERSION")
            .current_dir(project),
    );
}

fn run_nrz_build_json(project: &Path) -> serde_json::Value {
    let assert = AssertCommand::cargo_bin("nrz")
        .unwrap()
        .current_dir(project)
        .args(["build", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(stdout.trim()).unwrap()
}

fn read_descriptor(project: &Path) -> serde_json::Value {
    serde_json::from_str(
        &fs::read_to_string(project.join(".onreza/next-adapter-output.json")).unwrap(),
    )
    .unwrap()
}

fn next16_version() -> String {
    std::env::var("NRZ_NEXTJS_CONFORMANCE_NEXT_VERSION").unwrap_or_else(|_| "16.2.10".into())
}

fn write_next16_package(project: &Path, next_version: &str, esm: bool) {
    let module_type = if esm { "\n  \"type\": \"module\"," } else { "" };
    write(
        &project.join("package.json"),
        &format!(
            r#"{{
  "private": true,{module_type}
  "scripts": {{ "build": "next build" }},
  "dependencies": {{
    "next": "{next_version}",
    "react": "19.2.1",
    "react-dom": "19.2.1"
  }}
}}
"#
        ),
    );
}

fn write_legacy_package(project: &Path, next_version: &str) {
    let react_version = if next_version.starts_with("16") {
        "19.2.1"
    } else {
        "18.3.1"
    };
    write(
        &project.join("package.json"),
        &format!(
            r#"{{
  "private": true,
  "scripts": {{ "build": "next build" }},
  "dependencies": {{
    "next": "{next_version}",
    "react": "{react_version}",
    "react-dom": "{react_version}"
  }}
}}
"#
        ),
    );
}

fn has_route_with_source(routes: &serde_json::Value, bucket: &str, source: &str) -> bool {
    routes
        .get(bucket)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|route| {
                route.get("source").and_then(serde_json::Value::as_str) == Some(source)
            })
        })
}

fn contains_file_named(root: &Path, name: &str) -> bool {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file()
                && path.file_name().and_then(|value| value.to_str()) == Some(name)
            {
                return true;
            }
        }
    }
    false
}

#[test]
fn nextjs_16_2_adapter_conformance_builds_descriptor_and_cli_report() {
    if !conformance_enabled() {
        eprintln!("skipping Next.js conformance test; set NRZ_NEXTJS_CONFORMANCE=1 to run it");
        return;
    }

    let project = tempfile::tempdir().unwrap();
    let next_version = next16_version();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adapter_path = manifest_dir.join("assets/next-adapter/onreza-next-adapter.cjs");

    write_next16_package(project.path(), &next_version, true);
    write(
        &project.path().join("next.config.mjs"),
        r#"
/** @type {import('next').NextConfig} */
const nextConfig = {
  basePath: '/docs',
  trailingSlash: true,
  async redirects() {
    return [
      { source: '/old/:slug', destination: '/blog/:slug', permanent: false },
      { source: '/gone', destination: '/new-home', permanent: true },
    ]
  },
  async rewrites() {
    return {
      beforeFiles: [
        {
          source: '/rewrite-header/:slug',
          has: [{ type: 'header', key: 'x-fixture-rewrite', value: 'yes' }],
          missing: [{ type: 'header', key: 'x-fixture-skip', value: '1' }],
          destination: '/rewritten/:slug',
        },
        {
          source: '/rewrite-query/:slug',
          has: [{ type: 'query', key: 'preview', value: '1' }],
          destination: '/rewritten/:slug',
        },
      ],
      afterFiles: [
        { source: '/legacy/:path*', destination: '/modern/:path*' },
        { source: '/plain-http/:path*', destination: 'http://127.0.0.1:3001/:path*' },
      ],
      fallback: [],
    }
  },
  async headers() {
    return [{
      source: '/headers/:path*',
      headers: [{ key: 'x-next-config-header', value: 'yes' }],
    }]
  },
}

export default nextConfig
"#,
    );
    write(
        &project.path().join("app/layout.js"),
        r#"
export const metadata = { title: 'ONREZA Next adapter conformance' }

export default function RootLayout({ children }) {
  return <html><body>{children}</body></html>
}
"#,
    );
    write(
        &project.path().join("app/page.js"),
        r#"
import Image from 'next/image'

export default function Page() {
  return <main>home <Image src="/pixel.png" width={1} height={1} alt="pixel" /></main>
}
"#,
    );
    write(
        &project.path().join("app/blog/[slug]/page.js"),
        r#"
export default async function Page({ params }) {
  const { slug } = await params
  return <main>blog {slug}</main>
}
"#,
    );
    write(
        &project.path().join("app/rewritten/[slug]/page.js"),
        r#"
export default async function Page({ params }) {
  const { slug } = await params
  return <main>rewritten {slug}</main>
}
"#,
    );
    write(
        &project.path().join("app/modern/[...path]/page.js"),
        r#"
export default async function Page({ params }) {
  const { path } = await params
  return <main>modern {path.join('/')}</main>
}
"#,
    );
    write(
        &project.path().join("app/headers/[...path]/page.js"),
        r#"
export default function Page() {
  return <main>headers page</main>
}
"#,
    );
    write(
        &project.path().join("app/isr/page.js"),
        r#"
export const revalidate = 60

export default function Page() {
  return <main>isr</main>
}
"#,
    );
    write(
        &project.path().join("app/robots.js"),
        r#"
export default function robots() {
  return { rules: [{ userAgent: '*', allow: '/' }] }
}
"#,
    );
    write(
        &project.path().join("app/sitemap.js"),
        r#"
export default function sitemap() {
  return [{ url: 'https://example.com/', lastModified: new Date('2026-01-01') }]
}
"#,
    );
    write(
        &project.path().join("app/api/ping/route.js"),
        r#"
export async function GET() {
  return Response.json({ ok: true })
}
"#,
    );
    write(
        &project.path().join("middleware.js"),
        r#"
import { NextResponse } from 'next/server'

export function middleware() {
  return NextResponse.next()
}

export const config = {
  matcher: ['/private/:path*'],
}
"#,
    );
    write_bytes(&project.path().join("public/pixel.png"), PNG_1X1);

    npm_install(project.path());
    run_next_build_with_adapter(project.path(), &adapter_path);

    let descriptor = read_descriptor(project.path());
    assert_eq!(descriptor["version"], 1);
    assert_eq!(descriptor["adapter"]["name"], "@onreza/nrz-next-adapter");
    assert_eq!(descriptor["config"]["output"], "standalone");
    assert_eq!(descriptor["config"]["basePath"], "/docs");
    assert_eq!(descriptor["config"]["trailingSlash"], true);
    assert_eq!(descriptor["config"]["images"]["loader"], "custom");
    assert_eq!(
        descriptor["config"]["images"]["loaderFile"],
        "./.onreza/cache/next-adapter/onreza-image-loader.mjs"
    );
    assert_eq!(descriptor["config"]["images"]["path"], "/_onreza/image");
    assert_eq!(
        descriptor["deploymentHints"]["imageOptimizer"]["status"],
        "onreza_optimizer"
    );
    assert!(descriptor["routing"]["beforeMiddleware"].is_array());
    assert!(descriptor["routing"]["beforeFiles"].is_array());
    assert!(descriptor["routing"]["afterFiles"].is_array());
    assert!(descriptor["routing"]["onMatch"].is_array());
    assert!(descriptor["routing"]["fallback"].is_array());
    assert!(has_route_with_source(
        &descriptor["routing"],
        "beforeMiddleware",
        "/docs/old/:slug"
    ));
    assert!(has_route_with_source(
        &descriptor["routing"],
        "beforeFiles",
        "/docs/rewrite-header/:slug"
    ));
    assert!(has_route_with_source(
        &descriptor["routing"],
        "afterFiles",
        "/docs/legacy/:path*"
    ));
    assert!(has_route_with_source(
        &descriptor["routing"],
        "afterFiles",
        "/docs/plain-http/:path*"
    ));
    assert!(
        descriptor["outputs"]["appPages"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(
        descriptor["outputs"]["appRoutes"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(descriptor["outputs"]["middleware"].is_object());
    assert!(
        descriptor["outputs"]["staticFiles"]
            .as_array()
            .is_some_and(|items| items.iter().any(|file| file["pathname"]
                .as_str()
                .is_some_and(|pathname| pathname.starts_with("/docs/_next/static/"))))
    );
    assert!(
        descriptor["outputs"]["prerenders"]
            .as_array()
            .is_some_and(|items| items.iter().any(|output| output["pathname"] == "/docs"))
    );
    assert!(
        fs::read_to_string(project.path().join(".next/server/app/index.html"))
            .unwrap()
            .contains("/_onreza/image?url=%2Fpublic%2Fpixel.png")
    );

    let output = run_nrz_build_json(project.path());
    assert_eq!(output["compatibility"]["config"]["basePath"], "/docs");
    assert_eq!(output["compatibility"]["config"]["trailingSlash"], true);
    assert_eq!(
        output["compatibility"]["platform"]["imageOptimizer"]["status"],
        "onreza_optimizer"
    );
    assert_eq!(
        output["compatibility"]["platform"]["nodeRuntime"]["status"],
        "supported"
    );
    assert!(
        output["compatibility"]["platform"]["routeHandlers"]["count"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        output["compatibility"]["platform"]["routing"]["edgeRulesGenerated"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        output["compatibility"]["platform"]["routing"]["edgeRulesUnsupported"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    assert!(
        output["compatibility"]["platform"]["prerenders"]["staticLayerCount"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "prerender report: {}",
        serde_json::to_string_pretty(&serde_json::json!({
            "report": output["compatibility"]["platform"]["prerenders"],
            "routing": descriptor["routing"],
        }))
        .unwrap()
    );
    assert!(contains_file_named(
        &project.path().join(".next/standalone"),
        "robots.txt"
    ));
    assert!(contains_file_named(
        &project.path().join(".next/standalone"),
        "sitemap.xml"
    ));
}

#[test]
fn nextjs_16_2_static_export_uses_generic_static_build_path() {
    if !conformance_enabled() {
        eprintln!("skipping Next.js conformance test; set NRZ_NEXTJS_CONFORMANCE=1 to run it");
        return;
    }

    let project = tempfile::tempdir().unwrap();
    let next_version = next16_version();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adapter_path = manifest_dir.join("assets/next-adapter/onreza-next-adapter.cjs");

    write_next16_package(project.path(), &next_version, true);
    write(
        &project.path().join("next.config.mjs"),
        "export default { output: 'export' }\n",
    );
    write(
        &project.path().join("app/layout.js"),
        r#"
export default function RootLayout({ children }) {
  return <html><body>{children}</body></html>
}
"#,
    );
    write(
        &project.path().join("app/page.js"),
        r#"
export default function Page() {
  return <main>static export</main>
}
"#,
    );

    npm_install(project.path());
    run_next_build_with_adapter(project.path(), &adapter_path);

    assert!(project.path().join("out/index.html").is_file());
    let descriptor = read_descriptor(project.path());
    assert_eq!(descriptor["config"]["output"], "export");
    assert!(descriptor["outputs"]["staticFiles"].is_array());

    let output = run_nrz_build_json(project.path());
    assert_eq!(output["framework"], "nextjs");
    assert!(output.get("compatibility").is_none());
    assert!(
        output["layers"]
            .as_array()
            .is_some_and(|layers| layers.iter().all(|layer| layer["target"] == "STATIC"))
    );
}

#[test]
fn nextjs_16_2_pages_i18n_conformance_builds_static_locales() {
    if !conformance_enabled() {
        eprintln!("skipping Next.js conformance test; set NRZ_NEXTJS_CONFORMANCE=1 to run it");
        return;
    }

    let project = tempfile::tempdir().unwrap();
    let next_version = next16_version();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adapter_path = manifest_dir.join("assets/next-adapter/onreza-next-adapter.cjs");

    write_next16_package(project.path(), &next_version, false);
    write(
        &project.path().join("next.config.mjs"),
        r#"
const nextConfig = {
  output: 'standalone',
  i18n: { locales: ['en', 'fr', 'ru'], defaultLocale: 'en' },
  trailingSlash: true,
  async redirects() {
    return [{ source: '/i18n-old', destination: '/about', permanent: false }]
  },
  async rewrites() {
    return [{ source: '/i18n-rewrite', destination: '/about' }]
  },
  async headers() {
    return [{ source: '/about', headers: [{ key: 'x-i18n-header', value: 'yes' }] }]
  },
}

export default nextConfig
"#,
    );
    write(
        &project.path().join("pages/index.js"),
        r#"
export default function Page({ locale }) {
  return <main>home {locale}</main>
}

export function getStaticProps({ locale }) {
  return { props: { locale } }
}
"#,
    );
    write(
        &project.path().join("pages/about.js"),
        r#"
export default function Page({ locale }) {
  return <main>about {locale}</main>
}

export function getStaticProps({ locale }) {
  return { props: { locale } }
}
"#,
    );
    write(
        &project.path().join("pages/api/hello.js"),
        r#"
export default function handler(req, res) {
  res.status(200).json({ ok: true, locale: req.query.__nextLocale || null })
}
"#,
    );

    npm_install(project.path());
    run_next_build_with_adapter(project.path(), &adapter_path);

    let descriptor = read_descriptor(project.path());
    assert_eq!(descriptor["config"]["i18n"]["defaultLocale"], "en");
    assert_eq!(descriptor["config"]["trailingSlash"], true);
    assert!(
        descriptor["outputs"]["pages"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(
        descriptor["outputs"]["pagesApi"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(
        descriptor["outputs"]["prerenders"]
            .as_array()
            .is_some_and(|items| items.iter().any(|output| output["pathname"] == "/fr/about"))
    );

    let output = run_nrz_build_json(project.path());
    assert_eq!(
        output["compatibility"]["config"]["i18n"]["defaultLocale"],
        "en"
    );
    assert_eq!(
        output["compatibility"]["platform"]["routeHandlers"]["status"],
        "compute_fallback"
    );
    assert!(
        output["compatibility"]["platform"]["prerenders"]["staticLayerCount"]
            .as_u64()
            .is_some_and(|count| count >= 3),
        "prerender report: {}",
        serde_json::to_string_pretty(&output["compatibility"]["platform"]["prerenders"]).unwrap()
    );
}

#[test]
fn nextjs_legacy_conformance_uses_standalone_fallback_without_adapter_descriptor() {
    if !conformance_enabled() {
        eprintln!("skipping Next.js conformance test; set NRZ_NEXTJS_CONFORMANCE=1 to run it");
        return;
    }

    let versions =
        std::env::var("NRZ_NEXTJS_LEGACY_VERSIONS").unwrap_or_else(|_| "14,15,16.1.6".into());
    for next_version in versions
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let project = tempfile::tempdir().unwrap();
        write_legacy_package(project.path(), next_version);
        write(
            &project.path().join("next.config.js"),
            r#"
module.exports = {}
"#,
        );
        write(
            &project.path().join("pages/index.js"),
            r#"
export default function Page({ generatedAt }) {
  return <main>legacy {generatedAt}</main>
}

export function getStaticProps() {
  return { props: { generatedAt: 'build-time' } }
}
"#,
        );
        write(
            &project.path().join("pages/api/hello.js"),
            r#"
export default function handler(req, res) {
  res.status(200).json({ ok: true })
}
"#,
        );

        npm_install(project.path());
        run_next_build_legacy(project.path());

        assert!(
            project.path().join(".next/standalone/server.js").is_file(),
            "legacy Next {next_version} should honor NEXT_PRIVATE_STANDALONE=1"
        );
        assert!(
            !project
                .path()
                .join(".onreza/next-adapter-output.json")
                .exists(),
            "legacy Next {next_version} should not use NEXT_ADAPTER_PATH"
        );

        let output = run_nrz_build_json(project.path());
        assert_eq!(
            output["framework"], "nextjs",
            "legacy Next {next_version} should still be detected as Next.js"
        );
        assert!(
            output.get("compatibility").is_none(),
            "legacy Next {next_version} should use the standalone manifest path, not adapter compatibility"
        );
        assert!(
            output["layers"].as_array().is_some_and(|layers| layers
                .iter()
                .any(|layer| layer["target"] == "COMPUTE" && layer["entry"] == "server.js")),
            "legacy Next {next_version} should produce a standalone COMPUTE layer"
        );
    }
}
