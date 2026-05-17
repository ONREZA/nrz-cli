//! CLI integration tests
//!
//! Note: binary is run through a pipe, so stdout.is_terminal() = false
//! and JSON mode activates automatically. All assertions check JSON in stdout.

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

/// Get the binary command
fn nrz() -> Command {
    Command::cargo_bin("nrz").unwrap()
}

#[test]
fn help_returns_exit_0() {
    let mut cmd = nrz();
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn kv_set_and_get_in_tempdir() {
    let temp = tempfile::tempdir().unwrap();

    // Set a key (JSON: {"status":"ok"})
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "mykey", "myvalue"]);
    cmd.assert().success().stdout(contains("\"status\""));

    // Get the key (JSON: {"key":"mykey","value":"myvalue"})
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "mykey"]);
    cmd.assert().success().stdout(contains("myvalue"));
}

#[test]
fn kv_get_nonexistent_key() {
    let temp = tempfile::tempdir().unwrap();

    // JSON: {"key":"nonexistent","value":null}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "nonexistent"]);
    cmd.assert().success().stdout(contains("null"));
}

#[test]
fn kv_set_with_ttl() {
    let temp = tempfile::tempdir().unwrap();

    // Set with TTL (JSON: {"status":"ok"})
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "tempkey", "tempvalue", "--ttl", "3600"]);
    cmd.assert().success().stdout(contains("\"status\""));

    // Verify it's set
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "tempkey"]);
    cmd.assert().success().stdout(contains("tempvalue"));
}

#[test]
fn kv_list_keys() {
    let temp = tempfile::tempdir().unwrap();

    // Set multiple keys
    for i in 1..=3 {
        let mut cmd = nrz();
        cmd.current_dir(&temp)
            .args(["kv", "set", &format!("key{i}"), &format!("value{i}")]);
        cmd.assert().success();
    }

    // List keys (JSON: {"keys":[...]})
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "list"]);
    cmd.assert().success().stdout(contains("key1"));
}

#[test]
fn kv_delete_key() {
    let temp = tempfile::tempdir().unwrap();

    // Set and then delete
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "delkey", "delvalue"]);
    cmd.assert().success();

    // JSON: {"status":"ok"}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "delete", "delkey"]);
    cmd.assert().success().stdout(contains("\"status\""));

    // Verify it's gone (JSON: {"key":"delkey","value":null})
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "delkey"]);
    cmd.assert().success().stdout(contains("null"));
}

#[test]
fn dev_without_command_fails() {
    let temp = tempfile::tempdir().unwrap();

    // No onreza.toml, no --command flag → error
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["dev"]);
    cmd.assert()
        .failure()
        .stdout(contains("no dev command specified"));
}

#[test]
fn dev_with_custom_command_works_without_detection() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(temp.path().join("package.json"), r#"{"name":"test"}"#).unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["dev", "--command", "echo test"]);

    let output = cmd.output().unwrap();

    // Should not contain "could not detect framework" error
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("could not detect framework"),
        "Should not fail on framework detection when --command is provided"
    );
}

// ── deploy --app error paths ────────────────────────────────

#[test]
fn deploy_app_in_non_monorepo_fails() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"name": "simple-app", "dependencies": {"next": "14.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["deploy", "--app", "web"]);
    cmd.assert()
        .failure()
        .stdout(contains("no monorepo detected"));
}

#[test]
fn deploy_app_not_found_lists_available() {
    let temp = tempfile::tempdir().unwrap();
    // Create a monorepo with npm workspaces
    fs::write(
        temp.path().join("package.json"),
        r#"{"name": "root", "workspaces": ["apps/*"]}"#,
    )
    .unwrap();
    let apps_web = temp.path().join("apps").join("web");
    fs::create_dir_all(&apps_web).unwrap();
    fs::write(apps_web.join("package.json"), r#"{"name": "@my/web"}"#).unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["deploy", "--app", "nonexistent"]);
    cmd.assert()
        .failure()
        .stdout(contains("not found"))
        .stdout(contains("@my/web"));
}

// ── nrz detect ──────────────────────────────────────────────

#[test]
fn detect_nextjs_project() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"nextjs\""))
        .stdout(contains("\"name\":\"Next.js\""));
}

