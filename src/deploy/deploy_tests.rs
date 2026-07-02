use std::fs;
use std::path::Path;

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::put;
use tempfile::tempdir;

use crate::artifact::source_bundle_v1;
use crate::build;
use crate::cli::BuildArgs;
use crate::frameworks::{
    clear_before_build as clear_nextjs_descriptor_before_build, is_nextjs_project,
    is_sveltekit_with_adapter_auto,
};

use super::hash::sha256_hex;
use super::*;

fn fe(path: &str, size: u64, content_hash: &str) -> FileEntry {
    FileEntry {
        path: path.into(),
        size,
        content_hash: content_hash.into(),
        kind: crate::artifact::ArtifactFileKind::File,
        symlink_resolved_path: None,
    }
}

fn git_lfs_pointer() -> &'static str {
    "version https://git-lfs.github.com/spec/v1\n\
     oid sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n\
     size 104857600\n"
}

fn effective_config(
    project_dir: &Path,
    config: nrz::config::ProjectConfig,
) -> nrz::config::EffectiveProjectConfig {
    nrz::config::EffectiveProjectConfig::from_project_config(project_dir.to_path_buf(), config)
}

fn effective_with_server_settings(
    project_dir: &Path,
    config: nrz::config::ProjectConfig,
    settings: nrz::config::ProjectBuildSettings,
) -> nrz::config::EffectiveProjectConfig {
    let mut effective = effective_config(project_dir, config);
    effective.apply_server_settings(Some(&settings));
    effective
}

fn server_build_settings(
    command: Option<&str>,
    source: Option<nrz::config::BuildSettingSource>,
) -> nrz::config::ProjectBuildSettings {
    nrz::config::ProjectBuildSettings {
        build_command: command.map(str::to_string),
        build_command_source: source,
        ..Default::default()
    }
}

fn server_install_settings(
    command: Option<&str>,
    source: Option<nrz::config::BuildSettingSource>,
) -> nrz::config::ProjectBuildSettings {
    nrz::config::ProjectBuildSettings {
        install_command: command.map(str::to_string),
        install_command_source: source,
        ..Default::default()
    }
}

async fn conditional_pack_put(headers: HeaderMap) -> impl IntoResponse {
    match headers
        .get("if-none-match")
        .and_then(|value| value.to_str().ok())
    {
        Some("*") => (StatusCode::OK, "ok"),
        _ => (StatusCode::BAD_REQUEST, "missing if-none-match"),
    }
}

async fn spawn_conditional_pack_put_mock() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route("/upload", put(conditional_pack_put));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

#[test]
fn scan_files_flat_directory() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    fs::write(dir.path().join("style.css"), "body{}").unwrap();

    let files = scan_dir(dir.path()).unwrap();

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

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 3);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"index.html"));
    assert!(paths.contains(&"assets/app.js"));
    assert!(paths.contains(&"assets/images/logo.png"));
}

#[test]
fn scan_files_skips_vcs_internal_dirs() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".git/objects/pack")).unwrap();
    fs::create_dir_all(dir.path().join("packages/app/.git/objects/pack")).unwrap();
    fs::create_dir_all(dir.path().join(".hg/store")).unwrap();
    fs::create_dir_all(dir.path().join("vendor/pkg/.svn")).unwrap();
    fs::create_dir_all(dir.path().join(".svn")).unwrap();
    fs::write(dir.path().join(".git/objects/pack/pack.dat"), "git").unwrap();
    fs::write(
        dir.path().join("packages/app/.git/objects/pack/pack.dat"),
        "nested-git",
    )
    .unwrap();
    fs::write(dir.path().join(".hg/store/data"), "hg").unwrap();
    fs::write(dir.path().join("vendor/pkg/.svn/entries"), "nested-svn").unwrap();
    fs::write(dir.path().join(".svn/entries"), "svn").unwrap();
    fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
    fs::write(dir.path().join("index.html"), "hi").unwrap();

    let files = scan_dir(dir.path()).unwrap();
    let paths = files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec![".gitignore", "index.html"]);
}

#[test]
fn scan_files_records_correct_sizes() {
    let dir = tempdir().unwrap();
    let content = "hello world";
    fs::write(dir.path().join("file.txt"), content).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}

#[test]
fn lfs_pointer_requires_project_lfs_setting() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("public")).unwrap();
    fs::write(dir.path().join("public/model.glb"), git_lfs_pointer()).unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let err = ensure_no_unresolved_lfs_pointers(dir.path(), &files, false).unwrap_err();
    let coded = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .expect("LFS pointer error must carry a structured code");

    assert_eq!(coded.code, "GIT_LFS_REQUIRED");
    assert!(err.to_string().contains("public/model.glb"), "{err}");
}

#[test]
fn lfs_pointer_still_fails_when_project_lfs_enabled() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("public")).unwrap();
    fs::write(dir.path().join("public/model.glb"), git_lfs_pointer()).unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let err = ensure_no_unresolved_lfs_pointers(dir.path(), &files, true).unwrap_err();
    let coded = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .expect("LFS pointer error must carry a structured code");

    assert_eq!(coded.code, "GIT_LFS_UNRESOLVED");
    assert!(err.to_string().contains("git lfs pull"), "{err}");
}

#[test]
fn scan_files_computes_sha256_from_original_content() {
    let dir = tempdir().unwrap();
    let content = "hello world";
    fs::write(dir.path().join("file.txt"), content).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    let hash = files[0].content_hash.as_str();
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Known SHA-256 of "hello world" — guards against accidental hashing of
    // anything other than SOURCE_BUNDLE_V1 identity bytes.
    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn scan_files_handles_file_larger_than_chunk() {
    // Stream-hash path covers a file that needs multiple read() calls — guards
    // against an off-by-one that would only hash the first chunk.
    let dir = tempdir().unwrap();
    let content = vec![0xABu8; 200 * 1024]; // 200 KiB > 64 KiB SCAN_HASH_CHUNK_BYTES
    fs::write(dir.path().join("big.bin"), &content).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
    let expected = sha256_hex(&content);
    assert_eq!(files[0].content_hash, expected);
}

#[test]
fn scan_files_sha256_deterministic_across_calls() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "same content").unwrap();

    let files1 = scan_dir(dir.path()).unwrap();
    let files2 = scan_dir(dir.path()).unwrap();

    assert_eq!(files1[0].content_hash, files2[0].content_hash);
}

#[test]
fn scan_files_sha256_differs_for_different_content() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "content A").unwrap();
    fs::write(dir.path().join("b.txt"), "content B").unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_ne!(files[0].content_hash, files[1].content_hash);
}

#[test]
fn scan_files_empty_directory() {
    let dir = tempdir().unwrap();
    let files = scan_dir(dir.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn scan_files_sorted_alphabetically() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.txt"), "z").unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("m.txt"), "m").unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "m.txt");
    assert_eq!(files[2].path, "z.txt");
}

#[test]
fn scan_files_allows_double_dots_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file..backup.js"), "x").unwrap();

    let files = scan_dir(dir.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "file..backup.js");
}

#[cfg(unix)]
#[test]
fn scan_files_preserves_safe_relative_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink("real.txt", dir.path().join("link.txt")).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    let link = files.iter().find(|file| file.path == "link.txt").unwrap();
    assert_eq!(link.size, 0);
    assert_eq!(link.content_hash, sha256_hex(b"real.txt"));
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_absolute_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();
    assert!(err.to_string().contains("absolute target"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_overlong_symlink_targets() {
    let dir = tempdir().unwrap();
    let target = "a".repeat(source_bundle_v1::SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS + 1);
    std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();

    assert!(err.to_string().contains("target too long"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_broken_symlinks() {
    let dir = tempdir().unwrap();
    std::os::unix::fs::symlink("missing.txt", dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();
    assert!(err.to_string().contains("broken symlink"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_symlinks_that_escape_output() {
    let root = tempdir().unwrap();
    let dir = root.path().join("app");
    let outside = root.path().join("outside");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink("../outside/real.txt", dir.join("link.txt")).unwrap();

    let err = scan_dir(&dir).unwrap_err();
    assert!(err.to_string().contains("escapes build output"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_symlinks_that_traverse_through_regular_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink("real.txt/", dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();

    assert!(err.to_string().contains("broken symlink"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_symlink_parent_traversal_after_regular_file() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("dir")).unwrap();
    fs::write(dir.path().join("dir/file"), "file").unwrap();
    fs::write(dir.path().join("dir/other"), "other").unwrap();
    std::os::unix::fs::symlink("dir/file/../other", dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();

    assert!(err.to_string().contains("broken symlink"), "{err}");
}

// ── synthetic_sha tests ─────────────────────────────────────

#[test]
fn synthetic_sha_deterministic() {
    let files = vec![fe("a.js", 100, "aa"), fe("b.css", 200, "bb")];
    let sha1 = synthetic_sha(&files);
    let sha2 = synthetic_sha(&files);
    assert_eq!(sha1, sha2);
}

#[test]
fn synthetic_sha_differs_for_different_files() {
    let files_a = vec![fe("a.js", 100, "aa")];
    let files_b = vec![fe("b.js", 100, "aa")];
    assert_ne!(synthetic_sha(&files_a), synthetic_sha(&files_b));
}

#[test]
fn synthetic_sha_differs_for_same_path_different_hash() {
    // Now that synthetic_sha mixes content_hash into the digest, two deploys
    // of the same paths but mutated bytes produce distinct synthetic SHAs.
    // This is what makes the synthetic SHA a valid commit-SHA fallback for
    // cross-deploy CAS dedup keying.
    let files_a = vec![fe("a.js", 100, "aaaa")];
    let files_b = vec![fe("a.js", 100, "bbbb")];
    assert_ne!(synthetic_sha(&files_a), synthetic_sha(&files_b));
}

#[test]
fn synthetic_sha_is_64_hex_chars() {
    let files = vec![fe("x.txt", 1, "ff")];
    let sha = synthetic_sha(&files);
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── resolve_build_command tests ──────────────────────────────

#[test]
fn build_command_explicit_wins_over_config_and_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(Some("explicit cmd"), dir.path(), &effective);
    assert_eq!(result.unwrap(), "explicit cmd");
}

#[test]
fn build_command_config_wins_over_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_auto_detect_bun_lock() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("bun.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_bun_lockb() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("bun.lockb"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_pnpm() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_auto_detect_yarn() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "yarn run build");
}

#[test]
fn build_command_auto_detect_npm_fallback() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"next build"}}"#,
    )
    .unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "npm run build");
}

#[test]
fn build_command_none_without_build_script() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"dev":"next dev"}}"#,
    )
    .unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn build_command_none_without_package_json() {
    let dir = tempdir().unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_config(dir.path(), config);
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

// ── resolve_build_command server fallback ────────────────────

#[test]
fn build_command_server_wins_over_auto_detect() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("server build cmd"), None),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "server build cmd");
}

#[test]
fn build_command_config_wins_over_server() {
    let dir = tempdir().unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("server cmd"), None),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_explicit_wins_over_server() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();

    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("server cmd"), None),
    );
    let result = resolve_build_command(Some("explicit"), dir.path(), &effective);
    assert_eq!(result.unwrap(), "explicit");
}

#[test]
fn build_command_server_used_without_package_json() {
    let dir = tempdir().unwrap();
    // No package.json — auto-detect would return None, but server command should still work
    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(Some("make build"), None),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "make build");
}

#[test]
fn build_command_user_source_used_without_package_json() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(
            Some("make build"),
            Some(nrz::config::BuildSettingSource::User),
        ),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "make build");
}

