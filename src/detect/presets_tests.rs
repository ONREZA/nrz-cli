use super::presets::*;
use super::types::RuntimeType;

#[test]
fn presets_sorted_by_priority() {
    let priorities: Vec<u32> = PRESETS.iter().map(|p| p.priority).collect();
    let mut sorted = priorities.clone();
    sorted.sort();
    assert_eq!(priorities, sorted, "presets must be sorted by priority");
}

#[test]
fn all_slugs_unique() {
    let mut slugs: Vec<&str> = PRESETS.iter().map(|p| p.slug).collect();
    let count = slugs.len();
    slugs.sort();
    slugs.dedup();
    assert_eq!(slugs.len(), count, "all preset slugs must be unique");
}

#[test]
fn get_preset_by_slug_found() {
    let p = get_preset_by_slug("nextjs").unwrap();
    assert_eq!(p.name, "Next.js");
    assert_eq!(p.priority, 1);
}

#[test]
fn get_preset_by_slug_not_found() {
    assert!(get_preset_by_slug("nonexistent").is_none());
}

#[test]
fn default_preset_is_other() {
    let p = get_default_preset();
    assert_eq!(p.slug, "other");
    assert_eq!(p.priority, 1000);
}

#[test]
fn static_html_preset_exists() {
    let p = get_static_html_preset();
    assert_eq!(p.slug, "static-html");
    assert_eq!(p.runtime, RuntimeType::Static);
    assert_eq!(p.output_directory, ".");
    // Verify it's NOT in PRESETS (separate const)
    assert!(PRESETS.iter().all(|preset| preset.slug != "static-html"));
}

#[test]
fn ssr_frameworks_recognized() {
    assert!(is_ssr_framework("nextjs"));
    assert!(is_ssr_framework("nuxt"));
    assert!(is_ssr_framework("sveltekit"));
    assert!(is_ssr_framework("astro"));
    assert!(is_ssr_framework("remix"));
    assert!(is_ssr_framework("react-router"));
    assert!(is_ssr_framework("solidstart"));
    assert!(is_ssr_framework("qwik"));
    assert!(is_ssr_framework("analog"));
    assert!(is_ssr_framework("blitzjs"));
    assert!(is_ssr_framework("payload"));
    assert!(is_ssr_framework("tanstack-start"));
    assert!(is_ssr_framework("hydrogen"));
    assert!(!is_ssr_framework("vite"));
    assert!(!is_ssr_framework("hono"));
    assert!(!is_ssr_framework("expo"));
}

#[test]
fn server_frameworks_recognized() {
    assert!(is_server_framework("hono"));
    assert!(is_server_framework("elysia"));
    assert!(is_server_framework("nestjs"));
    assert!(is_server_framework("fastify"));
    assert!(is_server_framework("adonis"));
    assert!(is_server_framework("express"));
    assert!(is_server_framework("koa"));
    assert!(is_server_framework("h3"));
    assert!(is_server_framework("nitro"));
    assert!(is_server_framework("keystone"));
    assert!(is_server_framework("redwoodjs"));
    assert!(is_server_framework("strapi"));
    // Blitz.js and Payload are Next.js wrappers, not plain server frameworks
    assert!(!is_server_framework("blitzjs"));
    assert!(!is_server_framework("payload"));
    assert!(!is_server_framework("nextjs"));
    assert!(!is_server_framework("vite"));
}

#[test]
fn detection_presets_have_dependencies() {
    for p in detection_presets() {
        assert!(
            !p.dependencies.is_empty(),
            "detection preset '{}' must have dependencies",
            p.slug
        );
    }
}

#[test]
fn total_preset_count() {
    // 39 detection presets (static-html is a separate const, not in PRESETS)
    assert_eq!(PRESETS.len(), 39);
}

