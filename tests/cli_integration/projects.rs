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
    let (sender, receiver) = channel();
    let app = Router::new().fallback(
        move |method: axum::http::Method, uri: axum::http::Uri, headers: HeaderMap, body: Bytes| {
            let response = response.clone();
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
        branch: "main".into(),
        root_directory: "apps/web".into(),
        node_version: nrz_api::Project200Response3NodeVersion::Node24,
        package_manager: nrz_api::ProjectRequestBodyPackageManager::Bun,
        created_at: TIMESTAMP.parse().unwrap(),
        updated_at: TIMESTAMP.parse().unwrap(),
        ..Default::default()
    })
    .unwrap();
    let (api_url, requests) = project_api(response);
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
            "createdAt":TIMESTAMP, "updatedAt":TIMESTAMP, "deployments":[], "_count":{"deployments":0,"envVars":0}
        })
    );
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