#[test]
fn build_command_preset_source_without_package_json_skips() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(
            Some("npm run build"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn build_command_preset_source_uses_local_package_manager() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(
            Some("npm run build"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_detected_empty_suppresses_auto_detect() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(None, Some(nrz::config::BuildSettingSource::Detected)),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn build_command_preset_empty_keeps_auto_detect_fallback() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();

    let config = nrz::config::ProjectConfig::default();
    let effective = effective_with_server_settings(
        dir.path(),
        config,
        server_build_settings(None, Some(nrz::config::BuildSettingSource::Preset)),
    );
    let result = resolve_build_command(None, dir.path(), &effective);
    assert_eq!(result.unwrap(), "npm run build");
}

// ── install command source handling ──────────────────────────

#[test]
fn install_command_preset_source_without_package_json_skips() {
    let dir = tempdir().unwrap();
    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("npm install"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert!(result.is_none());
}

#[test]
fn install_command_preset_source_uses_local_package_manager() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("npm install"),
            Some(nrz::config::BuildSettingSource::Preset),
        ),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert_eq!(result.unwrap(), "pnpm install");
}

#[test]
fn install_command_user_source_used_without_package_json() {
    let dir = tempdir().unwrap();
    let effective = effective_with_server_settings(
        dir.path(),
        nrz::config::ProjectConfig::default(),
        server_install_settings(
            Some("make deps"),
            Some(nrz::config::BuildSettingSource::User),
        ),
    );
    let result = resolve_install_command(dir.path(), &effective);
    assert_eq!(result.unwrap(), "make deps");
}

#[test]
fn pnpm_sandbox_build_script_compat_preserves_user_environment() {
    let dir = tempdir().unwrap();

    let (cmd, env) = prepare_install_command_with_sandbox("pnpm install", dir.path(), true, true);

    assert_eq!(cmd, "pnpm install");
    assert!(env.contains(&(
        "npm_config_dangerously_allow_all_builds".to_string(),
        "true".to_string()
    )));
    assert!(env.contains(&(
        "pnpm_config_dangerously_allow_all_builds".to_string(),
        "true".to_string()
    )));
    assert!(
        env.iter()
            .all(|(key, _)| !matches!(key.as_str(), "HOME" | "XDG_CONFIG_HOME")),
        "compat env must not hide user-level npm/pnpm config: {env:?}"
    );
}

#[test]
fn pnpm_sandbox_build_script_compat_uses_existing_builder_marker() {
    let platform_marker = |key: &str| match key {
        "ONREZA" => Some("1".to_string()),
        "CI" => Some("true".to_string()),
        _ => None,
    };
    assert!(running_in_onreza_build_sandbox_from_env(platform_marker));

    let explicit_marker = |key: &str| match key {
        "NRZ_BUILD_SANDBOX" => Some("true".to_string()),
        _ => None,
    };
    assert!(running_in_onreza_build_sandbox_from_env(explicit_marker));

    let onreza_without_ci = |key: &str| match key {
        "ONREZA" => Some("1".to_string()),
        _ => None,
    };
    assert!(!running_in_onreza_build_sandbox_from_env(onreza_without_ci));
}

#[test]
fn pnpm_sandbox_build_script_compat_respects_project_policy() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        "dangerouslyAllowAllBuilds: false\n",
    )
    .unwrap();

    let (cmd, env) = prepare_install_command_with_sandbox("pnpm install", dir.path(), true, true);

    assert_eq!(cmd, "pnpm install");
    assert!(env.is_empty());
}

#[test]
fn pnpm_sandbox_build_script_compat_does_not_treat_false_ignore_scripts_as_policy() {
    let cases = [
        (".npmrc", "ignore-scripts=false\n"),
        (".pnpmrc", "pnpm.ignoreScripts=false\n"),
        ("pnpm-workspace.yaml", "ignoreDepScripts: false\n"),
        ("package.json", r#"{"pnpm":{"ignoreScripts":false}}"#),
    ];

    for (file, contents) in cases {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(file), contents).unwrap();

        let (_, env) = prepare_install_command_with_sandbox("pnpm install", dir.path(), true, true);

        assert!(
            !env.is_empty(),
            "{file}={contents:?} should not disable sandbox pnpm build-script compat"
        );
    }
}

#[test]
fn pnpm_sandbox_build_script_compat_respects_true_ignore_scripts_policy() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".npmrc"), "ignore-scripts=true\n").unwrap();

    let (_, env) = prepare_install_command_with_sandbox("pnpm install", dir.path(), true, true);

    assert!(env.is_empty());
}

#[test]
fn pnpm_install_command_parser_handles_workspace_forms() {
    for cmd in [
        "pnpm install",
        "pnpm i",
        "pnpm -C app install",
        "pnpm --filter web install",
        "pnpm --filter=web install",
        "pnpm@11.1.2 install",
        "corepack pnpm install",
        "cd app && pnpm install",
        r#""pnpm" install"#,
    ] {
        assert!(
            is_pnpm_install_command(cmd),
            "{cmd:?} should be treated as pnpm install"
        );
    }
}

#[test]
fn pnpm_install_command_parser_rejects_non_install_commands() {
    for cmd in [
        "pnpm build",
        "pnpm run install",
        "npm install",
        "corepack yarn install",
        "pnpm --filter web build",
    ] {
        assert!(
            !is_pnpm_install_command(cmd),
            "{cmd:?} should not be treated as pnpm install"
        );
    }
}

#[test]
fn pnpm_build_policy_detection_recognizes_explicit_allowlists() {
    let cases = [
        ("pnpm-workspace.yaml", "onlyBuiltDependencies:\n  - sharp\n"),
        (
            "pnpm-workspace.yml",
            "ignoredBuiltDependencies:\n  - esbuild\n",
        ),
        (".pnpmrc", "only-built-dependencies[]=sharp\n"),
        (
            "package.json",
            r#"{"pnpm":{"onlyBuiltDependencies":["sharp"]}}"#,
        ),
    ];

    for (file, contents) in cases {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(file), contents).unwrap();

        let (_, env) = prepare_install_command_with_sandbox("pnpm install", dir.path(), true, true);

        assert!(
            env.is_empty(),
            "{file}={contents:?} should disable sandbox pnpm compat"
        );
    }
}

#[test]
fn prepare_deploy_files_prunes_next_cache_only() {
    let manifest: build_manifest::Manifest = serde_json::from_value(serde_json::json!({
        "version": 1,
        "layers": [
            {"name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js"}
        ],
        "routes": [
            {"pattern": "^/.*$", "layer": "server"}
        ]
    }))
    .unwrap();
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"next":"15.0.0","react":"19.0.0"}}"#,
    )
    .unwrap();
    let detection = crate::detect::detect_with_framework_override(dir.path(), None);
    let files = vec![
        fe(".next/cache/webpack/client.json", 10, "aa"),
        fe(
            "node_modules/@next/swc-linux-x64-gnu/package.json",
            20,
            "bb",
        ),
        fe("server.js", 30, "cc"),
    ];

    let deployable = prepare_deploy_files(&manifest, files, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![
            "node_modules/@next/swc-linux-x64-gnu/package.json",
            "server.js"
        ]
    );
}

#[test]
fn prepare_deploy_files_prunes_root_static_metadata() {
    let manifest = build_manifest::generate_static_manifest();
    let detection = make_detection("static-html", None);
    let files = vec![
        fe("index.html", 10, "aa"),
        fe("assets/app.js", 20, "bb"),
        fe(".onreza/manifest.json", 30, "cc"),
        fe("package.json", 40, "dd"),
        fe("node_modules/pkg/index.js", 50, "ee"),
        fe(".env.local", 60, "ff"),
        fe("onreza.toml", 70, "gg"),
    ];

    let deployable = prepare_deploy_files(&manifest, files, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["index.html", "assets/app.js"]);
}

#[test]
fn prepare_deploy_files_keeps_static_build_output_node_modules_assets() {
    let output = tempdir().unwrap();
    fs::create_dir_all(output.path().join("node_modules/pkg")).unwrap();
    fs::write(
        output.path().join("index.html"),
        r#"<script type="module" src="/node_modules/pkg/index.js"></script>"#,
    )
    .unwrap();
    fs::write(
        output.path().join("node_modules/pkg/index.js"),
        "export const ok = true;",
    )
    .unwrap();

    let manifest = build_manifest::generate_static_manifest();
    let detection = make_detection("static-html", None);
    let scanned = scan_runtime_artifact(output.path(), &RuntimeArtifactScan::All).unwrap();
    let deployable = prepare_artifact_files(
        &manifest,
        scanned,
        &detection,
        crate::artifact::ArtifactRootScope::BuildOutput,
        true,
    )
    .deployable_entries();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert!(paths.contains(&"index.html"));
    assert!(paths.contains(&"node_modules/pkg/index.js"));
    source_bundle_v1::build_source_bundle_plan(output.path(), &manifest, &deployable).unwrap();
}

#[cfg(unix)]
#[test]
fn prepare_deploy_files_preserves_build_only_target_for_deployable_symlink() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    std::os::unix::fs::symlink("../package.json", dir.path().join("assets/pkg")).unwrap();

    let manifest = build_manifest::generate_static_manifest();
    let detection = make_detection("static-html", None);
    let scanned = scan_runtime_artifact(dir.path(), &RuntimeArtifactScan::All).unwrap();
    let deployable = prepare_deploy_files(&manifest, scanned, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert!(paths.contains(&"assets/pkg"));
    assert!(paths.contains(&"package.json"));
    source_bundle_v1::build_source_bundle_plan(dir.path(), &manifest, &deployable).unwrap();
}

#[test]
fn prepare_deploy_files_keeps_package_json_for_compute_runtime() {
    let manifest = build_manifest::generate_compute_manifest("server.js");
    let detection = make_detection("express", None);
    let files = vec![
        fe("package.json", 10, "aa"),
        fe(".onreza/manifest.json", 20, "bb"),
        fe("server.js", 30, "cc"),
    ];

    let deployable = prepare_deploy_files(&manifest, files, &detection, true).unwrap();
    let paths = deployable
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec!["package.json", "server.js"]);
}

#[test]
fn node_process_runtime_artifact_uses_project_root_for_nestjs() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{
            "main": "dist/src/main.js",
            "dependencies": {
                "@nestjs/core": "10.0.0",
                "rxjs": "7.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("dist/src")).unwrap();
    fs::write(
        dir.path().join("dist/src/main.js"),
        "require('@nestjs/core')",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        dir.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.ts"), "source only").unwrap();

    let detection = crate::detect::detect_with_framework_override(dir.path(), None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        dir.path(),
        dir.path(),
        dir.path().join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, dir.path());
    let compute_layer = artifact
        .manifest
        .layers
        .iter()
        .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
        .unwrap();
    assert_eq!(compute_layer.directory, ".");
    assert_eq!(compute_layer.entry.as_deref(), Some("dist/src/main.js"));

    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"dist/src/main.js"));
    assert!(paths.contains(&"node_modules/@nestjs/core/index.js"));
    assert!(paths.contains(&"package.json"));
    assert!(!paths.contains(&"src/main.ts"));

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
    assert_eq!(plan.logical_manifest.entrypoints, vec!["dist/src/main.js"]);
    let nest_runtime = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "node_modules/@nestjs/core/index.js")
        .unwrap();
    assert_eq!(
        nest_runtime.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
}

#[cfg(unix)]
#[test]
fn node_process_runtime_artifact_prefers_workspace_root_for_hoisted_app_symlink() {
    let workspace = tempdir().unwrap();
    let app = workspace.path().join("apps/api");
    fs::create_dir_all(app.join("dist/src")).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::write(app.join("dist/src/main.js"), "require('@nestjs/core')").unwrap();

    fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        workspace.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(app.join("node_modules/@nestjs")).unwrap();
    std::os::unix::fs::symlink(
        "../../../../node_modules/@nestjs/core",
        app.join("node_modules/@nestjs/core"),
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        workspace.path(),
        &app,
        app.join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, workspace.path());
    let compute_layer = artifact
        .manifest
        .layers
        .iter()
        .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
        .unwrap();
    assert_eq!(compute_layer.directory, ".");
    assert_eq!(
        compute_layer.entry.as_deref(),
        Some("apps/api/dist/src/main.js")
    );

    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"apps/api/node_modules/@nestjs/core"));
    assert!(paths.contains(&"node_modules/@nestjs/core/index.js"));

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
    let symlink = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "apps/api/node_modules/@nestjs/core")
        .unwrap();
    assert_eq!(
        symlink.entry_type,
        Some(source_bundle_v1::SourceLogicalManifestEntryType::Symlink)
    );
    assert_eq!(
        symlink.link_target.as_deref(),
        Some("../../../../node_modules/@nestjs/core")
    );
}