#[test]
fn detect_astro_project() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"astro": "4.0.0"}}"#,
    )
    .unwrap();
    fs::write(temp.path().join("pnpm-lock.yaml"), "").unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"astro\""))
        .stdout(contains("\"pnpm\""));
}

#[test]
fn detect_static_html_site() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("index.html"),
        "<html><body>hello</body></html>",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"static-html\""));
}

#[test]
fn detect_unknown_project() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"other\""));
}

#[test]
fn detect_slug_only() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"nuxt": "3.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["detect", "--slug-only", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"nuxt\""));
}

#[test]
fn detect_with_package_manager_field() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"packageManager": "bun@1.0.0", "dependencies": {"vite": "5.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"vite\""))
        .stdout(contains("\"bun\""));
}

#[test]
fn detect_suggested_compute_static_for_vite() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"vite": "5.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"suggestedCompute\":\"STATIC\""));
}

#[test]
fn detect_suggested_compute_process_for_nextjs() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_suggested_compute_process_for_remix() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"@remix-run/react": "2.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"remix\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_remix_spa_mode_is_static() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"@remix-run/react": "2.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("vite.config.ts"),
        r#"import { vitePlugin as remix } from "@remix-run/dev";
export default defineConfig({ plugins: [remix({ ssr: false })] })"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"remix\""))
        .stdout(contains("\"suggestedCompute\":\"STATIC\""));
}

#[test]
fn detect_react_router_v7() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"devDependencies": {"@react-router/dev": "7.0.0"}, "dependencies": {"react-router": "7.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"react-router\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_hono_is_process() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"hono": "4.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"hono\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_elysia_is_process() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"elysia": "1.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"elysia\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_save_writes_framework_to_toml() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"astro": "4.0.0"}}"#,
    )
    .unwrap();
    // Create onreza.toml (--save requires it)
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--save", "--json"]);
    cmd.assert().success();

    let content = fs::read_to_string(temp.path().join("onreza.toml")).unwrap();
    assert!(
        content.contains("framework = \"astro\""),
        "onreza.toml should contain framework: {content}"
    );
}

#[test]
fn detect_save_without_onreza_toml_fails_honestly() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"vite": "5.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--save", "--json"]);
    cmd.assert()
        .failure()
        .stdout(contains("cannot save detected framework"))
        .stdout(contains("onreza.toml not found"));

    assert!(!temp.path().join("onreza.toml").exists());
}

#[test]
fn build_uses_onreza_toml_from_dir_argument() {
    let temp = tempfile::tempdir().unwrap();
    let app = temp.path().join("app");
    fs::create_dir_all(app.join("dist")).unwrap();
    fs::write(app.join("onreza.toml"), "[project]\nframework = \"vite\"\n").unwrap();
    fs::write(
        app.join("package.json"),
        r#"{
          "scripts": {"build": "vite build"},
          "dependencies": {"express": "^4.19.0", "react": "^18.3.0"},
          "devDependencies": {"vite": "^5.0.0", "@vitejs/plugin-react": "^4.0.0"}
        }"#,
    )
    .unwrap();
    fs::write(app.join("vite.config.js"), "x".repeat(600)).unwrap();
    fs::write(app.join("dist/index.html"), "<div id=\"root\"></div>").unwrap();
    fs::write(app.join("dist/app.js"), "console.log('app')").unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["build", "app", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("Auto-generated STATIC manifest"))
        .stdout(contains("\\\"framework\\\":\\\"vite\\\""))
        .stdout(contains("\\\"target\\\":\\\"STATIC\\\""));
}

#[test]
fn detect_nonexistent_directory_returns_error() {
    let mut cmd = nrz();
    cmd.args(["detect", "--json", "/tmp/nrz_test_nonexistent_dir_12345"]);
    cmd.assert().failure();
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("error"),
        "should return JSON error for nonexistent dir: {stdout}"
    );
}

#[test]
fn broken_onreza_toml_emits_invalid_config_code() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[deploy]\nentry = \"/abs/path\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert().failure();
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"code\":\"INVALID_CONFIG\""),
        "config load fault must surface as structured error with code=INVALID_CONFIG, got: {stdout}"
    );
}
