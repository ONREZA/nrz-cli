use super::fixtures::*;
use super::*;

#[test]
fn login_requests_device_code_without_body() {
    let api_url = spawn_device_flow_mock();
    let config_dir = tempfile::tempdir().unwrap();

    let output = nrz()
        .env("NRZ_API_URL", api_url)
        .env("XDG_CONFIG_HOME", config_dir.path())
        .arg("login")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let objects = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(objects.len(), 2);
    assert_eq!(objects[0]["status"], "awaiting_authorization");
    assert_eq!(objects[1]["workspace_slug"], "test-workspace");
    assert!(config_dir.path().join("nrz/config.json").is_file());
}

#[test]
fn projects_list_accepts_null_display_name() {
    let api_url = spawn_nullable_project_mock();
    let output = nrz()
        .env("NRZ_API_URL", api_url)
        .args(["--token", "test-token", "projects", "list"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value = stdout_json(&output);
    assert_eq!(value["projects"][0]["displayName"], serde_json::Value::Null);
}

#[test]
fn link_uses_project_name_when_display_name_is_null() {
    let api_url = spawn_nullable_project_mock();
    let temp = tempfile::tempdir().unwrap();

    let output = nrz()
        .current_dir(&temp)
        .env("NRZ_API_URL", api_url)
        .args([
            "--token",
            "test-token",
            "link",
            "--project-id",
            "00000000-0000-0000-0000-000000000001",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value = stdout_json(&output);
    assert_eq!(value["project_name"], "internal-name");
    let config = fs::read_to_string(temp.path().join("onreza.toml")).unwrap();
    assert!(config.contains("name = \"internal-name\""), "{config}");
}

#[test]
fn preview_access_creates_bypass_secret_json() {
    let api_url = spawn_preview_access_mock();
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"00000000-0000-0000-0000-000000000001\"\n",
    )
    .unwrap();

    let output = nrz()
        .current_dir(&temp)
        .env("NRZ_API_URL", api_url)
        .args([
            "--token",
            "test-token",
            "preview",
            "access",
            "--url",
            "https://preview.onreza.app/docs",
            "--note",
            "agent smoke",
            "--ttl",
            "30m",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["projectId"], "00000000-0000-0000-0000-000000000001");
    assert_eq!(value["secretId"], "00000000-0000-0000-0000-000000000002");
    assert_eq!(value["headerName"], "X-ONREZA-Protection-Bypass");
    assert_eq!(value["headerValue"], "token-value");
    assert_eq!(value["queryName"], "_bypass");
    assert_eq!(value["queryValue"], "token-value");
    assert_eq!(value["expiresAt"], "2026-06-24T17:00:00Z");
    assert_eq!(value["ttlSeconds"], 1800);
    assert_eq!(value["ttlEnforced"], true);
    assert_eq!(
        value["browserUrl"],
        "https://preview.onreza.app/docs?_bypass=token-value"
    );
    assert_eq!(
        value["revokeCommand"],
        "nrz preview revoke --project-id 00000000-0000-0000-0000-000000000001 --secret-id 00000000-0000-0000-0000-000000000002"
    );
}

#[test]
fn preview_revoke_deletes_bypass_secret_json() {
    let api_url = spawn_preview_access_mock();
    let temp = tempfile::tempdir().unwrap();

    let output = nrz()
        .current_dir(&temp)
        .env("NRZ_API_URL", api_url)
        .args([
            "--token",
            "test-token",
            "preview",
            "revoke",
            "--project-id",
            "00000000-0000-0000-0000-000000000001",
            "--secret-id",
            "00000000-0000-0000-0000-000000000002",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["success"], true);
}