#[test]
fn node_process_runtime_artifact_includes_workspace_hoisted_deps_with_app_node_modules() {
    let workspace = tempdir().unwrap();
    let app = workspace.path().join("apps/api");
    fs::create_dir_all(app.join("dist/src")).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0",
                "local-only": "1.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::write(
        app.join("dist/src/main.js"),
        "require('@nestjs/core'); require('local-only')",
    )
    .unwrap();

    fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        workspace.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(app.join("node_modules/local-only")).unwrap();
    fs::write(
        app.join("node_modules/local-only/index.js"),
        "module.exports = {}",
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        workspace.path(),
        &app,
        app.join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, workspace.path());
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"node_modules/@nestjs/core/index.js"));
    assert!(paths.contains(&"apps/api/node_modules/local-only/index.js"));

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
    assert_eq!(
        plan.logical_manifest.entrypoints,
        vec!["apps/api/dist/src/main.js"]
    );
}

#[cfg(unix)]
#[test]
fn node_process_runtime_artifact_includes_workspace_package_symlink_targets() {
    let workspace = tempdir().unwrap();
    let app = workspace.path().join("apps/api");
    let shared = workspace.path().join("packages/shared");
    fs::create_dir_all(app.join("dist/src")).unwrap();
    fs::create_dir_all(&shared).unwrap();
    fs::write(
        app.join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0",
                "@scope/shared": "workspace:*"
            }
        }"#,
    )
    .unwrap();
    fs::write(
        app.join("dist/src/main.js"),
        "require('@nestjs/core'); require('@scope/shared')",
    )
    .unwrap();
    fs::write(shared.join("package.json"), r#"{"main":"index.js"}"#).unwrap();
    fs::write(shared.join("index.js"), "module.exports = {}").unwrap();

    fs::create_dir_all(workspace.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        workspace.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(workspace.path().join("node_modules/@scope")).unwrap();
    std::os::unix::fs::symlink(
        "../../packages/shared",
        workspace.path().join("node_modules/@scope/shared"),
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(&app, None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        workspace.path(),
        &app,
        app.join("dist"),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    assert_eq!(artifact.root_dir, workspace.path());
    let scanned = scan_runtime_artifact(&artifact.root_dir, &artifact.scan).unwrap();
    let paths = scanned
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"node_modules/@scope/shared"));
    assert!(paths.contains(&"packages/shared/index.js"));

    let deployable = prepare_deploy_files(&artifact.manifest, scanned, &detection, true).unwrap();
    let plan = source_bundle_v1::build_source_bundle_plan(
        &artifact.root_dir,
        &artifact.manifest,
        &deployable,
    )
    .unwrap();
    let shared_runtime = plan
        .logical_manifest
        .files
        .iter()
        .find(|file| file.path == "packages/shared/index.js")
        .unwrap();
    assert_eq!(
        shared_runtime.role,
        source_bundle_v1::SourceLogicalManifestFileRole::Compute
    );
}

#[test]
fn node_process_runtime_artifact_falls_back_when_build_output_outside_project() {
    let project = tempdir().unwrap();
    let external_output = tempdir().unwrap();
    fs::write(
        project.path().join("package.json"),
        r#"{
            "dependencies": {
                "@nestjs/core": "10.0.0"
            }
        }"#,
    )
    .unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { NestFactory } from '@nestjs/core';",
    )
    .unwrap();
    fs::create_dir_all(project.path().join("node_modules/@nestjs/core")).unwrap();
    fs::write(
        project.path().join("node_modules/@nestjs/core/index.js"),
        "module.exports = {}",
    )
    .unwrap();
    fs::create_dir_all(external_output.path().join("src")).unwrap();
    fs::write(
        external_output.path().join("src/main.js"),
        "require('@nestjs/core')",
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(project.path(), None);
    assert_eq!(detection.framework, "nestjs");
    let manifest = build_manifest::generate_compute_manifest("src/main.js");

    let artifact = resolve_runtime_artifact(
        project.path(),
        project.path(),
        external_output.path().to_path_buf(),
        manifest,
        &detection,
        true,
    )
    .unwrap();

    // Build output lives outside the project, so relocation can't apply: the
    // deploy degrades to scanning the build output as-is instead of erroring.
    assert_eq!(artifact.root_dir, external_output.path());
    assert!(matches!(artifact.scan, RuntimeArtifactScan::All));
    let compute_layer = artifact
        .manifest
        .layers
        .iter()
        .find(|layer| layer.target == build_manifest::LayerTarget::Compute)
        .unwrap();
    assert_eq!(compute_layer.directory, ".");
    assert_eq!(compute_layer.entry.as_deref(), Some("src/main.js"));
}

// ── framework preset source handling ─────────────────────────

#[test]
fn server_framework_other_does_not_mask_local_vite_detection() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"vite":"^5.0.0"},"scripts":{"start":"vite --host","build":"vite build"}}"#,
    )
    .unwrap();

    let detection = crate::detect::detect_with_framework_override(
        dir.path(),
        authoritative_server_framework_preset(Some("other")),
    );

    assert_eq!(detection.framework, "vite");
    assert_eq!(
        detection.suggested_compute,
        crate::detect::types::ComputeType::Static
    );
}

#[test]
fn server_framework_specific_preset_is_usable_when_local_framework_absent() {
    assert_eq!(
        authoritative_server_framework_preset(Some("vite")),
        Some("vite")
    );
    assert_eq!(authoritative_server_framework_preset(Some("other")), None);
}

// ── ProjectInfo deserialization ──────────────────────────────

#[test]
fn project_info_deserializes_camel_case() {
    let json = r#"{
        "id": "proj_123",
        "frameworkPreset": "vite",
        "installCommand": "npm ci",
        "installCommandSource": "DETECTED",
        "buildCommand": "npm run build",
        "buildCommandSource": "USER",
        "outputDirectory": "dist",
        "outputDirectorySource": "USER"
    }"#;
    let info: ProjectInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.framework_preset.unwrap(), "vite");
    assert_eq!(info.install_command.unwrap(), "npm ci");
    assert_eq!(
        info.install_command_source.unwrap(),
        crate::build::BuildSettingSource::Detected
    );
    assert_eq!(info.build_command.unwrap(), "npm run build");
    assert_eq!(
        info.build_command_source.unwrap(),
        crate::build::BuildSettingSource::User
    );
    assert_eq!(info.output_directory.unwrap(), "dist");
    assert_eq!(
        info.output_directory_source.unwrap(),
        crate::build::BuildSettingSource::User
    );
}

#[test]
fn project_info_optional_fields_default_to_none() {
    let json = r#"{"id": "proj_123"}"#;
    let info: ProjectInfo = serde_json::from_str(json).unwrap();
    assert!(info.install_command.is_none());
    assert!(info.install_command_source.is_none());
    assert!(info.build_command.is_none());
    assert!(info.build_command_source.is_none());
    assert!(info.output_directory.is_none());
    assert!(info.output_directory_source.is_none());
}

// (Content-Type guessing tests removed: blob/bundle PUTs go through `put_blob`,
// which omits Content-Type so the SigV4 signature stays valid — the helper
// `guess_content_type` is no longer in the codebase.)

// ── framework_static_hint tests ──────────────────────────────

#[test]
fn static_hint_known_frameworks_non_empty() {
    assert!(!framework_static_hint("nextjs").is_empty());
    assert!(!framework_static_hint("nuxt").is_empty());
    assert!(!framework_static_hint("sveltekit").is_empty());
    assert!(!framework_static_hint("astro").is_empty());
    assert!(!framework_static_hint("react-router").is_empty());
    assert!(!framework_static_hint("remix").is_empty());
    assert!(!framework_static_hint("solidstart").is_empty());
    assert!(!framework_static_hint("qwik").is_empty());
    assert!(!framework_static_hint("analog").is_empty());
    assert!(framework_static_hint("nextjs").contains("export"));
    assert!(framework_static_hint("react-router").contains("ssr: false"));
    assert!(framework_static_hint("remix").contains("ssr: false"));
    assert!(framework_static_hint("solidstart").contains("ssr: false"));
    assert!(framework_static_hint("analog").contains("ssr: false"));
}

#[test]
fn static_hint_unknown_returns_empty() {
    assert!(framework_static_hint("vite").is_empty());
    assert!(framework_static_hint("unknown").is_empty());
}

// ── compute/manifest contract tests ─────────────────────────

#[test]
fn process_with_manifest_is_ok() {
    // Manifest can declare COMPUTE layers — PROCESS + manifest is valid.
    assert!(validate_compute_manifest_contract(ComputeType::Process, true).is_ok());
}

#[test]
fn static_without_manifest_is_ok() {
    assert!(validate_compute_manifest_contract(ComputeType::Static, false).is_ok());
}

#[test]
fn static_with_manifest_is_ok() {
    // Manifest can declare only STATIC layers — STATIC + manifest is valid.
    assert!(validate_compute_manifest_contract(ComputeType::Static, true).is_ok());
}

#[test]
fn process_without_manifest_is_error() {
    // Safety net: PROCESS auto-generation should always produce a manifest before
    // validate_compute_manifest_contract is called, so reaching here with has_manifest=false
    // is an unexpected state.
    let err = validate_compute_manifest_contract(ComputeType::Process, false)
        .expect_err("PROCESS without manifest should fail");
    assert!(
        err.to_string().contains("Internal error"),
        "unexpected error: {err}"
    );
}

#[test]
fn create_deployment_body_serializes_required_fields() {
    let body = CreateDeploymentBody {
        manifest: serde_json::json!({ "version": 1 }),
        files: vec![],
        production: None,
        branch: None,
        commit_sha: "deadbeef".into(),
        functions: None,
    };

    let value = serde_json::to_value(&body).unwrap();
    assert!(value.get("manifest").is_some());
    assert!(value.get("production").is_none());
    assert_eq!(
        value.get("commitSha").and_then(|v| v.as_str()),
        Some("deadbeef")
    );
    assert!(value.get("computeType").is_none());
    assert!(value.get("processEntry").is_none());
    assert!(value.get("bundle").is_none());
    assert!(value.get("functions").is_none());
}

#[test]
fn create_deployment_body_serializes_functions_payload() {
    let body = CreateDeploymentBody {
        manifest: serde_json::json!({ "version": 1 }),
        files: vec![],
        production: Some(false),
        branch: None,
        commit_sha: "deadbeef".into(),
        functions: conform_functions_to_wire_contract(Some(
            crate::functions::FunctionPublishPayload {
                origin: "DEPLOYMENT",
                functions: vec![],
                edge_rules: None,
                edge_rules_force: false,
                generated_edge_rule_sets: Vec::new(),
            },
        ))
        .unwrap(),
    };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(
        value
            .get("functions")
            .and_then(|functions| functions.get("origin"))
            .and_then(|origin| origin.as_str()),
        Some("DEPLOYMENT")
    );
}

#[test]
fn functions_payload_serializes_edge_rules_force() {
    let value =
        conform_functions_to_wire_contract(Some(crate::functions::FunctionPublishPayload {
            origin: "DEPLOYMENT",
            functions: vec![],
            edge_rules: Some(serde_json::json!({
                "schemaVersion": "EDGE_RULE_SET_V1",
                "source": { "origin": "build" },
                "rules": [
                    {
                        "id": "allow-all",
                        "action": { "type": "allow" }
                    }
                ]
            })),
            edge_rules_force: true,
            generated_edge_rule_sets: Vec::new(),
        }))
        .unwrap()
        .unwrap();

    assert_eq!(value["edgeRulesForce"], true);
}

