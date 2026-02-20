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
    assert!(!is_ssr_framework("vite"));
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
    // 17 detection presets (static-html is a separate const, not in PRESETS)
    assert_eq!(PRESETS.len(), 17);
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

    let gatsby = get_preset_by_slug("gatsby").unwrap();
    assert_eq!(gatsby.dependencies, &["gatsby"]);
    assert_eq!(gatsby.output_directory, "public");
}

#[test]
fn framework_output_dirs_nextjs() {
    let dirs = framework_output_dirs("nextjs");
    assert!(dirs.contains(&".next"));
    assert!(dirs.contains(&".next/standalone"));
    assert!(dirs.contains(&"out"));
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
