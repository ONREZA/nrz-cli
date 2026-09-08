#[path = "support/cli.rs"]
mod cli;

use cli::{nrz, stdout_json};
use std::fs;

fn fixture(compute: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("dist")).unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"process-intent","type":"module"}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("dist/index.html"),
        "<h1>static fallback</h1>",
    )
    .unwrap();
    fs::write(dir.path().join("dist/server.mjs"), "import {createServer} from 'node:http'; createServer((_,r)=>r.end('process')).listen(process.env.PORT||3000);").unwrap();
    let selection = compute
        .map(|value| format!("compute = \"{value}\"\n"))
        .unwrap_or_default();
    fs::write(
        dir.path().join("onreza.toml"),
        format!(
            "[build]\noutput_directory = \"dist\"\n[deploy]\n{selection}entry = \"server.mjs\"\n"
        ),
    )
    .unwrap();
    dir
}

#[test]
fn process_flag_replaces_inferred_static_output_with_compute_artifact() {
    let dir = fixture(None);
    let output = nrz()
        .current_dir(dir.path())
        .args([
            "--token",
            "test-token",
            "--json",
            "deploy",
            "--dry",
            "--skip-build",
            "--skip-install",
            "--compute",
            "process",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let plan = stdout_json(&output);
    assert_eq!(plan["compute"], "PROCESS");
    assert_eq!(plan["runtimeArtifact"]["hasComputeLayer"], true);
    assert!(plan["healthCheck"].is_object());
}

#[test]
fn process_config_replaces_inferred_static_output_with_compute_artifact() {
    let dir = fixture(Some("process"));
    let output = nrz()
        .current_dir(dir.path())
        .args([
            "--token",
            "test-token",
            "--json",
            "deploy",
            "--dry",
            "--skip-build",
            "--skip-install",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let plan = stdout_json(&output);
    assert_eq!(plan["compute"], "PROCESS");
    assert_eq!(plan["runtimeArtifact"]["hasComputeLayer"], true);
}

#[test]
fn explicit_process_rejects_conflicting_authored_static_manifest() {
    let dir = fixture(Some("process"));
    fs::create_dir(dir.path().join("dist/.onreza")).unwrap();
    fs::write(dir.path().join("dist/.onreza/manifest.json"), r#"{"version":1,"layers":[{"name":"static","target":"STATIC","directory":"."}],"routes":[{"pattern":"^/.*$","layer":"static"}]}"#).unwrap();
    let output = nrz()
        .current_dir(dir.path())
        .args([
            "--token",
            "test-token",
            "--json",
            "deploy",
            "--dry",
            "--skip-build",
            "--skip-install",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(stdout_json(&output)["code"], "COMPUTE_MANIFEST_MISMATCH");
}

#[test]
fn static_mode_rejects_an_authored_process_runtime() {
    let dir = fixture(Some("static"));
    fs::create_dir(dir.path().join("dist/.onreza")).unwrap();
    fs::write(dir.path().join("dist/.onreza/manifest.json"), r#"{"version":1,"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":"server.mjs"}],"routes":[{"pattern":"^/.*$","layer":"server"}]}"#).unwrap();
    let output = nrz()
        .current_dir(dir.path())
        .args([
            "--token",
            "test-token",
            "--json",
            "deploy",
            "--dry",
            "--skip-build",
            "--skip-install",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(stdout_json(&output)["code"], "COMPUTE_MANIFEST_MISMATCH");
}