#[test]
fn tier1_presets_correct() {
    let nextjs = get_preset_by_slug("nextjs").unwrap();
    assert_eq!(nextjs.dependencies, &["next"]);
    assert_eq!(nextjs.output_directory, ".next");

    let nuxt = get_preset_by_slug("nuxt").unwrap();
    assert_eq!(nuxt.dependencies, &["nuxt"]);
    assert_eq!(nuxt.output_directory, ".output");
    assert_eq!(nuxt.build_script, Some("build"));

    let sveltekit = get_preset_by_slug("sveltekit").unwrap();
    assert_eq!(sveltekit.dependencies, &["@sveltejs/kit"]);

    let react_router = get_preset_by_slug("react-router").unwrap();
    assert_eq!(react_router.dependencies, &["@react-router/dev"]);
    assert_eq!(react_router.output_directory, "build");

    let remix = get_preset_by_slug("remix").unwrap();
    assert_eq!(remix.dependencies, &["@remix-run/react", "@remix-run/dev"]);
    assert_eq!(remix.output_directory, "build");

    let gatsby = get_preset_by_slug("gatsby").unwrap();
    assert_eq!(gatsby.dependencies, &["gatsby"]);
    assert_eq!(gatsby.output_directory, "public");
}

#[test]
fn expo_preset_correct() {
    let expo = get_preset_by_slug("expo").unwrap();
    assert_eq!(expo.dependencies, &["expo"]);
    assert_eq!(expo.output_directory, "dist");
    assert_eq!(expo.priority, 9);
}

#[test]
fn nextjs_wrappers_correct() {
    let blitzjs = get_preset_by_slug("blitzjs").unwrap();
    assert_eq!(blitzjs.priority, 1);
    assert_eq!(blitzjs.output_directory, ".next");
    assert!(is_nextjs_wrapper(blitzjs.slug));

    let payload = get_preset_by_slug("payload").unwrap();
    assert_eq!(payload.priority, 1);
    assert_eq!(payload.output_directory, ".next");
    assert!(is_nextjs_wrapper(payload.slug));

    assert!(!is_nextjs_wrapper("nextjs"));
    assert!(!is_nextjs_wrapper("vite"));
}

#[test]
fn framework_output_dirs_expo() {
    let dirs = framework_output_dirs("expo");
    assert!(dirs.contains(&"dist"));
}

#[test]
fn tanstack_start_preset_correct() {
    let ts = get_preset_by_slug("tanstack-start").unwrap();
    assert_eq!(ts.dependencies, &["@tanstack/react-start"]);
    assert_eq!(ts.output_directory, "dist");
    assert_eq!(ts.priority, 9);
    assert!(is_ssr_framework(ts.slug));
}

#[test]
fn hydrogen_preset_correct() {
    let h = get_preset_by_slug("hydrogen").unwrap();
    assert_eq!(h.dependencies, &["@shopify/hydrogen"]);
    assert_eq!(h.output_directory, "build");
    assert_eq!(h.priority, 1);
    assert!(is_ssr_framework(h.slug));
}

#[test]
fn framework_output_dirs_tanstack_start() {
    let dirs = framework_output_dirs("tanstack-start");
    assert!(dirs.contains(&"dist"));
}

#[test]
fn framework_output_dirs_hydrogen() {
    let dirs = framework_output_dirs("hydrogen");
    assert!(dirs.contains(&"build"));
    assert!(dirs.contains(&"build/client"));
    assert!(dirs.contains(&"build/server"));
}

#[test]
fn ssr_metaframework_presets_correct() {
    let solidstart = get_preset_by_slug("solidstart").unwrap();
    assert_eq!(solidstart.dependencies, &["@solidjs/start"]);
    assert_eq!(solidstart.output_directory, ".output");
    assert_eq!(solidstart.priority, 7);

    let qwik = get_preset_by_slug("qwik").unwrap();
    assert_eq!(
        qwik.dependencies,
        &["@builder.io/qwik-city", "@qwik.dev/router"]
    );
    assert_eq!(qwik.output_directory, "dist");
    assert_eq!(qwik.priority, 8);

    let analog = get_preset_by_slug("analog").unwrap();
    assert_eq!(analog.dependencies, &["@analogjs/platform"]);
    assert_eq!(analog.output_directory, "dist");
    assert_eq!(analog.priority, 9);
}

