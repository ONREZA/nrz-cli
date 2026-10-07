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

#[test]
fn standalone_build_preserves_declared_node_in_authored_compute_manifest() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(".onreza")).unwrap();
    fs::write(temp.path().join("onreza.toml"), "[build]\noutput_directory='.'\n[deploy]\nruntime='node'\nentry='server.js'\nargs=['literal argument']\n").unwrap();
    fs::write(temp.path().join("server.js"), "console.log('server')").unwrap();
    fs::write(temp.path().join(".onreza/manifest.json"), r#"{"version":1,"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":"server.js"}],"routes":[{"pattern":"^/.*$","layer":"server"}]}"#).unwrap();
    let output = nrz()
        .current_dir(temp.path())
        .args(["build", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result = stdout_json(&output);
    assert_eq!(result["layers"][0]["target"], "COMPUTE");
    assert_eq!(result["manifestSource"], "file");
}

#[test]
fn standalone_build_runtime_failure_emits_only_one_error_object() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(".onreza")).unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[build]\noutput_directory='.'\n[deploy]\nruntime='bun'\nentry='server.js'\nargs=[]\n",
    )
    .unwrap();
    fs::write(temp.path().join("server.js"), "console.log('server')").unwrap();
    fs::write(temp.path().join(".onreza/manifest.json"), r#"{"version":1,"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":"server.js","runtime":{"applicationRuntime":{"family":"NODE","args":[]}}}],"routes":[{"pattern":"^/.*$","layer":"server"}]}"#).unwrap();
    let output = nrz()
        .current_dir(temp.path())
        .args(["build", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let result = stdout_json(&output);
    assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
    assert!(result.get("layers").is_none());
}

#[test]
fn standalone_process_build_without_manifest_emits_one_json_summary() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("dist")).unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[build]\noutput_directory='dist'\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"scripts":{"start":"npm run serve"}}"#,
    )
    .unwrap();
    fs::write(temp.path().join("dist/server.js"), "console.log('server')").unwrap();
    let output = nrz()
        .current_dir(temp.path())
        .args(["build", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let result = stdout_json(&output);
    assert_eq!(result["manifestSource"], "absent");
    assert_eq!(result["layers"], json!([]));
    assert_eq!(result["routes"], 0);
}

#[test]
fn standalone_build_explicit_static_keeps_priority_over_declared_server() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("onreza.toml"), "[build]\noutput_directory='.'\n[deploy]\ncompute='static'\nruntime='node'\nentry='server.js'\nargs=[]\n").unwrap();
    fs::write(temp.path().join("index.html"), "<h1>static export</h1>").unwrap();
    fs::write(temp.path().join("server.js"), "console.log('server')").unwrap();
    let output = nrz()
        .current_dir(temp.path())
        .args(["build", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result = stdout_json(&output);
    assert_eq!(result["layers"][0]["target"], "STATIC");
    assert_eq!(result["manifestSource"], "generated");
}

#[test]
fn python_minor_is_observable_in_detection_config_and_inherited_dry_plan() {
    let config = tempfile::tempdir().unwrap();
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{"name":"root","workspaces":["apps/*"]}"#,
        )
        .unwrap();
        let configuration = format!(
            "[project]\nframework='python'\n[build]\noutput_dirs=['.']\n[deploy]\nruntime='python'\npython_version='{}'\n",
            minor.version()
        );
        fs::write(temp.path().join("onreza.toml"), &configuration).unwrap();
        let app = temp.path().join("apps/api");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("package.json"), r#"{"name":"api"}"#).unwrap();
        fs::write(app.join("main.py"), "print('ready')").unwrap();
        fs::write(app.join("onreza.toml"), "[build]\noutput_dirs=['.']\n").unwrap();
        let mut commands = [nrz(), nrz(), nrz()];
        for command in &mut commands {
            for (key, _) in std::env::vars_os() {
                if key.to_string_lossy().starts_with("NRZ_") {
                    command.env_remove(key);
                }
            }
            command
                .env("HOME", config.path())
                .env("USERPROFILE", config.path())
                .env("XDG_CONFIG_HOME", config.path())
                .env("APPDATA", config.path())
                .env("NRZ_API_URL", "http://127.0.0.1:9");
        }
        let [mut explain_command, mut plan_command, mut detect_command] = commands;
        let explained = explain_command
            .current_dir(&temp)
            .args(["--json", "config", "explain", "--app", "api", "--local"])
            .output()
            .unwrap();
        assert!(
            explained.status.success(),
            "{}",
            String::from_utf8_lossy(&explained.stdout)
        );
        let explained = stdout_json(&explained);
        assert_eq!(
            explained["effective"]["deployPythonVersion"]["value"],
            minor.version()
        );
        let plan = plan_command
            .current_dir(&temp)
            .args([
                "--token",
                "test-token",
                "--json",
                "deploy",
                "--app",
                "api",
                "--dry",
                "--skip-build",
                "--skip-install",
            ])
            .output()
            .unwrap();
        assert!(
            plan.status.success(),
            "stdout:{} stderr:{}",
            String::from_utf8_lossy(&plan.stdout),
            String::from_utf8_lossy(&plan.stderr)
        );
        let plan = stdout_json(&plan);
        assert_eq!(plan["framework"]["runtimeVersion"], minor.version());
        assert_eq!(
            plan["framework"]["pythonPatchVersion"],
            minor.exact_version()
        );
        // Local detection reads the selected project's declaration directly.
        fs::write(app.join("onreza.toml"), &configuration).unwrap();
        let detected = detect_command
            .current_dir(&app)
            .args(["--json", "detect"])
            .output()
            .unwrap();
        assert!(detected.status.success());
        let detected = stdout_json(&detected);
        assert_eq!(detected["metadata"]["runtime"]["version"], minor.version());
        assert_eq!(
            detected["metadata"]["applicationRuntime"]["pythonVersion"],
            minor.version()
        );
    }
}
