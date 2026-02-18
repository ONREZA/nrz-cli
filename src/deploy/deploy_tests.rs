use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn scan_files_flat_directory() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    fs::write(dir.path().join("style.css"), "body{}").unwrap();

    let files = scan_files(dir.path()).unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "index.html");
    assert_eq!(files[1].path, "style.css");
}

#[test]
fn scan_files_nested_directory() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets/images")).unwrap();
    fs::write(dir.path().join("index.html"), "hi").unwrap();
    fs::write(dir.path().join("assets/app.js"), "js").unwrap();
    fs::write(dir.path().join("assets/images/logo.png"), "png").unwrap();

    let files = scan_files(dir.path()).unwrap();

    assert_eq!(files.len(), 3);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"index.html"));
    assert!(paths.contains(&"assets/app.js"));
    assert!(paths.contains(&"assets/images/logo.png"));
}

#[test]
fn scan_files_records_correct_sizes() {
    let dir = tempdir().unwrap();
    let content = "hello world";
    fs::write(dir.path().join("file.txt"), content).unwrap();

    let files = scan_files(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}

#[test]
fn scan_files_empty_directory() {
    let dir = tempdir().unwrap();
    let files = scan_files(dir.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn scan_files_sorted_alphabetically() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.txt"), "z").unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("m.txt"), "m").unwrap();

    let files = scan_files(dir.path()).unwrap();

    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "m.txt");
    assert_eq!(files[2].path, "z.txt");
}

#[test]
fn scan_files_allows_double_dots_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file..backup.js"), "x").unwrap();

    let files = scan_files(dir.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "file..backup.js");
}

#[cfg(unix)]
#[test]
fn scan_files_skips_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

    let files = scan_files(dir.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "real.txt");
}

// ── synthetic_sha tests ─────────────────────────────────────

#[test]
fn synthetic_sha_deterministic() {
    let files = vec![
        FileEntry {
            path: "a.js".into(),
            size: 100,
        },
        FileEntry {
            path: "b.css".into(),
            size: 200,
        },
    ];

    let sha1 = synthetic_sha(&files);
    let sha2 = synthetic_sha(&files);
    assert_eq!(sha1, sha2);
}

#[test]
fn synthetic_sha_differs_for_different_files() {
    let files_a = vec![FileEntry {
        path: "a.js".into(),
        size: 100,
    }];
    let files_b = vec![FileEntry {
        path: "b.js".into(),
        size: 100,
    }];

    assert_ne!(synthetic_sha(&files_a), synthetic_sha(&files_b));
}

#[test]
fn synthetic_sha_is_64_hex_chars() {
    let files = vec![FileEntry {
        path: "x.txt".into(),
        size: 1,
    }];
    let sha = synthetic_sha(&files);
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── detect_migrations tests ─────────────────────────────────

#[test]
fn detect_migrations_skip_flag() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();
    fs::write(
        dir.path().join("migrations/0001_init.sql"),
        "CREATE TABLE t;",
    )
    .unwrap();

    let result = detect_migrations(dir.path(), true, true, "migrations").unwrap();
    assert!(result.is_none());
}

#[test]
fn detect_migrations_no_migrations_dir() {
    let dir = tempdir().unwrap();

    let result = detect_migrations(dir.path(), true, false, "migrations").unwrap();
    assert!(result.is_none());
}

#[test]
fn detect_migrations_empty_migrations_dir() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();

    let result = detect_migrations(dir.path(), true, false, "migrations").unwrap();
    assert!(result.is_none());
}

#[test]
fn detect_migrations_returns_entries() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();
    fs::write(
        dir.path().join("migrations/0001_init.sql"),
        "CREATE TABLE t;",
    )
    .unwrap();
    fs::write(
        dir.path().join("migrations/0002_users.sql"),
        "CREATE TABLE users;",
    )
    .unwrap();

    let entries = detect_migrations(dir.path(), true, false, "migrations")
        .unwrap()
        .expect("should return migrations");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "0001_init");
    assert_eq!(entries[1].name, "0002_users");
    assert!(!entries[0].checksum.is_empty());
}

#[test]
fn detect_migrations_custom_dir() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/0001_init.sql"),
        "CREATE TABLE t;",
    )
    .unwrap();

    let entries = detect_migrations(dir.path(), true, false, "db/migrations")
        .unwrap()
        .expect("should find migrations in custom dir");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "0001_init");
}

// ── resolve_build_command tests ──────────────────────────────

#[test]
fn build_command_explicit_wins_over_config_and_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let result = resolve_build_command(Some("explicit cmd"), dir.path(), &config);
    assert_eq!(result.unwrap(), "explicit cmd");
}

#[test]
fn build_command_config_wins_over_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_auto_detect_bun_lock() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("bun.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_bun_lockb() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("bun.lockb"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_pnpm() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_auto_detect_yarn() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "yarn run build");
}

#[test]
fn build_command_auto_detect_npm_fallback() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "npm run build");
}

#[test]
fn build_command_none_without_package_json() {
    let dir = tempdir().unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert!(result.is_none());
}

// ── guess_content_type tests ────────────────────────────────

#[test]
fn content_type_html() {
    assert_eq!(guess_content_type("index.html"), "text/html");
    assert_eq!(guess_content_type("page.htm"), "text/html");
}

#[test]
fn content_type_js() {
    assert_eq!(guess_content_type("app.js"), "application/javascript");
    assert_eq!(guess_content_type("entry.mjs"), "application/javascript");
    assert_eq!(guess_content_type("lib.cjs"), "application/javascript");
}

#[test]
fn content_type_css() {
    assert_eq!(guess_content_type("style.css"), "text/css");
}

#[test]
fn content_type_images() {
    assert_eq!(guess_content_type("logo.png"), "image/png");
    assert_eq!(guess_content_type("photo.jpg"), "image/jpeg");
    assert_eq!(guess_content_type("photo.jpeg"), "image/jpeg");
    assert_eq!(guess_content_type("hero.webp"), "image/webp");
    assert_eq!(guess_content_type("icon.svg"), "image/svg+xml");
    assert_eq!(guess_content_type("icon.ico"), "image/x-icon");
}

#[test]
fn content_type_fonts() {
    assert_eq!(guess_content_type("font.woff2"), "font/woff2");
    assert_eq!(guess_content_type("font.woff"), "font/woff");
    assert_eq!(guess_content_type("font.ttf"), "font/ttf");
}

#[test]
fn content_type_data() {
    assert_eq!(guess_content_type("data.json"), "application/json");
    assert_eq!(guess_content_type("app.d4e5f6.js.map"), "application/json");
    assert_eq!(guess_content_type("app.wasm"), "application/wasm");
}

#[test]
fn content_type_nested_path() {
    assert_eq!(
        guess_content_type("_astro/app.d4e5f6.js"),
        "application/javascript"
    );
    assert_eq!(
        guess_content_type("server/entry.mjs"),
        "application/javascript"
    );
}

#[test]
fn content_type_unknown_fallback() {
    assert_eq!(guess_content_type("file.xyz"), "application/octet-stream");
    assert_eq!(guess_content_type("noext"), "application/octet-stream");
}
