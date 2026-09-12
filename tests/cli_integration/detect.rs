use super::*;

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
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("src/server.ts"),
        r#"import { Hono } from "hono";"#,
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
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("src/server.ts"),
        r#"import { Elysia } from "elysia";"#,
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
fn detect_stdin_hono_uses_entry_content_signal() {
    let manifest = json!({
        "tree": ["package.json", "src/server.ts"],
        "files": {
            "package.json": r#"{"dependencies":{"hono":"4.0.0"}}"#,
            "src/server.ts": r#"import { Hono } from "hono";"#
        }
    });

    let mut cmd = nrz();
    cmd.args(["detect", "--stdin", "--json"])
        .write_stdin(manifest.to_string());
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"hono\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_stdin_validation_error_has_machine_readable_code() {
    let mut cmd = nrz();
    cmd.args(["detect", "--stdin", "--json"])
        .write_stdin(r#"{"tree":["../outside"],"files":{}}"#);

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let value = stdout_json(&output);
    assert_eq!(value["code"], "DETECTION_INPUT_INVALID");
    assert!(
        value["error"]
            .as_str()
            .is_some_and(|error| error.contains("path must be relative"))
    );
}

#[test]
fn detect_needed_files_includes_server_entry_candidates() {
    let mut cmd = nrz();
    cmd.args(["detect", "--needed-files", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"src/server.ts\""))
        .stdout(contains("\"nitro.config.ts\""));
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