#[test]
fn server_framework_presets_correct() {
    let nestjs = get_preset_by_slug("nestjs").unwrap();
    assert_eq!(nestjs.dependencies, &["@nestjs/core"]);
    assert_eq!(nestjs.output_directory, "dist");
    assert_eq!(nestjs.runtime, RuntimeType::Node);

    let fastify = get_preset_by_slug("fastify").unwrap();
    assert_eq!(fastify.dependencies, &["fastify"]);

    let adonis = get_preset_by_slug("adonis").unwrap();
    assert_eq!(adonis.dependencies, &["@adonisjs/core"]);
    assert_eq!(adonis.output_directory, "build");

    let express = get_preset_by_slug("express").unwrap();
    assert_eq!(express.dependencies, &["express"]);
    assert_eq!(express.output_directory, ".");
    assert_eq!(express.build_script, None);

    let koa = get_preset_by_slug("koa").unwrap();
    assert_eq!(koa.dependencies, &["koa"]);
    assert_eq!(koa.output_directory, ".");
    assert_eq!(koa.build_script, None);

    let h3 = get_preset_by_slug("h3").unwrap();
    assert_eq!(h3.dependencies, &["h3"]);

    let nitro = get_preset_by_slug("nitro").unwrap();
    assert_eq!(nitro.dependencies, &["nitropack"]);
    assert_eq!(nitro.output_directory, ".output");

    let blitzjs = get_preset_by_slug("blitzjs").unwrap();
    assert_eq!(blitzjs.dependencies, &["@blitzjs/next"]);
    assert_eq!(blitzjs.output_directory, ".next");

    let keystone = get_preset_by_slug("keystone").unwrap();
    assert_eq!(keystone.dependencies, &["@keystone-6/core"]);
    assert_eq!(keystone.output_directory, ".keystone");

    let redwoodjs = get_preset_by_slug("redwoodjs").unwrap();
    assert_eq!(redwoodjs.dependencies, &["@redwoodjs/core"]);
    assert_eq!(redwoodjs.output_directory, "api/dist");

    let payload = get_preset_by_slug("payload").unwrap();
    assert_eq!(payload.dependencies, &["@payloadcms/next"]);
    assert_eq!(payload.output_directory, ".next");
    assert_eq!(payload.priority, 1);

    let strapi = get_preset_by_slug("strapi").unwrap();
    assert_eq!(strapi.dependencies, &["@strapi/strapi"]);
    assert_eq!(strapi.output_directory, "dist");
}

#[test]
fn framework_output_dirs_nextjs() {
    let dirs = framework_output_dirs("nextjs");
    assert!(dirs.contains(&".next"));
    assert!(dirs.contains(&".next/standalone"));
    assert!(dirs.contains(&"out"));
}

#[test]
fn framework_output_dirs_nextjs_standalone_before_dot_next() {
    let dirs = framework_output_dirs("nextjs");
    let standalone_pos = dirs.iter().position(|d| *d == ".next/standalone").unwrap();
    let dot_next_pos = dirs.iter().position(|d| *d == ".next").unwrap();
    assert!(
        standalone_pos < dot_next_pos,
        ".next/standalone should come before .next (standalone_pos={standalone_pos}, dot_next_pos={dot_next_pos})"
    );
}

#[test]
fn framework_output_dirs_react_router() {
    let dirs = framework_output_dirs("react-router");
    assert!(dirs.contains(&"build"));
    assert!(dirs.contains(&"build/client"));
    assert!(dirs.contains(&"build/server"));
}

#[test]
fn framework_output_dirs_remix() {
    let dirs = framework_output_dirs("remix");
    assert!(dirs.contains(&"build"));
    assert!(dirs.contains(&"build/client"));
    assert!(dirs.contains(&"build/server"));
}

#[test]
fn framework_output_dirs_nuxt() {
    let dirs = framework_output_dirs("nuxt");
    assert!(dirs.contains(&".output"));
}

#[test]
fn framework_output_dirs_gatsby_includes_public() {
    let dirs = framework_output_dirs("gatsby");
    assert!(dirs.contains(&"public"));
}

