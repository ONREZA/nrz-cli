use super::{nrz, stdout_json, support};
use axum::{Json, Router, body::Bytes, http::HeaderMap};
use serde_json::{Value, json};
use std::sync::mpsc::{Receiver, channel};

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";
const TIMESTAMP: &str = "2026-10-08T00:00:00Z";

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    headers: HeaderMap,
    body: Option<Value>,
}

fn project_api(response: Value) -> (String, Receiver<Request>) {
    project_api_with_environments(response, Vec::new())
}

fn project_api_with_environments(
    response: Value,
    environments: Vec<nrz_api::Environment200ResponseEnvironment>,
) -> (String, Receiver<Request>) {
    let environment_response = serde_json::to_value(nrz_api::Environment200Response {
        environments,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = channel();
    let app = Router::new().fallback(
        move |method: axum::http::Method, uri: axum::http::Uri, headers: HeaderMap, body: Bytes| {
            let response = if uri.path() == format!("/v1/environments/{PROJECT}") {
                environment_response.clone()
            } else {
                response.clone()
            };
            let sender = sender.clone();
            async move {
                sender
                    .send(Request {
                        method: method.to_string(),
                        path: uri.path().to_string(),
                        headers,
                        body: (!body.is_empty()).then(|| serde_json::from_slice(&body).unwrap()),
                    })
                    .unwrap();
                Json(response)
            }
        },
    );
    (support::api_mock::spawn(app), receiver)
}

fn project_command(
    api_url: &str,
    directory: &std::path::Path,
    mode: &str,
    args: &[&str],
) -> std::process::Output {
    nrz()
        .current_dir(directory)
        .env("NRZ_API_URL", api_url)
        .env("NRZ_HUMAN", "false")
        .args([mode, "--token", "test-token", "projects"])
        .args(args)
        .output()
        .unwrap()
}

fn request(receiver: &Receiver<Request>, method: &str, path: &str) -> Request {
    let request = receiver
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("CLI did not send a project request");
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.headers["X-API-Key"], "test-token");
    request
}

fn create_response() -> Value {
    serde_json::to_value(nrz_api::Project200Response2 {
        id: PROJECT.parse().unwrap(),
        name: "server-name".into(),
        source_type: nrz_api::Project200Response2SourceType::Git,
        git_url: Some("https://example.com/app.git".into()),
        branch: "release".into(),
        created_at: TIMESTAMP.parse().unwrap(),
        message: "created".into(),
        warnings: Some(vec!["SDK-only warning".into()]),
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn create_sends_explicit_settings_and_links_the_returned_project() {
    let (api_url, requests) = project_api(create_response());
    let directory = tempfile::tempdir().unwrap();
    let output = project_command(
        &api_url,
        directory.path(),
        "--json",
        &[
            "create",
            "--name",
            "app",
            "--display-name",
            "App",
            "--git-url",
            "https://example.com/app.git",
            "--branch",
            "release",
            "--framework",
            "astro",
            "--install-command",
            "npm ci",
            "--build-command",
            "npm run build",
            "--output-directory",
            "dist",
            "--link",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        request(&requests, "POST", "/v1/projects/").body,
        Some(json!({
            "name":"app", "displayName":"App", "gitUrl":"https://example.com/app.git",
            "branch":"release", "frameworkPreset":"astro", "installCommand":"npm ci",
            "installCommandSource":"USER", "buildCommand":"npm run build",
            "buildCommandSource":"USER", "outputDirectory":"dist", "outputDirectorySource":"USER",
            "rootDirectory":".", "autoDeployTriggerScope":"REPOSITORY"
        }))
    );
    assert_eq!(
        stdout_json(&output),
        json!({
            "id":PROJECT, "name":"server-name", "sourceType":"GIT",
            "gitUrl":"https://example.com/app.git", "branch":"release",
            "createdAt":TIMESTAMP, "message":"created", "linked":true
        })
    );
    let config = nrz::config::load(directory.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some(PROJECT));
    assert_eq!(config.project.name.as_deref(), Some("App"));
    assert!(
        std::fs::read_to_string(directory.path().join(".gitignore"))
            .unwrap()
            .contains(".onreza")
    );
}

#[test]
fn create_distinguishes_default_build_settings_from_explicit_empty_commands() {
    for empty_commands in [false, true] {
        let (api_url, requests) = project_api(create_response());
        let directory = tempfile::tempdir().unwrap();
        let mut args = vec!["create", "--name", "app"];
        let mut body =
            json!({"name":"app", "rootDirectory":".", "autoDeployTriggerScope":"REPOSITORY"});
        if empty_commands {
            args.extend([
                "--install-command",
                "",
                "--build-command",
                "",
                "--output-directory",
                "dist",
            ]);
            body.as_object_mut().unwrap().extend(
                json!({
                    "installCommand":"", "installCommandSource":"USER",
                    "buildCommand":"", "buildCommandSource":"USER",
                    "outputDirectory":"dist", "outputDirectorySource":"USER"
                })
                .as_object()
                .unwrap()
                .clone(),
            );
        }
        let output = project_command(&api_url, directory.path(), "--json", &args);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(request(&requests, "POST", "/v1/projects/").body, Some(body));
        assert!(stdout_json(&output).get("linked").is_none());
        assert!(!directory.path().join("onreza.toml").exists());
    }
}

fn mutation_response() -> Value {
    json!({"id":PROJECT, "message":"saved", "warnings":["SDK-only warning"]})
}

#[test]
fn update_sends_every_explicit_setting_and_preserves_machine_and_human_output() {
    for mode in ["--json", "--human"] {
        let (api_url, requests) = project_api(mutation_response());
        let directory = tempfile::tempdir().unwrap();
        let output = project_command(
            &api_url,
            directory.path(),
            mode,
            &[
                "update",
                PROJECT,
                "--display-name",
                "Renamed",
                "--git-url",
                "https://example.com/new.git",
                "--branch",
                "next",
                "--framework",
                "nuxt",
                "--install-command",
                "bun install",
                "--build-command",
                "bun run build",
                "--output-directory",
                ".output",
                "--root-directory",
                "apps/web",
                "--node-version",
                "NODE_24",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            request(&requests, "PATCH", &format!("/v1/projects/{PROJECT}")).body,
            Some(json!({
                "displayName":"Renamed", "gitUrl":"https://example.com/new.git", "branch":"next",
                "frameworkPreset":"nuxt", "installCommand":"bun install", "installCommandSource":"USER",
                "buildCommand":"bun run build", "buildCommandSource":"USER", "outputDirectory":".output",
                "outputDirectorySource":"USER", "rootDirectory":"apps/web", "nodeVersion":"NODE_24"
            }))
        );
        if mode == "--json" {
            assert_eq!(
                stdout_json(&output),
                json!({"id":PROJECT, "message":"saved"})
            );
        } else {
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("Updated project"));
        }
    }
}

#[test]
fn update_omits_unspecified_fields_and_keeps_explicit_empty_commands() {
    for empty_commands in [false, true] {
        let (api_url, requests) = project_api(mutation_response());
        let directory = tempfile::tempdir().unwrap();
        let mut args = vec!["update", PROJECT, "--display-name", "Renamed"];
        let mut body = json!({"displayName":"Renamed"});
        if empty_commands {
            args.extend([
                "--install-command",
                "",
                "--build-command",
                "",
                "--output-directory",
                "",
            ]);
            body.as_object_mut().unwrap().extend(json!({
                "installCommand":"", "installCommandSource":"USER", "buildCommand":"",
                "buildCommandSource":"USER", "outputDirectory":"", "outputDirectorySource":"USER"
            }).as_object().unwrap().clone());
        }
        let output = project_command(&api_url, directory.path(), "--json", &args);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            request(&requests, "PATCH", &format!("/v1/projects/{PROJECT}")).body,
            Some(body)
        );
    }
}

#[test]
fn info_preserves_the_cli_projection_of_project_details() {
    let response = serde_json::to_value(nrz_api::Project200Response3 {
        id: PROJECT.parse().unwrap(),
        name: "app".into(),
        display_name: Some("App".into()),
        source_type: Some(nrz_api::Project200Response2SourceType::Git),
        auto_deploy_enabled: Some(false),
        branch: Some("main".into()),
        root_directory: Some("apps/web".into()),
        node_version: Some("NODE_24".into()),
        package_manager: Some(nrz_api::ProjectRequestBodyPackageManager::Bun),
        created_at: TIMESTAMP.parse().unwrap(),
        updated_at: TIMESTAMP.parse().unwrap(),
        ..Default::default()
    })
    .unwrap();
    let environment = nrz_api::Environment200ResponseEnvironment {
        id: uuid::Uuid::from_u128(2),
        name: "app/Production/registry-id".into(),
        r#type: nrz_api::DomainResponseItemEnvironmentType::Production,
        branch_pattern: Some("main".into()),
        ..Default::default()
    };
    let environments = vec![environment];
    let expected_environments = serde_json::to_value(&environments).unwrap();
    let (api_url, requests) = project_api_with_environments(response, environments);
    let directory = tempfile::tempdir().unwrap();
    let output = project_command(&api_url, directory.path(), "--json", &["info", PROJECT]);
    assert!(output.status.success(), "{output:?}");
    assert!(
        request(&requests, "GET", &format!("/v1/projects/{PROJECT}"))
            .body
            .is_none()
    );
    assert_eq!(
        stdout_json(&output),
        json!({
            "id":PROJECT, "name":"app", "displayName":"App", "sourceType":"GIT",
            "gitUrl":null, "branch":"main", "frameworkPreset":null, "installCommand":null,
            "buildCommand":null, "outputDirectory":null, "rootDirectory":"apps/web",
            "nodeVersion":"NODE_24", "packageManager":"BUN", "autoDeployEnabled":false,
            "createdAt":TIMESTAMP, "updatedAt":TIMESTAMP, "deployments":[], "_count":{"deployments":0,"envVars":0},
            "environments":expected_environments
        })
    );
    assert!(
        request(&requests, "GET", &format!("/v1/environments/{PROJECT}"))
            .body
            .is_none()
    );
    let human = project_command(&api_url, directory.path(), "--human", &["info", PROJECT]);
    assert!(human.status.success(), "{human:?}");
    let text = String::from_utf8_lossy(&human.stderr);
    for expected in [
        "Environments",
        "app/Production/registry-id",
        "PRODUCTION",
        "00000000-0000-0000-0000-000000000002",
        "main",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
}

#[test]
fn delete_requires_force_in_machine_and_noninteractive_human_modes() {
    for mode in ["--json", "--human"] {
        let (api_url, requests) = project_api(mutation_response());
        let directory = tempfile::tempdir().unwrap();
        let denied = project_command(&api_url, directory.path(), mode, &["delete", PROJECT]);
        assert_eq!(denied.status.code(), Some(1));
        let text = if mode == "--json" {
            stdout_json(&denied)["error"].as_str().unwrap().to_string()
        } else {
            String::from_utf8_lossy(&denied.stderr).into_owned()
        };
        assert!(text.contains("--force is required"), "{text}");
        assert!(
            requests.try_recv().is_err(),
            "unconfirmed deletion reached the API"
        );
        let deleted = project_command(
            &api_url,
            directory.path(),
            mode,
            &["delete", PROJECT, "--force"],
        );
        assert!(deleted.status.success(), "{deleted:?}");
        assert!(
            request(&requests, "DELETE", &format!("/v1/projects/{PROJECT}"))
                .body
                .is_none()
        );
        if mode == "--json" {
            assert_eq!(
                stdout_json(&deleted),
                json!({"id":PROJECT, "message":"saved"})
            );
        } else {
            assert!(deleted.stdout.is_empty());
            assert!(String::from_utf8_lossy(&deleted.stderr).contains("Deleted project"));
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn terminal_project_deletion_requires_the_matching_id() {
    for (answer, accepted) in [
        (format!("  {PROJECT}  \n"), true),
        ("wrong-id\n".into(), false),
    ] {
        let (api_url, requests) = project_api(mutation_response());
        let directory = tempfile::tempdir().unwrap();
        let mut command = std::process::Command::new(nrz().get_program());
        command
            .current_dir(directory.path())
            .env("NRZ_API_URL", &api_url)
            .env("NO_COLOR", "1")
            .args([
                "--human",
                "--token",
                "test-token",
                "projects",
                "delete",
                PROJECT,
            ]);
        let output = super::terminal::output(&mut command, &answer);
        assert_eq!(output.status.success(), accepted, "{output:?}");
        assert!(output.stdout.is_empty());
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(
            text.contains(&format!("Type project ID ({PROJECT}) to confirm deletion:")),
            "{text}"
        );
        if accepted {
            request(&requests, "DELETE", &format!("/v1/projects/{PROJECT}"));
            assert!(text.contains("Deleted project"), "{text}");
        } else {
            assert!(text.contains("confirmation did not match"), "{text}");
            assert!(requests.try_recv().is_err());
        }
    }
}

#[test]
fn service_removal_requires_force_and_preserves_explicit_target_and_pending_status() {
    const SERVICE: &str = "00000000-0000-0000-0000-000000000041";
    let directory = tempfile::tempdir().unwrap();
    for status in ["REMOVING", "DELETED"] {
        let response =
            json!({"productId":PROJECT,"serviceId":SERVICE,"status":status,"message":"accepted"});
        let (api_url, requests) = project_api(response.clone());
        for mode in ["--json", "--human"] {
            let invoke = |force: bool| {
                let mut command = nrz();
                command
                    .current_dir(directory.path())
                    .env("NRZ_API_URL", &api_url)
                    .args([
                        mode,
                        "--token",
                        "test-token",
                        "--workspace",
                        "original",
                        "services",
                        "delete",
                        SERVICE,
                        "--project-id",
                        PROJECT,
                    ]);
                if force {
                    command.arg("--force");
                }
                command.output().unwrap()
            };
            let denied = invoke(false);
            assert!(!denied.status.success());
            assert!(requests.try_recv().is_err());
            if mode == "--json" {
                assert!(
                    stdout_json(&denied)["error"]
                        .as_str()
                        .unwrap()
                        .contains("--force")
                );
            } else {
                assert!(denied.stdout.is_empty());
                assert!(String::from_utf8_lossy(&denied.stderr).contains("--force"));
            }
            let accepted = invoke(true);
            assert!(accepted.status.success(), "{accepted:?}");
            let wire = request(
                &requests,
                "DELETE",
                &format!("/v1/projects/{PROJECT}/services/{SERVICE}"),
            );
            assert!(wire.body.is_none());
            assert!(requests.try_recv().is_err());
            if mode == "--json" {
                assert_eq!(stdout_json(&accepted), response);
            } else {
                assert!(accepted.stdout.is_empty());
                let stderr = String::from_utf8_lossy(&accepted.stderr);
                assert!(stderr.contains("databases and KV data are preserved"));
                if status == "REMOVING" {
                    assert!(stderr.contains("Runtime withdrawal is pending"));
                    assert!(!stderr.contains("Deleted service"));
                } else {
                    assert!(stderr.contains("Deleted service"));
                }
            }
        }
    }
}

#[test]
fn service_removal_requires_project_id_and_rejects_invalid_ids_before_transport() {
    let directory = tempfile::tempdir().unwrap();
    let (api_url, requests) = project_api(json!({}));
    for arguments in [
        vec!["services", "delete", PROJECT, "--force"],
        vec![
            "services",
            "delete",
            "../victim",
            "--project-id",
            PROJECT,
            "--force",
        ],
        vec![
            "services",
            "delete",
            PROJECT,
            "--project-id",
            "../victim",
            "--force",
        ],
    ] {
        let output = nrz()
            .current_dir(directory.path())
            .env("NRZ_API_URL", &api_url)
            .args(["--json", "--token", "test-token"])
            .args(arguments)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(requests.try_recv().is_err());
    }
}

#[test]
fn nonempty_product_delete_returns_actionable_error_without_redirecting_to_service() {
    let (sender, receiver) = channel();
    let app = Router::new().fallback(move |method: axum::http::Method, uri: axum::http::Uri| {
        let sender = sender.clone();
        async move {
            sender
                .send((method.to_string(), uri.path().to_string()))
                .unwrap();
            (
                axum::http::StatusCode::CONFLICT,
                Json(json!({"error":"Product is not empty","code":"PRODUCT_NOT_EMPTY"})),
            )
        }
    });
    let api_url = support::api_mock::spawn(app);
    let directory = tempfile::tempdir().unwrap();
    let output = project_command(
        &api_url,
        directory.path(),
        "--json",
        &["delete", PROJECT, "--force"],
    );
    assert!(!output.status.success());
    let error = stdout_json(&output);
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("nrz services delete")
    );
    assert_eq!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap(),
        ("DELETE".into(), format!("/v1/projects/{PROJECT}"))
    );
    assert!(receiver.try_recv().is_err());
}

#[test]
fn info_accepts_database_only_product_without_inventing_an_application() {
    let response = serde_json::to_value(nrz_api::Project200Response3 {
        id: PROJECT.parse().unwrap(),
        name: "database-only".into(),
        ..Default::default()
    })
    .unwrap();
    let (api_url, _requests) = project_api(response);
    let directory = tempfile::tempdir().unwrap();
    let output = project_command(&api_url, directory.path(), "--json", &["info", PROJECT]);
    assert!(output.status.success(), "{output:?}");
    let result = stdout_json(&output);
    assert_eq!(result["name"], "database-only");
    assert_eq!(result["environments"], json!([]));
    for key in [
        "sourceType",
        "branch",
        "rootDirectory",
        "nodeVersion",
        "packageManager",
        "autoDeployEnabled",
    ] {
        assert!(result[key].is_null(), "{key}: {result}");
    }
    let human = project_command(&api_url, directory.path(), "--human", &["info", PROJECT]);
    assert!(human.status.success(), "{human:?}");
    assert!(String::from_utf8_lossy(&human.stderr).contains("No environments found."));
}

#[test]
fn endpoints_keep_machine_stdout_separate_from_human_output() {
    let service = "00000000-0000-0000-0000-000000000004";
    let environment = "00000000-0000-0000-0000-000000000005";
    let response = json!({
        "context": {"productId": PROJECT, "serviceId": service, "environmentId": environment,
            "workspaceEnvironmentId": "00000000-0000-0000-0000-000000000006"},
        "canManage": false, "endpoints": [], "available": []
    });
    for mode in ["--json", "--human"] {
        let (api_url, requests) = project_api(response.clone());
        let directory = tempfile::tempdir().unwrap();
        let output = nrz()
            .current_dir(directory.path())
            .env("NRZ_API_URL", api_url)
            .args([
                mode,
                "--token",
                "test-token",
                "endpoints",
                "--project-id",
                PROJECT,
                "--service-id",
                service,
                "--environment-id",
                environment,
                "list",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        request(
            &requests,
            "GET",
            &format!(
                "/v1/projects/{PROJECT}/services/{service}/environments/{environment}/endpoints/"
            ),
        );
        if mode == "--json" {
            assert_eq!(stdout_json(&output), response);
        } else {
            assert!(
                output.stdout.is_empty(),
                "human endpoint data leaked into machine stdout"
            );
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stderr).unwrap(),
                response
            );
        }
    }
}

#[test]
fn services_discover_sources_and_configure_an_explicit_neighbor_service() {
    let service = "00000000-0000-0000-0000-000000000004";
    let source = "00000000-0000-0000-0000-000000000005";
    let connection = "00000000-0000-0000-0000-000000000006";
    let git_url = "https://github.com/onreza/private-api.git";
    let directory = tempfile::tempdir().unwrap();
    let configured = json!({"serviceId":service,"sourceId":source,"name":"private-api","sourceRef":"release","rootDirectory":"services/api"});
    let source_response =
        json!({"sourceId":source,"sourceType":"CLI","gitUrl":null,"sourceUrl":null});
    let git_source_response =
        json!({"sourceId":source,"sourceType":"GIT","gitUrl":git_url,"sourceUrl":git_url});
    let listed = json!({"canManage":true,"sources":[source_response.clone()],"services":[{
        "serviceId":service,"sourceId":source,"name":"private-api","sourceRef":"release","rootDirectory":"services/api",
        "sourceType":"CLI","gitUrl":null,"sourceUrl":null,"deletionRequestedAt":null,"isDefault":false
    }]});
    for (arguments, response, method, path, expected_body) in [
        (
            vec!["list", "--project-id", PROJECT],
            listed,
            "GET",
            format!("/v1/projects/{PROJECT}/services"),
            None,
        ),
        (
            vec!["create-source", "--project-id", PROJECT],
            source_response,
            "POST",
            format!("/v1/projects/{PROJECT}/sources"),
            Some(json!({"sourceType":"CLI"})),
        ),
        (
            vec![
                "create-source",
                "--project-id",
                PROJECT,
                "--git-url",
                git_url,
            ],
            git_source_response.clone(),
            "POST",
            format!("/v1/projects/{PROJECT}/sources"),
            Some(json!({"sourceType":"GIT","gitUrl":git_url})),
        ),
        (
            vec![
                "create-source",
                "--project-id",
                PROJECT,
                "--git-url",
                git_url,
                "--git-connection-id",
                connection,
            ],
            git_source_response,
            "POST",
            format!("/v1/projects/{PROJECT}/sources"),
            Some(json!({"sourceType":"GIT","gitUrl":git_url,"gitConnectionId":connection})),
        ),
        (
            vec![
                "create",
                "--project-id",
                PROJECT,
                "--name",
                "private-api",
                "--source-id",
                source,
                "--source-ref",
                "release",
                "--root-directory",
                "services/api",
            ],
            configured.clone(),
            "POST",
            format!("/v1/projects/{PROJECT}/services"),
            Some(
                json!({"name":"private-api","sourceId":source,"sourceRef":"release","rootDirectory":"services/api"}),
            ),
        ),
        (
            vec![
                "update",
                service,
                "--project-id",
                PROJECT,
                "--name",
                "private-api",
                "--source-id",
                source,
                "--source-ref",
                "release",
                "--root-directory",
                "services/api",
            ],
            configured,
            "PATCH",
            format!("/v1/projects/{PROJECT}/services/{service}"),
            Some(
                json!({"name":"private-api","sourceId":source,"sourceRef":"release","rootDirectory":"services/api"}),
            ),
        ),
    ] {
        let (api_url, requests) = project_api(response.clone());
        let output = nrz()
            .current_dir(directory.path())
            .env("NRZ_API_URL", api_url)
            .args(["--json", "--token", "test-token", "services"])
            .args(arguments)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(stdout_json(&output), response);
        assert_eq!(request(&requests, method, &path).body, expected_body);
    }
}
