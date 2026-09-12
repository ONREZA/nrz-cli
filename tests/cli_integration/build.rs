use super::*;

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
    // Terminal outcome envelope goes to stdout (the CLI/automation contract).
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

#[test]
fn deploy_dry_json_outputs_plan_without_creating_deployment() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("index.html"), "<h1>hello</h1>").unwrap();

    let output = nrz()
        .current_dir(&temp)
        .args([
            "--token",
            "test-token",
            "--json",
            "deploy",
            "--dry",
            "--skip-build",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json = stdout_json(&output);
    assert_eq!(json["schemaVersion"], "DEPLOY_PLAN_V1");
    assert_eq!(json["framework"]["slug"], "static-html");
    assert_eq!(json["compute"], "STATIC");
    assert_eq!(json["target"]["environment"], "default");
    assert_eq!(json["build"]["outputManifestSource"], "generated");
    assert_eq!(
        json["runtimeArtifact"]["uploadStrategy"],
        "source_bundle_v1"
    );
    assert_eq!(json["sourceBundle"]["format"], "tar.zst");
    assert!(json["sourceBundle"]["sourceSizeBytes"].as_u64().unwrap() > 0);
    assert!(json["sourceBundle"]["sourceSha256"].as_str().unwrap().len() >= 12);
    assert!(
        json["sourceBundle"]["logicalManifestSha256"]
            .as_str()
            .unwrap()
            .len()
            >= 12
    );
    assert_eq!(json["files"]["deployableFiles"], 1);
}

// ── nrz config ───────────────────────────────────────────────

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
