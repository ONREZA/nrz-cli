//! CLI integration tests
//!
//! Note: binary is run through a pipe, so stdout.is_terminal() = false
//! and JSON mode activates automatically. All assertions check JSON in stdout.

use assert_cmd::Command;
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use predicates::str::contains;
use serde_json::json;
use std::fs;

/// Get the binary command
fn nrz() -> Command {
    Command::cargo_bin("nrz").unwrap()
}

fn spawn_project_settings_mock() -> String {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let app = Router::new().route(
                "/v1/projects/{project_id}",
                get(
                    |axum::extract::Path(_project_id): axum::extract::Path<String>| async {
                        Json(json!({
                            "frameworkPreset": "vite",
                            "buildCommand": "npm run server-build",
                            "buildCommandSource": "USER",
                            "outputDirectory": "server-dist",
                            "outputDirectorySource": "USER"
                        }))
                    },
                ),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    rx.recv().unwrap()
}

fn spawn_project_settings_failure_mock(status: StatusCode) -> String {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let app = Router::new().route(
                "/v1/projects/{project_id}",
                get(move || async move {
                    (
                        status,
                        Json(json!({
                            "error": "project settings unavailable"
                        })),
                    )
                        .into_response()
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    rx.recv().unwrap()
}

#[test]
fn help_returns_exit_0() {
    let mut cmd = nrz();
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn root_help_does_not_expose_env_as_global_flag() {
    let output = nrz().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("--env"));
}

#[test]
fn command_scoped_env_flags_are_visible_only_where_used() {
    for args in [
        ["deploy", "--help"].as_slice(),
        ["env", "--help"].as_slice(),
        ["kv", "--help"].as_slice(),
    ] {
        let output = nrz().args(args).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("--env"),
            "expected --env in help for args {args:?}"
        );
    }
}

#[test]
fn internal_flags_are_hidden_from_help() {
    let deploy_help = nrz().args(["deploy", "--help"]).output().unwrap();
    assert!(deploy_help.status.success());
    let deploy_stdout = String::from_utf8_lossy(&deploy_help.stdout);
    assert!(!deploy_stdout.contains("--resume-deployment"));

    let detect_help = nrz().args(["detect", "--help"]).output().unwrap();
    assert!(detect_help.status.success());
    let detect_stdout = String::from_utf8_lossy(&detect_help.stdout);
    assert!(!detect_stdout.contains("--stdin"));
    assert!(!detect_stdout.contains("--needed-files"));
}

#[test]
fn project_id_works_after_nested_env_and_domains_subcommands() {
    let env_help = nrz()
        .args(["env", "validate", "--project-id", "proj_123", "--help"])
        .output()
        .unwrap();
    assert!(env_help.status.success());

    let domains_help = nrz()
        .args(["domains", "list", "--project-id", "proj_123", "--help"])
        .output()
        .unwrap();
    assert!(domains_help.status.success());
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
fn kv_env_namespaces_local_state() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "preview", "set", "shared", "preview"]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "production", "set", "shared", "production"]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "preview", "get", "shared"]);
    cmd.assert().success().stdout(contains("preview"));

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "production", "get", "shared"]);
    cmd.assert().success().stdout(contains("production"));
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

// ── nrz config ───────────────────────────────────────────────

#[test]
fn config_explain_app_merges_root_identity_with_app_config() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"name": "root", "workspaces": ["apps/*"]}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"proj_root\"\nframework = \"nextjs\"\n",
    )
    .unwrap();
    let apps_web = temp.path().join("apps").join("web");
    fs::create_dir_all(&apps_web).unwrap();
    fs::write(apps_web.join("package.json"), r#"{"name": "web"}"#).unwrap();
    fs::write(
        apps_web.join("onreza.toml"),
        "[project]\nid = \"\"\nframework = \"vite\"\n\n[build]\ncommand = \"pnpm build\"\noutput_directory = \"dist\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["config", "explain", "--app", "web", "--local"]);
    cmd.assert()
        .success()
        .stdout(contains(
            "\"selectedApp\":{\"requested\":\"web\",\"path\":\"apps/web\",\"source\":\"cli\"}",
        ))
        .stdout(contains(
            "\"projectId\":{\"value\":\"proj_root\",\"source\":\"onreza.toml\"}",
        ))
        .stdout(contains(
            "\"framework\":{\"value\":\"vite\",\"source\":\"onreza.toml\"}",
        ))
        .stdout(contains(
            "\"buildCommand\":{\"value\":\"pnpm build\",\"source\":\"onreza.toml\"}",
        ))
        .stdout(contains(
            "\"outputDirectory\":{\"value\":\"dist\",\"source\":\"onreza.toml\"}",
        ))
        .stdout(contains(
            "\"deployApp\":{\"value\":\"web\",\"source\":\"cli\"}",
        ));
}

#[test]
fn config_explain_cli_app_override_replaces_root_deploy_app() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"name": "root", "workspaces": ["apps/*"]}"#,
    )
    .unwrap();
    fs::write(temp.path().join("onreza.toml"), "[deploy]\napp = \"api\"\n").unwrap();
    for app in ["api", "web"] {
        let app_dir = temp.path().join("apps").join(app);
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(
            app_dir.join("package.json"),
            format!(r#"{{"name": "{app}"}}"#),
        )
        .unwrap();
    }

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["config", "explain", "--app", "web", "--local"]);
    cmd.assert()
        .success()
        .stdout(contains(
            "\"selectedApp\":{\"requested\":\"web\",\"path\":\"apps/web\",\"source\":\"cli\"}",
        ))
        .stdout(contains(
            "\"deployApp\":{\"value\":\"web\",\"source\":\"cli\"}",
        ));
}

