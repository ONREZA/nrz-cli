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

#[test]
fn deployments_preserve_platform_rows_in_json_and_render_human_table() {
    use axum::{Json, Router, extract, http::HeaderMap, routing::get};
    let response = json!({
        "deployments": [
            {
                "id": "89abcdef-0000-0000-0000-000000000001",
                "status": "LIVE",
                "isPreview": true,
                "isRollback": false,
                "isActive": true,
                "commitSha": "abc123",
                "branch": "main",
                "url": "https://preview.example.com",
                "createdAt": "2026-10-08T12:00:00Z",
                "deployedAt": "2026-10-08T12:01:00Z",
                "finishedAt": null
            },
            {
                "id": "01234567-0000-0000-0000-000000000002",
                "status": "SMOKE_TESTING",
                "isPreview": false,
                "isRollback": true,
                "isActive": false,
                "commitSha": "def456",
                "branch": "release",
                "url": null,
                "createdAt": "2026-10-08T12:02:00Z",
                "deployedAt": null,
                "finishedAt": "2026-10-08T12:03:00Z"
            }
        ],
        "total": 5
    });
    let mut expected = response.clone();
    expected["deployments"][0]["status"] = json!("live");
    expected["deployments"][1]["status"] = json!("smoke_testing");
    let app = Router::new().route(
        "/v1/deployments/project/{project_id}",
        get(
            move |extract::Path(project): extract::Path<String>,
                  extract::Query(query): extract::Query<
                std::collections::HashMap<String, String>,
            >,
                  headers: HeaderMap| {
                let response = response.clone();
                async move {
                    assert_eq!(project, "00000000-0000-0000-0000-000000000001");
                    assert_eq!(headers["X-API-Key"], "test-token");
                    assert_eq!(query["limit"], "3");
                    assert_eq!(query["offset"], "0");
                    Json(response)
                }
            },
        ),
    );
    let api_url = support::api_mock::spawn(app);
    let project_dir = tempfile::tempdir().unwrap();
    for mode in ["--json", "--human"] {
        let output = nrz()
            .current_dir(&project_dir)
            .env("NRZ_API_URL", &api_url)
            .args([
                mode,
                "--token",
                "test-token",
                "deployments",
                "--project-id",
                "00000000-0000-0000-0000-000000000001",
                "--limit",
                "3",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        if mode == "--json" {
            assert_eq!(stdout_json(&output), expected);
            assert!(output.stderr.is_empty(), "{output:?}");
        } else {
            assert!(output.stdout.is_empty(), "{output:?}");
            let table = String::from_utf8(output.stderr).unwrap();
            for text in [
                "89abcdef",
                "01234567",
                "live",
                "smoke_testing",
                "main",
                "release",
                "https://preview.example.com",
                "5 deployment(s)",
                "nrz preview access",
            ] {
                assert!(table.contains(text), "missing {text}: {table}");
            }
        }
    }
}

#[test]
fn deployments_reject_invalid_token_before_transport() {
    let project_dir = tempfile::tempdir().unwrap();
    let output = nrz()
        .current_dir(&project_dir)
        .env("NRZ_API_URL", "http://127.0.0.1:1")
        .args([
            "--json",
            "--token",
            "invalid\ntoken",
            "deployments",
            "--project-id",
            "00000000-0000-0000-0000-000000000001",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout_json(&output)["error"]
            .as_str()
            .unwrap()
            .contains("invalid token format")
    );
}

#[test]
fn logs_cli_preserves_filters_and_machine_and_human_output() {
    for (mode, empty) in [("--json", false), ("--human", false), ("--human", true)] {
        let project = "00000000-0000-0000-0000-000000000001";
        let deployment = "00000000-0000-0000-0000-000000000002";
        let response = serde_json::json!({
            "entries": if empty { serde_json::json!([]) } else { serde_json::json!([
                {"timestamp":"2026-09-12T00:00:00Z","level":"WARN","message":"slow path","deploymentId":deployment}
            ]) },
            "pagination":{"limit":73,"hasMore":false,"nextCursor":null},
            "filters":{"stream":"access","startTime":"2026-09-11T00:00:00Z","endTime":"2026-09-12T00:00:00Z"}
        });
        let server_response = response.clone();
        let app = axum::Router::new().route(
            "/v1/projects/{id}/runtime-logs",
            axum::routing::get(
                move |axum::extract::Path(id): axum::extract::Path<String>,
                      axum::extract::Query(query): axum::extract::Query<
                    std::collections::HashMap<String, String>,
                >| {
                    let response = server_response.clone();
                    async move {
                        assert_eq!(id, project);
                        assert_eq!(
                            query,
                            std::collections::HashMap::from([
                                ("limit".into(), "73".into()),
                                ("search".into(), "a&b + /שלום".into()),
                                ("deploymentId".into(), deployment.into()),
                            ])
                        );
                        axum::Json(response)
                    }
                },
            ),
        );
        let api_url = super::support::api_mock::spawn(app);
        let temp = tempfile::tempdir().unwrap();
        let output = nrz()
            .current_dir(&temp)
            .env("NRZ_API_URL", api_url)
            .env("NRZ_HUMAN", "false")
            .args([
                mode,
                "--token",
                "test-token",
                "logs",
                "--project-id",
                project,
                "--limit",
                "73",
                "--search",
                "a&b + /שלום",
                "--deployment-id",
                deployment,
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}, empty={empty}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if mode == "--json" {
            assert_eq!(stdout_json(&output), response);
            assert!(output.stderr.is_empty());
        } else {
            assert!(output.stdout.is_empty());
            assert_eq!(
                String::from_utf8(output.stderr).unwrap(),
                if empty {
                    "  No logs found.\n"
                } else {
                    "[2026-09-12T00:00:00Z] [WARN] slow path\n"
                }
            );
        }
    }
}