#[test]
fn framework_output_dirs_vitepress() {
    let dirs = framework_output_dirs("vitepress");
    assert!(dirs.contains(&".vitepress/dist"));
}

#[test]
fn framework_output_dirs_hono() {
    let dirs = framework_output_dirs("hono");
    assert!(dirs.contains(&"dist"));
}

#[test]
fn framework_output_dirs_elysia() {
    let dirs = framework_output_dirs("elysia");
    assert!(dirs.contains(&"dist"));
}

#[test]
fn framework_output_dirs_nestjs() {
    let dirs = framework_output_dirs("nestjs");
    assert!(dirs.contains(&"dist"));
}

#[test]
fn framework_output_dirs_fastify() {
    let dirs = framework_output_dirs("fastify");
    assert!(dirs.contains(&"dist"));
    assert!(dirs.contains(&"."));
}

#[test]
fn framework_output_dirs_adonis() {
    let dirs = framework_output_dirs("adonis");
    assert!(dirs.contains(&"build"));
}

#[test]
fn framework_output_dirs_express() {
    let dirs = framework_output_dirs("express");
    assert!(dirs.contains(&"."));
}

#[test]
fn framework_output_dirs_koa() {
    let dirs = framework_output_dirs("koa");
    assert!(dirs.contains(&"."));
}

#[test]
fn framework_output_dirs_h3() {
    let dirs = framework_output_dirs("h3");
    assert!(dirs.contains(&"dist"));
    assert!(dirs.contains(&"."));
}

#[test]
fn framework_output_dirs_nitro() {
    let dirs = framework_output_dirs("nitro");
    assert!(dirs.contains(&".output"));
}

#[test]
fn framework_output_dirs_solidstart() {
    let dirs = framework_output_dirs("solidstart");
    assert!(dirs.contains(&".output"));
}

#[test]
fn framework_output_dirs_qwik() {
    let dirs = framework_output_dirs("qwik");
    assert!(dirs.contains(&"dist"));
    assert!(dirs.contains(&"server"));
}

#[test]
fn framework_output_dirs_analog() {
    let dirs = framework_output_dirs("analog");
    assert!(dirs.contains(&"dist"));
    assert!(dirs.contains(&"dist/analog"));
}

#[test]
fn framework_output_dirs_blitzjs() {
    let dirs = framework_output_dirs("blitzjs");
    assert!(dirs.contains(&".next/standalone"));
    assert!(dirs.contains(&".next"));
}

#[test]
fn framework_output_dirs_keystone() {
    let dirs = framework_output_dirs("keystone");
    assert!(dirs.contains(&".keystone"));
}

#[test]
fn framework_output_dirs_redwoodjs() {
    let dirs = framework_output_dirs("redwoodjs");
    assert!(dirs.contains(&"api/dist"));
    assert!(dirs.contains(&"web/dist"));
}

#[test]
fn framework_output_dirs_payload() {
    let dirs = framework_output_dirs("payload");
    assert!(dirs.contains(&".next/standalone"));
    assert!(dirs.contains(&".next"));
}

#[test]
fn framework_output_dirs_strapi() {
    let dirs = framework_output_dirs("strapi");
    assert!(dirs.contains(&"dist"));
    assert!(dirs.contains(&"build"));
}

#[test]
fn framework_output_dirs_unknown_is_empty() {
    let dirs = framework_output_dirs("unknown-framework");
    assert!(dirs.is_empty());
}

#[test]
fn static_html_preset_is_separate_const() {
    // static-html is not in PRESETS (no dependencies to match)
    assert!(get_preset_by_slug("static-html").is_none());
    // but accessible via dedicated getter
    let p = get_static_html_preset();
    assert_eq!(p.slug, "static-html");
}

#[test]
fn framework_output_dirs_includes_preset_default() {
    for preset in PRESETS.iter() {
        if preset.slug == "other" {
            continue;
        }
        let dirs = framework_output_dirs(preset.slug);
        assert!(
            dirs.contains(&preset.output_directory),
            "framework_output_dirs('{}') = {:?} does not contain preset default '{}'",
            preset.slug,
            dirs,
            preset.output_directory
        );
    }
}