#[test]
fn build_functions_payload_generates_nextjs_edge_rules_when_local_rules_absent() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".onreza")).unwrap();
    fs::write(
        tmp.path().join(".onreza/next-adapter-output.json"),
        r#"
{
  "version": 1,
  "adapter": { "name": "@onreza/nrz-next-adapter", "version": "0.34.1" },
  "routing": {
    "beforeMiddleware": [
      {
        "source": "/old",
        "headers": { "Location": "/new" },
        "status": 308
      }
    ]
  },
  "outputs": {}
}
"#,
    )
    .unwrap();

    let payload = build_functions_payload(
        &nrz::config::ProjectConfig::default(),
        tmp.path(),
        true,
        false,
    )
    .unwrap()
    .expect("generated Next.js Edge Rules should create a functions payload");
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["origin"], "DEPLOYMENT");
    assert_eq!(value["functions"].as_array().unwrap().len(), 0);
    assert!(value.get("edgeRules").is_none());
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["producer"],
        "nextjs-adapter"
    );
    assert!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]
            .get("source")
            .is_none()
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"][0]["action"]["type"],
        "redirect"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"][0]["action"]["target"],
        "/new"
    );
}

#[test]
fn build_functions_payload_sends_empty_nextjs_generated_contribution_for_clearing() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".onreza")).unwrap();
    fs::write(
        tmp.path().join(".onreza/next-adapter-output.json"),
        r#"
{
  "version": 1,
  "adapter": { "name": "@onreza/nrz-next-adapter", "version": "0.34.1" },
  "routing": {},
  "outputs": {}
}
"#,
    )
    .unwrap();

    let payload = build_functions_payload(
        &nrz::config::ProjectConfig::default(),
        tmp.path(),
        true,
        false,
    )
    .unwrap()
    .expect("empty Next.js generated contribution should clear stale adapter rules");
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["origin"], "DEPLOYMENT");
    assert_eq!(value["functions"].as_array().unwrap().len(), 0);
    assert!(value.get("edgeRules").is_none());
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["producer"],
        "nextjs-adapter"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn build_functions_payload_sends_user_and_nextjs_generated_rules_separately() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".onreza")).unwrap();
    fs::write(
        tmp.path().join(".onreza/next-adapter-output.json"),
        r#"
{
  "version": 1,
  "adapter": { "name": "@onreza/nrz-next-adapter", "version": "0.34.1" },
  "routing": {
    "beforeMiddleware": [
      {
        "source": "/old",
        "headers": { "Location": "/new" },
        "status": 308
      }
    ]
  },
  "outputs": {}
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("onreza.rules.toml"),
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "user-owned"
action = { type = "allow" }
"#,
    )
    .unwrap();

    let payload = build_functions_payload(
        &nrz::config::ProjectConfig::default(),
        tmp.path(),
        true,
        false,
    )
    .unwrap()
    .expect("user Edge Rules should create a functions payload");
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["edgeRules"]["rules"][0]["id"], "user-owned");
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["producer"],
        "nextjs-adapter"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"][0]["action"]["target"],
        "/new"
    );
}

#[test]
fn deploy_env_preview_forces_preview_override() {
    let env = vec!["preview".to_string()];
    assert_eq!(
        resolve_deploy_production_override(false, &env).unwrap(),
        Some(false)
    );
}

#[test]
fn deploy_without_env_preserves_branch_inference() {
    assert_eq!(
        resolve_deploy_production_override(false, &[]).unwrap(),
        None
    );
}

#[test]
fn deploy_prod_conflicts_with_preview_env() {
    let env = vec!["preview".to_string()];
    let error = resolve_deploy_production_override(true, &env).unwrap_err();
    assert!(error.to_string().contains("conflicts"));
}

#[test]
fn stage_deployment_functions_path_targets_project_activation_family() {
    assert_eq!(
        stage_deployment_functions_path("project-1", "deployment-1"),
        "/v1/projects/project-1/function-activations/deployments/deployment-1/functions/stage"
    );
}

#[test]
fn prepare_upload_request_serializes_source_bundle_v1_contract() {
    let logical_manifest = source_bundle_v1::SourceLogicalManifest {
        schema_version: source_bundle_v1::SOURCE_BUNDLE_SCHEMA_VERSION.to_string(),
        capabilities: vec![],
        files: vec![source_bundle_v1::SourceLogicalManifestFile {
            path: "index.html".into(),
            sha256: "a".repeat(64),
            size: 12,
            entry_type: None,
            link_target: None,
            content_type: Some("text/html; charset=utf-8".into()),
            role: source_bundle_v1::SourceLogicalManifestFileRole::Static,
            layer_name: Some("static".into()),
        }],
        layers: vec![source_bundle_v1::SourceLogicalManifestLayer {
            name: "static".into(),
            target: source_bundle_v1::SourceLogicalManifestLayerTarget::Static,
            root_path: None,
            entrypoint: None,
            runtime_config: None,
        }],
        routes: vec![],
        entrypoints: vec![],
    };
    let logical_manifest_sha256 =
        source_bundle_v1::compute_logical_manifest_sha256(&logical_manifest).unwrap();
    let logical_manifest_summary = source_bundle_v1::SourceLogicalManifestSummary {
        file_count: 1,
        logical_static_bytes: "12".into(),
        artifact_size_bytes: "0".into(),
        max_static_file_size_bytes: "12".into(),
    };
    let body = CliPrepareUploadRequest {
        deployment_id: Uuid::now_v7(),
        workspace_id: Uuid::now_v7(),
        project_id: Uuid::now_v7(),
        deployment_attempt_id: Uuid::now_v7(),
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".into(),
        cli_protocol_version: source_bundle_v1::CLI_PROTOCOL_VERSION.try_into().unwrap(),
        logical_manifest_summary: to_contract_manifest_summary(&logical_manifest_summary).unwrap(),
        logical_manifest_sha256: logical_manifest_sha256.as_str().try_into().unwrap(),
        source_format: source_bundle_v1::SOURCE_BUNDLE_FORMAT.into(),
        source_sha256: "b".repeat(64).as_str().try_into().unwrap(),
        source_size_bytes: "4096".try_into().unwrap(),
        multipart: None,
        source_upload_recovery: None,
    };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["artifactFormat"], "SOURCE_BUNDLE_V1");
    assert_eq!(
        value["cliProtocolVersion"],
        "source-bundle-v1-embedded-manifest"
    );
    assert_eq!(value["sourceFormat"], "tar.zst");
    assert_eq!(value["sourceSizeBytes"], "4096");
    assert_eq!(value["logicalManifestSummary"]["fileCount"], 1);
    assert_eq!(value["logicalManifestSha256"], logical_manifest_sha256);
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        [
            "artifactFormat",
            "cliProtocolVersion",
            "deploymentAttemptId",
            "deploymentId",
            "logicalManifestSha256",
            "logicalManifestSummary",
            "operationId",
            "projectId",
            "sourceFormat",
            "sourceSha256",
            "sourceSizeBytes",
            "workspaceId",
        ]
    );
}

#[test]
fn prepare_upload_request_serializes_source_upload_recovery_context() {
    let failed_upload_session_id = Uuid::now_v7();
    let body = CliPrepareUploadRequest {
        deployment_id: Uuid::now_v7(),
        workspace_id: Uuid::now_v7(),
        project_id: Uuid::now_v7(),
        deployment_attempt_id: Uuid::now_v7(),
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".into(),
        cli_protocol_version: source_bundle_v1::CLI_PROTOCOL_VERSION.try_into().unwrap(),
        logical_manifest_summary: CliLogicalManifestSummary {
            file_count: 1,
            logical_static_bytes: "12".try_into().unwrap(),
            artifact_size_bytes: "0".try_into().unwrap(),
            max_static_file_size_bytes: "12".try_into().unwrap(),
        },
        logical_manifest_sha256: "a".repeat(64).as_str().try_into().unwrap(),
        source_format: source_bundle_v1::SOURCE_BUNDLE_FORMAT.into(),
        source_sha256: "b".repeat(64).as_str().try_into().unwrap(),
        source_size_bytes: "4096".try_into().unwrap(),
        multipart: None,
        source_upload_recovery: Some(CliPrepareUploadSourceUploadRecovery {
            failed_upload_session_id,
            reason: SOURCE_UPLOAD_RECOVERY_CONDITIONAL_PRECONDITION_FAILED.to_string(),
        }),
    };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(
        value["sourceUploadRecovery"]["reason"],
        "conditional-precondition-failed"
    );
    assert_eq!(
        value["sourceUploadRecovery"]["failedUploadSessionId"],
        failed_upload_session_id.to_string()
    );
}

#[test]
fn upload_failed_request_serializes_source_bundle_v1_contract() {
    let body = CliUploadFailedRequest {
        deployment_id: Uuid::now_v7(),
        upload_session_id: Uuid::now_v7(),
        deployment_attempt_id: Uuid::now_v7(),
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".into(),
        source_artifact_id: "c".repeat(64).as_str().try_into().unwrap(),
        error_code: SOURCE_UPLOAD_PUT_FAILED.try_into().unwrap(),
        error_log: "S3 rejected the upload".try_into().unwrap(),
    };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["artifactFormat"], "SOURCE_BUNDLE_V1");
    assert_eq!(value["errorCode"], SOURCE_UPLOAD_PUT_FAILED);
    assert_eq!(value["errorLog"], "S3 rejected the upload");
    assert!(value.get("sourceSha256").is_none());
    assert!(value.get("sourceSizeBytes").is_none());
}

#[test]
fn upload_complete_request_serializes_source_bundle_v1_contract() {
    let body = CliUploadCompleteRequest {
        deployment_id: Uuid::now_v7(),
        upload_session_id: Uuid::now_v7(),
        deployment_attempt_id: Uuid::now_v7(),
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".into(),
        source_artifact_id: "c".repeat(64).as_str().try_into().unwrap(),
        source_sha256: "d".repeat(64).as_str().try_into().unwrap(),
        source_size_bytes: "4096".try_into().unwrap(),
        logical_manifest_sha256: "e".repeat(64).as_str().try_into().unwrap(),
    };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["artifactFormat"], "SOURCE_BUNDLE_V1");
    assert_eq!(value["sourceSizeBytes"], "4096");
    assert!(value["deploymentId"].is_string());
    assert!(value["uploadSessionId"].is_string());
}

#[test]
fn multipart_complete_request_serializes_source_bundle_v1_contract() {
    let body = CliMultipartCompleteRequest {
        deployment_id: Uuid::now_v7(),
        upload_session_id: Uuid::now_v7(),
        deployment_attempt_id: Uuid::now_v7(),
        operation_id: Uuid::now_v7(),
        artifact_format: "SOURCE_BUNDLE_V1".into(),
        source_artifact_id: "c".repeat(64).as_str().try_into().unwrap(),
        upload_id: "upload-id".try_into().unwrap(),
        parts: vec![CliMultipartCompletePart {
            part_number: std::num::NonZeroU64::new(1).unwrap(),
            e_tag: "\"etag\"".try_into().unwrap(),
        }],
    };

    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["artifactFormat"], "SOURCE_BUNDLE_V1");
    assert_eq!(value["parts"][0]["partNumber"], 1);
    assert_eq!(value["parts"][0]["eTag"], "\"etag\"");
}

#[test]
fn upload_failed_reporting_uses_only_source_object_upload_failure_code() {
    assert_eq!(SOURCE_UPLOAD_PUT_FAILED, "SOURCE_UPLOAD_PUT_FAILED");
}

