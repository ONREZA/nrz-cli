use super::fixtures::*;
use super::*;
use axum::http::StatusCode;

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
        "[project]\nid = \"00000000-0000-0000-0000-000000000003\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).env("NRZ_API_URL", api_url).args([
        "--token",
        "test-token",
        "config",
        "explain",
        "--project-id",
        "00000000-0000-0000-0000-000000000004",
    ]);
    cmd.assert()
        .success()
        .stdout(contains(
            "\"serverSettings\":{\"applied\":true,\"projectId\":\"00000000-0000-0000-0000-000000000004\",\"source\":\"server\"}",
        ))
        .stdout(contains(
            "\"projectId\":{\"value\":\"00000000-0000-0000-0000-000000000004\",\"source\":\"cli\"}",
        ));
}

#[test]
fn config_explain_applies_server_project_settings() {
    let api_url = spawn_project_settings_mock();
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"00000000-0000-0000-0000-000000000003\"\n",
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
            "\"serverSettings\":{\"applied\":true,\"projectId\":\"00000000-0000-0000-0000-000000000003\",\"source\":\"server\"}",
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
        "[project]\nid = \"00000000-0000-0000-0000-000000000003\"\n\n[build]\ncommand = \"pnpm build\"\n",
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
            "\"serverSettings\":{\"applied\":false,\"projectId\":\"00000000-0000-0000-0000-000000000003\",\"source\":\"server-unavailable\"}",
        ))
        .stdout(contains(
            "\"buildCommand\":{\"value\":\"pnpm build\",\"source\":\"onreza.toml\"}",
        ));
}

// ── nrz detect ──────────────────────────────────────────────

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
    // Dual-channel contract: the terminal envelope on stdout carries the code for
    // CLI/automation, and a structured error frame on stderr carries it for the
    // Builder (which reads the merged log stream).
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("\"code\":\"INVALID_CONFIG\""),
        "terminal error envelope with code=INVALID_CONFIG must be on stdout, got: {stdout}"
    );
    assert!(
        stderr.contains("\"code\":\"INVALID_CONFIG\""),
        "structured error frame with code=INVALID_CONFIG must be on stderr for the Builder, got: {stderr}"
    );
}