#[test]
fn config_explain_project_id_override_updates_effective_project_id() {
    let api_url = spawn_project_settings_mock();
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"proj_root\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).env("NRZ_API_URL", api_url).args([
        "--token",
        "test-token",
        "config",
        "explain",
        "--project-id",
        "proj_cli",
    ]);
    cmd.assert()
        .success()
        .stdout(contains(
            "\"serverSettings\":{\"applied\":true,\"projectId\":\"proj_cli\",\"source\":\"server\"}",
        ))
        .stdout(contains(
            "\"projectId\":{\"value\":\"proj_cli\",\"source\":\"cli\"}",
        ));
}

#[test]
fn config_explain_applies_server_project_settings() {
    let api_url = spawn_project_settings_mock();
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"proj_root\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).env("NRZ_API_URL", api_url).args([
        "--token",
        "test-token",
        "config",
        "explain",
    ]);
    cmd.assert()
        .success()
        .stdout(contains(
            "\"serverSettings\":{\"applied\":true,\"projectId\":\"proj_root\",\"source\":\"server\"}",
        ))
        .stdout(contains(
            "\"framework\":{\"value\":\"vite\",\"source\":\"server\"}",
        ))
        .stdout(contains(
            "\"buildCommand\":{\"value\":\"npm run server-build\",\"source\":\"server:USER\"}",
        ))
        .stdout(contains(
            "\"outputDirectory\":{\"value\":\"server-dist\",\"source\":\"server:USER\"}",
        ));
}

#[test]
fn config_explain_uses_local_config_when_server_settings_are_transiently_unavailable() {
    let api_url = spawn_project_settings_failure_mock(StatusCode::SERVICE_UNAVAILABLE);
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"proj_root\"\n\n[build]\ncommand = \"pnpm build\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).env("NRZ_API_URL", api_url).args([
        "--token",
        "test-token",
        "config",
        "explain",
    ]);
    cmd.assert()
        .success()
        .stdout(contains(
            "\"serverSettings\":{\"applied\":false,\"projectId\":\"proj_root\",\"source\":\"server-unavailable\"}",
        ))
        .stdout(contains(
            "\"buildCommand\":{\"value\":\"pnpm build\",\"source\":\"onreza.toml\"}",
        ));
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
fn init_local_creates_scaffold_without_platform_link() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["init", "--local", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"projectId\":null"));

    assert!(temp.path().join("onreza.toml").exists());
    assert!(temp.path().join(".onreza").is_dir());
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
    let output = cmd.output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "expected one JSON object, got: {stdout}");
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(value["framework"], "vite");
    assert_eq!(value["layers"][0]["target"], "STATIC");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("Auto-generated STATIC manifest"),
        "expected build progress in stderr, got: {stderr}"
    );
}

#[test]
fn functions_check_json_emits_single_report_object() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("functions")).unwrap();
    fs::write(
        temp.path().join("functions/api.nrz-fn.ts"),
        "export const config = {};\nexport default { fetch() { return new Response('ok'); } };\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["functions", "check", "--json"]);
    let output = cmd.output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "expected one JSON object, got: {stdout}");
    let value: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(value["functions"][0]["name"], "api");
    assert_eq!(value["functions"][0]["report"]["status"], "passed");
    assert_eq!(
        value["functions"][0]["report"]["entrypoint"],
        "functions/api.nrz-fn.ts"
    );
}

#[test]
fn functions_check_json_failure_emits_single_report_object() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("functions")).unwrap();
    fs::write(
        temp.path().join("functions/api.nrz-fn.ts"),
        "export const config = {};\nexport default { fetch() { return Bun.sql`select 1`; } };\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["functions", "check", "--json"]);
    let output = cmd.output().unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "expected one JSON object, got: {stdout}");
    let value: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(value["code"], "ONREZA_FUNCTIONS_POLICY");
    assert!(
        value["error"]
            .as_str()
            .unwrap()
            .contains("function policy check failed")
    );
    assert_eq!(value["functions"][0]["report"]["status"], "failed");
}

#[test]
fn functions_check_json_accepts_static_rules_only_project() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.rules.toml"),
        r#"
schemaVersion = "EDGE_RULE_SET_V1"
source = { origin = "build" }

[[rules]]
id = "redirect-old"
condition.path = { type = "prefix", value = "/old" }
action = { type = "redirect", target = "/new" }
"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["functions", "check", "--json"]);
    let output = cmd.output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(value["functions"].as_array().unwrap().len(), 0);
    assert_eq!(value["edgeRules"]["ruleCount"], 1);
    assert_eq!(value["edgeRules"]["rules"][0]["id"], "redirect-old");
    assert_eq!(value["edgeRules"]["rules"][0]["position"], 0);
    assert_eq!(value["edgeRules"]["rules"][0]["action"], "redirect");
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