#[test]
fn upload_failure_log_is_bounded_for_server_contract() {
    let long = "x".repeat(MAX_UPLOAD_FAILURE_LOG_LENGTH + 10);
    let error = anyhow::anyhow!("{long}");
    let truncated = upload_failure_log(&error);
    assert_eq!(truncated.len(), MAX_UPLOAD_FAILURE_LOG_LENGTH);
}

#[test]
fn upload_failure_log_redacts_presigned_url_query_before_reporting() {
    let error = anyhow::anyhow!(
        "error sending request for url (https://bucket.s3.example/source.tar.zst?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Signature=signature-secret&X-Amz-Credential=credential-secret)"
    )
    .context("failed to upload SOURCE_BUNDLE_V1 source object");

    let log = upload_failure_log(&error);

    assert!(log.contains("failed to upload SOURCE_BUNDLE_V1 source object"));
    assert!(
        log.contains("https://bucket.s3.example/source.tar.zst?REDACTED)"),
        "{log}"
    );
    assert!(!log.contains("X-Amz-Signature"), "{log}");
    assert!(!log.contains("signature-secret"), "{log}");
    assert!(!log.contains("credential-secret"), "{log}");
}

#[test]
fn control_plane_backpressure_honors_retry_after_above_backoff_cap() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
        code: "SERVICE_UNAVAILABLE".into(),
        message: "Artifact ingest service is overloaded".into(),
        retry_after_seconds: Some(30),
        details: None,
    }
    .into();

    assert_eq!(
        classify_prepare_upload_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::ControlPlaneBackpressure,
            retry_after: Some(Duration::from_secs(30)),
        })
    );
    assert_eq!(
        retry_delay_with_hint(
            Some(Duration::from_secs(30)),
            Duration::from_millis(500),
            Duration::from_secs(5),
            Duration::from_secs(60),
        ),
        Duration::from_secs(30)
    );
}

#[test]
fn retry_delay_caps_only_fallback_backoff_and_remaining_budget() {
    assert_eq!(
        retry_delay_with_hint(
            None,
            Duration::from_secs(30),
            Duration::from_secs(5),
            Duration::from_secs(60),
        ),
        Duration::from_secs(5)
    );
    assert_eq!(
        retry_delay_with_hint(
            Some(Duration::from_secs(30)),
            Duration::from_millis(500),
            Duration::from_secs(5),
            Duration::from_secs(60),
        ),
        Duration::from_secs(30)
    );
    assert_eq!(
        retry_delay_with_hint(
            Some(Duration::from_secs(30)),
            Duration::from_millis(500),
            Duration::from_secs(5),
            Duration::from_secs(10),
        ),
        Duration::from_secs(10)
    );
}

#[tokio::test]
async fn source_single_put_sends_conditional_header_from_wire_hint() {
    let content = Bytes::from_static(b"hello source");
    let sha256 = sha256_hex(&content);
    let (url, _server) = spawn_conditional_pack_put_mock().await;
    let client = ApiClient::anonymous().unwrap();
    let headers = PresignedPutHeaders::if_none_match_any();

    upload_single_put(
        &client,
        SinglePutUpload {
            url: &url,
            bytes: content,
            content_length: 12,
            sha256: &sha256,
            headers: &headers,
            verify_head: None,
            label: "SOURCE_BUNDLE_V1 source object".into(),
        },
    )
    .await
    .expect("single SOURCE_BUNDLE_V1 PUT should include If-None-Match");
}

#[test]
fn upload_complete_incomplete_response_is_retryable() {
    let attempt = classify_upload_complete_response(UploadCompleteResponse::Incomplete {
        missing_source_object: true,
    })
    .unwrap();

    assert_eq!(
        attempt,
        SourceCompletionAttempt::Retry(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::S3Visibility,
            retry_after: None,
        })
    );
}

#[test]
fn upload_complete_operation_in_progress_error_is_retryable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::CONFLICT,
        code: "OPERATION_IN_PROGRESS".into(),
        message: "upload-complete: waiting on owner verify".into(),
        retry_after_seconds: None,
        details: None,
    }
    .into();

    assert_eq!(
        classify_upload_complete_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::OwnerVerifyInProgress,
            retry_after: None,
        })
    );
}

#[test]
fn upload_complete_control_plane_backpressure_is_retryable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
        code: "SERVICE_UNAVAILABLE".into(),
        message: "Artifact ingest service is overloaded".into(),
        retry_after_seconds: Some(2),
        details: None,
    }
    .into();

    assert_eq!(
        classify_upload_complete_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::ControlPlaneBackpressure,
            retry_after: Some(Duration::from_secs(2)),
        })
    );
}

#[test]
fn prepare_upload_operation_in_progress_error_is_retryable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::CONFLICT,
        code: "OPERATION_IN_PROGRESS".into(),
        message: "prepare-upload is already in progress".into(),
        retry_after_seconds: None,
        details: None,
    }
    .into();

    assert_eq!(
        classify_prepare_upload_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::PrepareUploadInProgress,
            retry_after: None,
        })
    );
}

fn limit_exceeded_api_error() -> anyhow::Error {
    crate::api::StructuredApiError {
        status: reqwest::StatusCode::FORBIDDEN,
        code: "LIMIT_EXCEEDED".into(),
        message: "Deployment exceeds maximum file count (15503 / 5000).".into(),
        retry_after_seconds: None,
        details: Some(serde_json::json!({"limitType": "maxDeploymentFiles", "limit": 5000})),
    }
    .into()
}

#[test]
fn prepare_upload_limit_error_in_json_mode_is_reported_on_both_channels() {
    // JSON mode: report_terminal_error emits the stderr frame (Builder) + stdout
    // envelope with code (CLI/automation) and returns AlreadyReportedError so
    // main does not re-emit a code-less envelope.
    let mapped = prepare_upload_terminal_error(limit_exceeded_api_error(), true);
    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_some(),
        "limit error in JSON mode must be fully reported (AlreadyReportedError), got: {mapped:#}"
    );
}

#[test]
fn prepare_upload_limit_error_in_human_mode_stays_contextual() {
    // Human mode: no machine envelope — a contextual error main prints as "Error:".
    let mapped = prepare_upload_terminal_error(limit_exceeded_api_error(), false);
    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_none()
    );
    assert!(mapped.to_string().contains("failed to prepare upload"));
}

#[test]
fn prepare_upload_platform_error_in_json_mode_is_not_swallowed() {
    // A non-limit (platform-fault) structured error must NOT be marked
    // already-reported — main keeps ownership so it surfaces / routes to Sentry.
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        code: "INTERNAL".into(),
        message: "boom".into(),
        retry_after_seconds: None,
        details: None,
    }
    .into();
    let mapped = prepare_upload_terminal_error(error, true);
    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_none()
    );
    assert!(mapped.to_string().contains("failed to prepare upload"));
}

#[tokio::test]
async fn prepare_upload_transport_error_is_retryable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let error = anyhow::Error::new(
        reqwest::Client::new()
            .post(format!("http://{addr}/v1/deployments/d/prepare-upload"))
            .send()
            .await
            .expect_err("closed listener should refuse the request"),
    )
    .context("request failed: POST /v1/deployments/d/prepare-upload");

    assert_eq!(
        classify_prepare_upload_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::TransportAmbiguous,
            retry_after: None,
        })
    );
}

#[test]
fn upload_failed_control_plane_backpressure_is_retryable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
        code: "SERVICE_UNAVAILABLE".into(),
        message: "Artifact ingest service is overloaded".into(),
        retry_after_seconds: Some(2),
        details: None,
    }
    .into();

    assert_eq!(
        classify_upload_failed_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::ControlPlaneBackpressure,
            retry_after: Some(Duration::from_secs(2)),
        })
    );
}

#[test]
fn upload_failed_operation_in_progress_error_is_retryable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::CONFLICT,
        code: "OPERATION_IN_PROGRESS".into(),
        message: "upload-failed is already running".into(),
        retry_after_seconds: None,
        details: None,
    }
    .into();

    assert_eq!(
        classify_upload_failed_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::FailureReportInProgress,
            retry_after: None,
        })
    );
}

#[test]
fn multipart_complete_operation_in_progress_error_is_retryable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::CONFLICT,
        code: "OPERATION_IN_PROGRESS".into(),
        message: "multipart-complete is already running".into(),
        retry_after_seconds: None,
        details: None,
    }
    .into();

    assert_eq!(
        classify_multipart_complete_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::CompletionInProgress,
            retry_after: None,
        })
    );
}

#[tokio::test]
async fn multipart_complete_transport_error_is_retryable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let error = anyhow::Error::new(
        reqwest::Client::new()
            .post(format!("http://{addr}/v1/deployments/d/multipart-complete"))
            .send()
            .await
            .expect_err("closed listener should refuse the request"),
    )
    .context("request failed: POST /v1/deployments/d/multipart-complete");

    assert_eq!(
        classify_multipart_complete_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::TransportAmbiguous,
            retry_after: None,
        })
    );
}

#[test]
fn upload_complete_source_object_incomplete_error_is_retryable() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: reqwest::StatusCode::BAD_REQUEST,
        code: "VALIDATION_ERROR".into(),
        message: "Upload is incomplete: source object is not visible in S3 yet.".into(),
        retry_after_seconds: None,
        details: Some(serde_json::json!({ "field": "sourceObject" })),
    }
    .into();

    assert_eq!(
        classify_upload_complete_retry_error(&error),
        Some(SourceControlPlaneRetry {
            reason: SourceControlPlaneRetryReason::S3Visibility,
            retry_after: None,
        })
    );
}

#[test]
fn file_entry_serializes_with_camel_case_content_hash() {
    let entry = FileEntry {
        path: "a.js".into(),
        size: 42,
        content_hash: "abc123".into(),
        kind: crate::artifact::ArtifactFileKind::File,
        symlink_resolved_path: None,
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["contentHash"], "abc123");
    assert_eq!(json["path"], "a.js");
    assert_eq!(json["size"], 42);
    // Server schema (FileEntrySchema) is `.strict()`, so any stray key would
    // make the deployment-create POST fail validation.
    let obj = json.as_object().unwrap();
    assert_eq!(
        obj.len(),
        3,
        "expected exactly path/size/contentHash, got {obj:?}"
    );
}

// ── manifest → compute type mapping tests ────────────────────
//
// Verifies the contract: primary_compute_target(manifest) → LayerTarget,
// which deploy maps as: Compute→Process, Static→Static.

#[test]
fn manifest_compute_layer_maps_to_process() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [
            {"name": "assets", "target": "STATIC", "directory": "static"},
            {"name": "server", "target": "COMPUTE", "directory": "standalone", "entry": "server.js"}
        ],
        "routes": [{"pattern": "^/.*$", "layer": "server"}]
    }"#,
    )
    .unwrap();

    let target = crate::build::manifest::primary_compute_target(&manifest);
    let compute = match target {
        crate::build::manifest::LayerTarget::Compute => ComputeType::Process,
        crate::build::manifest::LayerTarget::Static => ComputeType::Static,
    };
    assert_eq!(compute, ComputeType::Process);
}

#[test]
fn manifest_static_only_maps_to_static() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [{"name": "site", "target": "STATIC", "directory": "."}],
        "routes": [{"pattern": "^/.*$", "layer": "site"}]
    }"#,
    )
    .unwrap();

    let target = crate::build::manifest::primary_compute_target(&manifest);
    let compute = match target {
        crate::build::manifest::LayerTarget::Compute => ComputeType::Process,
        crate::build::manifest::LayerTarget::Static => ComputeType::Static,
    };
    assert_eq!(compute, ComputeType::Static);
}

// ── validate_process_output tests ────────────────────────────

fn make_detection(
    framework: &str,
    ssr: Option<crate::detect::types::SsrAnalysis>,
) -> crate::detect::types::DetectionResult {
    crate::detect::types::DetectionResult {
        framework: framework.to_string(),
        name: framework.to_string(),
        version: None,
        suggested_compute: crate::detect::types::ComputeType::Process,
        reason: String::new(),
        metadata: crate::detect::types::DetectionMetadata {
            uses_typescript: None,
            config_files: vec![],
            runtime: crate::detect::types::RuntimeInfo {
                runtime_type: crate::detect::types::RuntimeType::Node,
                version: None,
            },
            package_manager: None,
            build_info: None,
            monorepo: None,
            ssr_analysis: ssr,

            structure: vec![],
        },
    }
}

#[test]
fn validate_nextjs_dot_next_without_standalone_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("output: 'standalone'"),
        "should mention standalone: {msg}"
    );
}

#[test]
fn validate_nextjs_dot_next_with_standalone_but_missing_server_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next");
    fs::create_dir(&output_dir).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Missing file"),
        "should mention missing file: {msg}"
    );
}

#[test]
fn validate_nextjs_standalone_dir_without_server_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("server.js is missing"),
        "should mention missing server.js: {msg}"
    );
}

#[test]
fn validate_nextjs_standalone_dir_with_server_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(output_dir.join("server.js"), "console.log('ok')").unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_nuxt_without_server_entry_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nuxt", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("nuxi build"),
        "should mention nuxi build: {msg}"
    );
}

#[test]
fn validate_nuxt_with_server_entry_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/index.mjs"), "export default {}").unwrap();

    let detection = make_detection("nuxt", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_unknown_framework_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("vite", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_prebuild_process_project_rejects_cloudflare_vite_plugin() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let err = validate_prebuild_process_project(dir.path())
        .expect_err("package-level Workers target should fail before build");
    let msg = err.to_string();
    assert!(
        msg.contains("Cloudflare Workers target detected")
            && msg.contains("@cloudflare/vite-plugin"),
        "should mention package-level Cloudflare signal: {msg}"
    );
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("prebuild framework rejection must carry a CodedError");
    assert_eq!(coded.code, "FRAMEWORK_UNSUPPORTED");
}

#[test]
fn validate_prebuild_compute_intent_skips_autodetect_process() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let explicit_compute = resolve_explicit_compute_type(None, None).unwrap();

    assert_eq!(explicit_compute, None);
    assert!(validate_prebuild_compute_intent(dir.path(), explicit_compute).is_ok());
}

#[test]
fn resolve_deploy_compute_type_prefers_static_manifest_over_autodetect_process() {
    let manifest = crate::build::manifest::generate_static_manifest();
    let detection = make_detection("tanstack-start", None);

    let compute = resolve_deploy_compute_type(None, Some(&manifest), &detection);

    assert_eq!(compute, ComputeType::Static);
}

#[tokio::test]
async fn postbuild_detection_preserves_generated_root_static_html() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let effective =
        nrz::config::EffectiveProjectConfig::from_project_config(dir.path().to_path_buf(), config);

    let stale_detection =
        crate::detect::detect_with_framework_override(dir.path(), effective.framework_override());
    assert_eq!(stale_detection.framework, "other");

    fs::write(dir.path().join("index.html"), "<h1>generated</h1>").unwrap();

    let stale_result = build::run_with_effective_config(
        BuildArgs {
            dir: dir.path().to_string_lossy().into_owned(),
            skip_validation: true,
        },
        true,
        &effective,
        Some(&stale_detection),
        false,
    )
    .await;
    assert!(
        stale_result.is_err(),
        "stale prebuild detection should miss generated root static HTML"
    );

    let postbuild_detection =
        crate::detect::detect_with_framework_override(dir.path(), effective.framework_override());
    assert_eq!(postbuild_detection.framework, "static-html");

    let result = build::run_with_effective_config(
        BuildArgs {
            dir: dir.path().to_string_lossy().into_owned(),
            skip_validation: true,
        },
        true,
        &effective,
        Some(&postbuild_detection),
        false,
    )
    .await
    .unwrap();

    assert_eq!(result.output_dir, dir.path());
    let manifest = result
        .manifest
        .expect("root static HTML should auto-generate a STATIC manifest");
    assert_eq!(compute_type_from_manifest(&manifest), ComputeType::Static);
    assert_eq!(manifest.layers[0].directory, ".");
}

#[test]
fn validate_cloudflare_vite_plugin_dep_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Cloudflare Workers target detected")
            && msg.contains("@cloudflare/vite-plugin"),
        "should mention CF Workers + plugin: {msg}"
    );
    assert!(
        msg.contains("--compute static") && msg.contains("nitro"),
        "should offer both escape hatches: {msg}"
    );
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("error must carry a CodedError so Builder classifies it as user-fault");
    assert_eq!(coded.code, "FRAMEWORK_UNSUPPORTED");
}

#[test]
fn validate_cloudflare_wrangler_output_bails() {
    // Fallback signal: even without the dep in package.json (e.g. pnpm workspace root),
    // the build emits server/wrangler.json and we must catch it.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/wrangler.json"), r#"{"name":"x"}"#).unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("server/wrangler.json was emitted"),
        "should mention wrangler.json trigger: {msg}"
    );
}

#[test]
fn validate_hydrogen_oxygen_bails_via_mini_oxygen_dep() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@shopify/mini-oxygen":"^3.0.0","@shopify/hydrogen":"^2026.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("hydrogen", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Shopify Oxygen") && msg.contains("@shopify/mini-oxygen"),
        "should mention Oxygen + mini-oxygen: {msg}"
    );
    assert!(
        msg.contains("Express recipe"),
        "should recommend Express recipe: {msg}"
    );
}

#[test]
fn validate_hydrogen_oxygen_bails_via_output_marker() {
    // Fallback: even without the dep signal we catch Oxygen by its marker file.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/oxygen.json"), r#"{"version":1}"#).unwrap();

    let detection = make_detection("hydrogen", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("server/oxygen.json was emitted"),
        "should mention oxygen.json trigger: {msg}"
    );
}

#[test]
fn validate_hydrogen_express_recipe_ok() {
    // Express recipe emits build/server/index.js plus server.mjs at project root.
    // No Oxygen signals → validate passes, entry resolution happens downstream.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("build");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/index.js"), "x".repeat(600)).unwrap();
    fs::write(dir.path().join("server.mjs"), "x".repeat(600)).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"start":"node server.mjs"},"dependencies":{"express":"^4.19.0","@shopify/hydrogen":"^2026.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("hydrogen", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok(), "Express recipe should pass: {result:?}");
}

#[test]
fn validate_malformed_package_json_bails_with_parse_error() {
    // Regression: a corrupted package.json used to silently yield "no workers
    // signal" and ship a broken PROCESS deploy. validate_process_output must
    // surface the parse error loudly so the user can fix the manifest.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(dir.path().join("package.json"), "not json{").unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("failed to parse") && msg.contains("package.json"),
        "should report parse error: {msg}"
    );
}

#[test]
fn validate_tanstack_start_nitro_output_ok() {
    // TanStack Start with Nitro node-server preset: .output/server/index.mjs layout
    // should pass validation (no CF workers signals present).
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/index.mjs"), "export default {}").unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@tanstack/react-start":"^1.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("tanstack-start", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok(), "validation should pass: {result:?}");
}

// ── ensure_process_entry tests ───────────────────────────────

#[test]
fn ensure_process_entry_resolves_module_field() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"app","module":"./server.mjs"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("server.mjs"), "export default {}").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert_eq!(entry, Some("server.mjs".to_string()));
    assert!(warning.is_none());
}

#[test]
fn ensure_process_entry_ambiguous_candidates_errors_for_non_strict_framework() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("runtime")).unwrap();
    // Both files must exceed the heuristic-scan min-size threshold, otherwise
    // they're classified as ESM stubs and skipped before ambiguity kicks in.
    fs::write(
        dir.path().join("runtime/foo.mjs"),
        format!("console.log('foo') // {}", "x".repeat(600)),
    )
    .unwrap();
    fs::write(
        dir.path().join("runtime/bar.mjs"),
        format!("console.log('bar') // {}", "x".repeat(600)),
    )
    .unwrap();

    let detection = make_detection("other", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(msg.contains("ambiguous"));
    assert!(msg.contains("[deploy] entry"));
    expect_code(&err, "ENTRY_POINT_AMBIGUOUS");
}

#[test]
fn ensure_process_entry_root_prefers_server_over_main() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('server')").unwrap();
    fs::write(dir.path().join("main.js"), "console.log('main')").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert_eq!(entry, Some("server.js".to_string()));
    assert!(warning.is_none());
}

#[test]
fn ensure_process_entry_config_entry_allows_double_dot_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("foo..js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), Some("foo..js"), &detection, true).unwrap();
    assert_eq!(entry, Some("foo..js".to_string()));
    assert!(warning.is_none());
    assert!(!dir.path().join("package.json").exists());
}

#[test]
fn ensure_process_entry_config_entry_rejects_parent_traversal() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    let err = ensure_process_entry(
        dir.path(),
        dir.path(),
        Some("../server.js"),
        &detection,
        true,
    )
    .expect_err("parent traversal should fail");
    assert!(
        err.to_string()
            .contains("relative path within the output directory")
    );
}

#[test]
fn ensure_process_entry_config_entry_rejects_shell_command() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    let err = ensure_process_entry(
        dir.path(),
        dir.path(),
        Some("node index.js"),
        &detection,
        true,
    )
    .expect_err("shell command entry should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("not a shell command"),
        "unexpected error: {msg}"
    );
    expect_code(&err, "INVALID_DEPLOY_ENTRY");
}

#[test]
fn ensure_process_entry_not_found_errors_for_non_strict_framework() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.css"), "body{}").unwrap();

    let detection = make_detection("other", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(msg.contains("Cannot determine entry point"));
    assert!(msg.contains("[deploy] entry"));
    assert!(!msg.contains("Falling back to runtime default"));
}

#[test]
fn ensure_process_entry_not_found_is_error_for_strict_framework() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    assert!(
        err.to_string().contains("Nuxt PROCESS deployment expects"),
        "unexpected error: {err:#}"
    );
}

#[test]
fn ensure_process_entry_not_found_is_error_for_hydrogen() {
    // Regression: hydrogen lost its FrameworkHint in this PR; without strict
    // handling, a hydrogen project with no buildable entry would silently fall
    // back to `bun <output>` and 404. Must bail with Hydrogen diagnostic.
    let dir = tempdir().unwrap();
    let detection = make_detection("hydrogen", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(
        msg.contains("Hydrogen PROCESS") && msg.contains("Express recipe"),
        "expected Hydrogen diagnostic, got: {msg}"
    );
}

#[test]
fn ensure_process_entry_not_found_is_error_for_tanstack_start() {
    let dir = tempdir().unwrap();
    let detection = make_detection("tanstack-start", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    let msg = err.to_string();
    assert!(
        msg.contains("TanStack Start PROCESS") && msg.contains("server/index.mjs"),
        "expected TSS diagnostic, got: {msg}"
    );
}

#[test]
fn is_strict_process_framework_covers_all_ssr() {
    // All SSR frameworks must be strict — falling back to `bun <output>` for
    // a framework we claim to support is the exact silent-404 failure mode
    // this PR was created to eliminate.
    for framework in [
        "nextjs",
        "nuxt",
        "sveltekit",
        "astro",
        "remix",
        "react-router",
        "solidstart",
        "qwik",
        "analog",
        "blitzjs",
        "payload",
        "tanstack-start",
        "hydrogen",
    ] {
        assert!(
            is_strict_process_framework(framework),
            "{framework} must be strict"
        );
    }
    // Unknown / non-SSR frameworks stay non-strict (generic server projects
    // may legitimately want the bun fallback).
    assert!(!is_strict_process_framework("other"));
    assert!(!is_strict_process_framework("vite"));
    assert!(!is_strict_process_framework("hono"));
}

// ── COMPUTE auto-gen bail: entry not found ────────────────────

#[test]
fn ensure_process_entry_not_found_is_the_bail_precondition() {
    // Verify that a project with no runnable files fails before COMPUTE manifest
    // auto-generation instead of advertising a runtime default that deploy cannot
    // actually encode.
    let dir = tempdir().unwrap();
    // Create a "dist" output dir with no .js/.mjs/.cjs files
    fs::create_dir(dir.path().join("dist")).unwrap();
    fs::write(dir.path().join("dist/style.css"), "body{}").unwrap();

    let detection = make_detection("other", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    assert!(
        err.to_string().contains("Cannot determine entry point"),
        "unexpected error: {err:#}"
    );
}

// ── framework_process_diagnostic tests ───────────────────────

#[test]
fn diagnostic_nextjs_no_standalone_suggests_config() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nextjs", None);
    let msg = framework_process_diagnostic("nextjs", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("output: 'standalone'"));
}

#[test]
fn diagnostic_nextjs_standalone_mentions_server_js() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let msg = framework_process_diagnostic("nextjs", &detection, &output_dir);
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server.js"));
}

#[test]
fn diagnostic_nuxt_mentions_nuxi_build() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let msg = framework_process_diagnostic("nuxt", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("nuxi build"));
}

#[test]
fn diagnostic_sveltekit_mentions_adapter_node() {
    let dir = tempdir().unwrap();
    let detection = make_detection("sveltekit", None);
    let msg = framework_process_diagnostic("sveltekit", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("adapter-node"));
}

#[test]
fn diagnostic_unknown_framework_returns_none() {
    let dir = tempdir().unwrap();
    let detection = make_detection("vite", None);
    let msg = framework_process_diagnostic("vite", &detection, dir.path());
    assert!(msg.is_none());
}

#[test]
fn diagnostic_react_router_mentions_server_index() {
    let dir = tempdir().unwrap();
    let detection = make_detection("react-router", None);
    let msg = framework_process_diagnostic("react-router", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server/index.js"));
}

#[test]
fn diagnostic_remix_mentions_server_index() {
    let dir = tempdir().unwrap();
    let detection = make_detection("remix", None);
    let msg = framework_process_diagnostic("remix", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server/index.js"));
}

#[test]
fn diagnostic_hono_mentions_entry_point() {
    let dir = tempdir().unwrap();
    let detection = make_detection("hono", None);
    let msg = framework_process_diagnostic("hono", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("dist/"));
}

#[test]
fn diagnostic_elysia_mentions_bun() {
    let dir = tempdir().unwrap();
    let detection = make_detection("elysia", None);
    let msg = framework_process_diagnostic("elysia", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("Bun"));
}

// ── resolve_health_check ─────────────────────────────────────

#[test]
fn health_check_flag_wins_over_config_and_autodetect() {
    let dir = tempdir().unwrap();
    // Create a detectable endpoint that should be ignored
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Http(
        "/from-config".to_string(),
    ));

    let detection = make_detection("nextjs", None);
    let result = resolve_health_check(
        Some("/from-flag"),
        &config,
        dir.path(),
        &detection,
        dir.path(),
        true, // json mode suppresses output
    )
    .unwrap();

    assert_eq!(result.path, Some("/from-flag".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Flag));
}

#[test]
fn health_check_config_wins_over_autodetect() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Http(
        "/from-config".to_string(),
    ));

    let detection = make_detection("nextjs", None);
    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert_eq!(result.path, Some("/from-config".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Config));
}

#[test]
fn health_check_autodetect_when_no_flag_or_config() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("nextjs", None);

    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert_eq!(result.path, Some("/api/health".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Detected));
}

#[test]
fn health_check_default_tcp_when_nothing_found() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("other", None);

    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert!(result.path.is_none());
    assert!(matches!(result.source, HealthCheckSource::Default));
}

#[test]
fn health_check_flag_none_gives_tcp() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("other", None);

    for alias in &["none", "NONE", "false", "tcp", "TCP", "None"] {
        let result = resolve_health_check(
            Some(alias),
            &config,
            dir.path(),
            &detection,
            dir.path(),
            true,
        )
        .unwrap();

        assert!(
            result.path.is_none(),
            "expected TCP for alias \"{alias}\", got: {:?}",
            result.path
        );
        assert!(matches!(result.source, HealthCheckSource::Flag));
    }
}

#[test]
fn health_check_config_tcp_overrides_autodetect() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Tcp);

    let detection = make_detection("nextjs", None);
    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert!(result.path.is_none());
    assert!(matches!(result.source, HealthCheckSource::Config));
}

// ── validate_health_path ─────────────────────────────────────

#[test]
fn validate_health_path_rejects_no_slash() {
    let result = validate_health_path("health", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must start with '/'")
    );
}

#[test]
fn validate_health_path_rejects_parent_traversal() {
    let result = validate_health_path("/../../etc", "--health-check-path");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("'..'"));
}

#[test]
fn validate_health_path_rejects_query() {
    let result = validate_health_path("/health?v=1", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("query or fragment")
    );
}

#[test]
fn validate_health_path_rejects_fragment() {
    let result = validate_health_path("/health#section", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("query or fragment")
    );
}

#[test]
fn validate_health_path_accepts_valid_path() {
    assert!(validate_health_path("/health", "--health-check-path").is_ok());
    assert!(validate_health_path("/api/health", "--health-check-path").is_ok());
    assert!(validate_health_path("/v1/healthz", "--health-check-path").is_ok());
}

// ── ComputeConfigBody serialization ─────────────────────────

#[test]
fn compute_config_body_with_health_check_path_serializes_camel_case() {
    let body = ComputeConfigBody {
        health_check_path: Some("/api/health".to_string()),
    };
    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(
        value.get("healthCheckPath").and_then(|v| v.as_str()),
        Some("/api/health")
    );
    assert!(value.get("health_check_path").is_none());
}

#[test]
fn compute_config_body_without_path_omits_field() {
    let body = ComputeConfigBody {
        health_check_path: None,
    };
    let value = serde_json::to_value(&body).unwrap();
    assert!(value.get("healthCheckPath").is_none());
}

#[test]
fn deploy_output_serializes_public_json_as_camel_case() {
    let output = DeployOutput {
        deployment_id: "dep_123".to_string(),
        url: "https://example.test".to_string(),
        status: "live".to_string(),
        target: deploy_target_output(Some(false)),
        preview_protected: true,
        warnings: vec![],
        health_check: Some(HealthCheckInfo::Http {
            path: "/health".to_string(),
            source: HealthCheckSourceTag::Config,
        }),
        verification: Some(verify::DeployVerificationOutput {
            status: "passed",
            url: "https://example.test/health".to_string(),
            path: "/health".to_string(),
            status_code: 200,
            used_preview_bypass: true,
            preview_access_revoked: Some(true),
        }),
    };

    let value = serde_json::to_value(&output).unwrap();

    assert_eq!(value["deploymentId"], "dep_123");
    assert_eq!(value["target"]["environment"], "preview");
    assert_eq!(value["previewProtected"], true);
    assert_eq!(value["healthCheck"]["path"], "/health");
    assert_eq!(value["verification"]["usedPreviewBypass"], true);
    assert_eq!(value["verification"]["previewAccessRevoked"], true);
    assert!(value.get("deployment_id").is_none());
    assert!(value.get("health_check").is_none());
}

// ── is_nextjs_project ───────────────────────────────────────

#[test]
fn is_nextjs_detects_next_in_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"next":"^16.2.0","react":"^19.0.0"}}"#,
    )
    .unwrap();
    assert!(is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_detects_next_in_dev_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"next":"16.2.0"}}"#,
    )
    .unwrap();
    assert!(is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_false_for_non_next_project() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^19.0.0","vite":"^6.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_false_without_package_json() {
    let dir = tempdir().unwrap();
    assert!(!is_nextjs_project(dir.path()));
}

#[test]
fn nextjs_prebuild_clears_stale_adapter_descriptor() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"next":"15.5.0"}}"#,
    )
    .unwrap();
    let descriptor = dir.path().join(".onreza/next-adapter-output.json");
    fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
    fs::write(&descriptor, "{}").unwrap();

    clear_nextjs_descriptor_before_build(dir.path()).unwrap();

    assert!(!descriptor.exists());
}

// ── is_sveltekit_with_adapter_auto ───────────────────────────

#[test]
fn sveltekit_adapter_auto_false_for_non_sveltekit() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^19.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_node_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-node":"^5.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-node';\nexport default { kit: { adapter: adapter() } };",
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_true_with_adapter_auto_config() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-auto":"^3.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-auto';\nexport default { kit: { adapter: adapter() } };",
    )
    .unwrap();
    assert!(is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_true_when_no_config() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}}"#,
    )
    .unwrap();
    assert!(is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_vercel_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-vercel":"^6.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_cloudflare_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-cloudflare":"^7.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_netlify_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-netlify":"^6.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn diagnostic_payload_mentions_standalone() {
    let dir = tempdir().unwrap();
    let detection = make_detection("payload", None);
    let msg = framework_process_diagnostic("payload", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("standalone"));
}

#[cfg(unix)]
#[test]
fn run_command_streaming_emits_coded_error_on_nonzero_exit() {
    let dir = tempdir().unwrap();
    let err = run_command_streaming(
        "exit 2",
        dir.path(),
        false,
        crate::output::Phase::Build,
        "debug",
        &[],
    )
    .expect_err("non-zero exit must fail");

    let msg = err.to_string();
    assert!(
        msg.contains("exit code 2"),
        "human message should keep the exit code detail: {msg}"
    );
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("non-zero exit must carry a CodedError so Builder classifies it as user-fault");
    assert_eq!(coded.code, "BUILD_EXIT_CODE");
}

#[cfg(unix)]
#[test]
fn run_command_streaming_emits_phase_specific_code_for_install() {
    let dir = tempdir().unwrap();
    let err = run_command_streaming(
        "exit 1",
        dir.path(),
        true, // JSON mode: exercises the second bail! site
        crate::output::Phase::Install,
        "user",
        &[],
    )
    .expect_err("non-zero exit must fail");
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("CodedError expected in JSON-mode path as well");
    assert_eq!(coded.code, "INSTALL_EXIT_CODE");
}

#[cfg(unix)]
#[test]
fn run_command_streaming_does_not_hang_on_orphaned_grandchild() {
    // The build command exits immediately but leaves a backgrounded grandchild
    // holding the stdout pipe write-end open. Before the process-group reap, the
    // stdout reader join blocked until the grandchild died — turning a
    // successful build into a 15-minute "build timeout". The call must now
    // return promptly because we SIGKILL the whole group after `wait()`.
    use std::sync::mpsc;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let dir = tempdir().unwrap();
        let result = run_command_streaming(
            "sleep 30 & echo started",
            dir.path(),
            true, // JSON mode: exercises the piped stdout/stderr reader-join path
            crate::output::Phase::Build,
            "debug",
            &[],
        );
        let _ = tx.send(result.is_ok());
    });

    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(ok) => assert!(ok, "command itself should succeed"),
        Err(_) => panic!(
            "run_command_streaming hung waiting for EOF on a pipe held open by an orphaned grandchild"
        ),
    }
    worker.join().unwrap();
}

// ── Error-code contract (user-fault failures carry CodedError) ───────

fn expect_code(err: &anyhow::Error, expected: &str) {
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .unwrap_or_else(|| panic!("expected CodedError({expected}) in chain: {err:#}"));
    assert_eq!(
        coded.code, expected,
        "wrong code in chain for error: {err:#}"
    );
}

#[test]
fn function_stage_error_preserves_publish_failure_details() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "FUNCTION_PUBLISH_FAILED".to_string(),
        message: "Ошибка валидации".to_string(),
        retry_after_seconds: None,
        details: Some(serde_json::json!({
            "attemptId": "11111111-1111-1111-1111-111111111111",
            "category": "POLICY",
            "message": "function 'api' failed the function policy check",
        })),
    }
    .into();

    let mapped = map_function_stage_error(error, false);
    let rendered = format!("{mapped:#}");

    assert!(rendered.contains("failed to stage ONREZA Functions for deployment"));
    assert!(rendered.contains("ONREZA Functions publish failed [POLICY]"));
    assert!(rendered.contains("11111111-1111-1111-1111-111111111111"));
}

#[test]
fn create_deployment_error_maps_edge_rules_divergence() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "EDGE_RULES_DIVERGED".to_string(),
        message:
            "environment has UI-authored edge rules; run `nrz rules pull` to import them, or redeploy with --force-rules"
                .to_string(),
        retry_after_seconds: None,
        details: None,
    }
    .into();

    let mapped = map_create_deployment_error(error, false);
    let rendered = format!("{mapped:#}");
    let coded = mapped
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .expect("mapped error must keep EDGE_RULES_DIVERGED code");

    assert_eq!(coded.code, "EDGE_RULES_DIVERGED");
    assert!(rendered.contains("nrz rules pull"));
    assert!(rendered.contains("--force-rules"));
}

#[test]
fn create_deployment_validation_error_in_json_mode_preserves_details() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "VALIDATION_ERROR".to_string(),
        message: "Ошибка валидации данных".to_string(),
        retry_after_seconds: None,
        details: Some(serde_json::json!({
            "fields": [{ "field": "manifest.meta", "message": "Некорректное значение" }]
        })),
    }
    .into();

    let mapped = map_create_deployment_error(error, true);

    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_some(),
        "validation error in JSON mode must be fully reported with details, got: {mapped:#}"
    );
}

#[test]
fn function_stage_validation_error_in_json_mode_preserves_details() {
    let error: anyhow::Error = crate::api::StructuredApiError {
        status: StatusCode::BAD_REQUEST,
        code: "VALIDATION_ERROR".to_string(),
        message: "Ошибка валидации данных".to_string(),
        retry_after_seconds: None,
        details: Some(serde_json::json!({
            "fields": [{ "field": "generatedEdgeRuleSets", "message": "Некорректное значение" }]
        })),
    }
    .into();

    let mapped = map_function_stage_error(error, true);

    assert!(
        mapped
            .downcast_ref::<crate::output::AlreadyReportedError>()
            .is_some(),
        "stage validation error in JSON mode must be fully reported with details, got: {mapped:#}"
    );
}

#[test]
fn boundary_wrap_nuxt_missing_server_is_missing_process_entry() {
    // validate_process_output is an internal helper; the boundary wrap that
    // tags its failures with MISSING_PROCESS_ENTRY lives at the call site in
    // deploy::run (`with_default_code(..., "MISSING_PROCESS_ENTRY")`). We
    // simulate that wrap here so the full user-visible classification path is
    // exercised end-to-end.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nuxt", None);
    let raw = validate_process_output(&output_dir, dir.path(), &detection)
        .expect_err("nuxt without server/index.mjs must fail");
    let wrapped = crate::output::with_default_code(raw, "MISSING_PROCESS_ENTRY");
    expect_code(&wrapped, "MISSING_PROCESS_ENTRY");
}

#[test]
fn boundary_wrap_preserves_more_specific_framework_unsupported() {
    // CF Workers detection is tagged FRAMEWORK_UNSUPPORTED deeper in the stack;
    // the outer boundary wrap (MISSING_PROCESS_ENTRY) must NOT clobber it.
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@cloudflare/vite-plugin":"^1.0.0"}}"#,
    )
    .unwrap();

    let detection = make_detection("tanstack-start", None);
    let raw = validate_process_output(&output_dir, dir.path(), &detection)
        .expect_err("CF workers detection must fail");
    let wrapped = crate::output::with_default_code(raw, "MISSING_PROCESS_ENTRY");
    expect_code(&wrapped, "FRAMEWORK_UNSUPPORTED");
}

#[test]
fn ensure_process_entry_missing_user_entry_is_invalid_deploy_entry() {
    // User set [deploy] entry in onreza.toml but the file isn't in the build
    // output — the point-coded INVALID_DEPLOY_ENTRY must win over the outer
    // MISSING_PROCESS_ENTRY wrap, so users see the specific diagnosis.
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let err = ensure_process_entry(dir.path(), dir.path(), Some("server.mjs"), &detection, true)
        .expect_err("user entry missing on disk must fail");
    let wrapped = crate::output::with_default_code(err, "MISSING_PROCESS_ENTRY");
    expect_code(&wrapped, "INVALID_DEPLOY_ENTRY");
}

#[test]
fn ensure_process_entry_missing_user_entry_suggests_nested_output_dir() {
    // Regression from production: selected output root was /workspace, but the
    // build emitted onreza-output/server.cjs. The user needs an outputDirectory
    // fix, not a generic "server.cjs missing" message.
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("onreza-output")).unwrap();
    fs::write(
        dir.path().join("onreza-output/server.cjs"),
        "console.log('ok')",
    )
    .unwrap();

    let detection = make_detection("express", None);
    let err = ensure_process_entry(dir.path(), dir.path(), Some("server.cjs"), &detection, true)
        .expect_err("entry is outside selected output root");
    let msg = err.to_string();

    assert!(
        msg.contains("onreza-output/server.cjs"),
        "should mention discovered nested entry: {msg}"
    );
    assert!(
        msg.contains("[build] output_directory = \"onreza-output\""),
        "should suggest the outputDirectory fix: {msg}"
    );
    expect_code(&err, "INVALID_DEPLOY_ENTRY");
}

#[test]
fn format_deployment_failure_includes_runtime_startup_details() {
    let status = DeploymentStatusResponse {
        id: "dep-1".to_string(),
        status: "failed".to_string(),
        url: None,
        production: None,
        error: Some("Pre-warm failed".to_string()),
        error_code: Some("DEPLOY_PREWARM_PORT_MISMATCH".to_string()),
        error_details: Some(DeploymentErrorDetails {
            runtime_startup_failure: Some(RuntimeStartupFailureDetails {
                code: Some("port_mismatch".to_string()),
                message: Some("Your app is listening on port 3000.".to_string()),
                check_type: Some("tcp".to_string()),
                health_path: None,
                expected_port: Some(30123),
                detected_ports: vec![3000],
                timeout_seconds: Some(30),
                attempts: Some(2700),
                last_error: Some("Connection refused".to_string()),
                process_entry: Some("server.js".to_string()),
                log_tail: Some("server started on 3000".to_string()),
                retry_after_seconds: None,
            }),
        }),
        created_at: None,
        ready_at: None,
    };

    let msg = format_deployment_failure("Pre-warm failed", &status);

    assert!(msg.contains("Your app is listening on port 3000."));
    assert!(msg.contains("expected port: 30123"));
    assert!(msg.contains("detected ports: 3000"));
    assert!(msg.contains("Recent runtime output"));
}

#[test]
fn parse_compute_type_rejects_unknown_value_with_code() {
    let err = parse_compute_type("lambda").expect_err("unknown compute must fail");
    expect_code(&err, "INVALID_COMPUTE_TYPE");
}

#[test]
fn validate_health_path_rejects_query_string_with_code() {
    let err =
        validate_health_path("/health?x=1", "--health-check-path").expect_err("query must fail");
    expect_code(&err, "INVALID_ARGUMENT");
}

#[test]
fn platform_fault_errors_do_not_carry_coded_error() {
    // Negative coverage: uncoded `anyhow!` / `?` on I/O errors must leave the
    // chain free of CodedError, so the builder routes them to Sentry. If this
    // test ever goes green with a CodedError present, the contract "empty code
    // = platform-fault" has been silently eroded.
    let err = anyhow::anyhow!("simulated platform-fault");
    assert!(
        err.chain()
            .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
            .is_none(),
        "plain anyhow errors must not carry CodedError — got: {err:#}"
    );
}

#[test]
fn with_default_code_attaches_code_and_preserves_source_chain() {
    // A semantic (non-I/O) error walked through .context(..) and then
    // with_default_code must gain a CodedError AND keep the earlier context
    // reachable through the chain for downstream tooling.
    let err = anyhow::anyhow!("field \"entry\" missing").context("validating manifest");
    let wrapped = crate::output::with_default_code(err, "INVALID_MANIFEST");
    expect_code(&wrapped, "INVALID_MANIFEST");
    let rendered = format!("{wrapped:#}");
    assert!(
        rendered.contains("validating manifest") && rendered.contains("field \"entry\" missing"),
        "source chain must survive through CodedError wrapping: {rendered}"
    );
}

#[test]
fn with_default_code_skips_io_errors_so_platform_faults_reach_sentry() {
    // Guard against accidentally classifying a platform-fault I/O failure
    // (permission denied, TOCTOU, EIO) as user-fault just because the outer
    // boundary wrap fires on every error. io::Error anywhere in the chain must
    // keep the error uncoded so the builder routes it to Sentry.
    use std::io;
    let io_err: anyhow::Error =
        anyhow::Error::new(io::Error::other("perm")).context("canonicalizing entry");
    let result = crate::output::with_default_code(io_err, "MISSING_PROCESS_ENTRY");
    assert!(
        result
            .chain()
            .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
            .is_none(),
        "io::Error paths must stay uncoded: {result:#}"
    );
}

#[test]
fn wire_manifest_contract_rejects_unknown_layer_fields() {
    let conformant = serde_json::json!({
        "version": 1,
        "layers": [{ "name": "static", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*", "layer": "static" }],
    });
    let wire = conform_manifest_to_wire_contract(conformant)
        .expect("a conformant manifest must pass the wire contract");
    assert!(wire["version"].is_number());
    assert_eq!(wire["layers"][0]["target"], "STATIC");

    // A field the platform ManifestSchema does not define (e.g. the legacy `export`)
    // must be rejected at the wire instead of forwarded for the server to reject.
    let with_unknown = serde_json::json!({
        "version": 1,
        "layers": [{ "name": "static", "target": "STATIC", "directory": ".", "export": "esm" }],
        "routes": [{ "pattern": "^/.*", "layer": "static" }],
    });
    assert!(
        conform_manifest_to_wire_contract(with_unknown).is_err(),
        "unknown manifest fields must not reach the server"
    );
}

#[test]
fn wire_functions_contract_validates_origin_against_server_enum() {
    let ok = crate::functions::FunctionPublishPayload {
        origin: "DEPLOYMENT",
        functions: vec![],
        edge_rules: None,
        edge_rules_force: false,
        generated_edge_rule_sets: Vec::new(),
    };
    assert!(
        conform_functions_to_wire_contract(Some(ok))
            .unwrap()
            .is_some()
    );
    assert!(conform_functions_to_wire_contract(None).unwrap().is_none());

    // An origin value the server contract does not define must be rejected at the wire.
    let bad = crate::functions::FunctionPublishPayload {
        origin: "BOGUS",
        functions: vec![],
        edge_rules: None,
        edge_rules_force: false,
        generated_edge_rule_sets: Vec::new(),
    };
    assert!(
        conform_functions_to_wire_contract(Some(bad)).is_err(),
        "an unknown functions origin must not reach the server"
    );
}

#[test]
fn wire_functions_contract_accepts_generated_edge_rule_contributions() {
    let payload = crate::functions::FunctionPublishPayload {
        origin: "DEPLOYMENT",
        functions: vec![],
        edge_rules: None,
        edge_rules_force: false,
        generated_edge_rule_sets: vec![crate::functions::GeneratedEdgeRuleSet {
            producer: "nextjs-adapter".to_string(),
            version: Some("16.2.9".to_string()),
            edge_rules: serde_json::json!({
                "schemaVersion": "EDGE_RULE_SET_V1",
                "rules": [],
            }),
        }],
    };

    let value = conform_functions_to_wire_contract(Some(payload))
        .unwrap()
        .expect("functions payload");

    assert_eq!(
        value["generatedEdgeRuleSets"][0]["producer"],
        "nextjs-adapter"
    );
    assert_eq!(
        value["generatedEdgeRuleSets"][0]["edgeRules"]["rules"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
