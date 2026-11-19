use super::*;

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";
const SERVICE: &str = "00000000-0000-0000-0000-000000000002";
const SOURCE: &str = "00000000-0000-0000-0000-000000000003";

fn configure_service(
    command: &str,
    flags: &[&str],
    expected_body: serde_json::Value,
    response: serde_json::Value,
) -> std::process::Output {
    let update = command == "update";
    let app = axum::Router::new().route(
        if update {
            "/v1/projects/{id}/services/{service_id}"
        } else {
            "/v1/projects/{id}/services"
        },
        axum::routing::any(
            move |method: axum::http::Method,
                  headers: axum::http::HeaderMap,
                  axum::Json(body): axum::Json<serde_json::Value>| {
                let expected_body = expected_body.clone();
                let response = response.clone();
                async move {
                    assert_eq!(method.as_str(), if update { "PATCH" } else { "POST" });
                    assert_eq!(headers["X-API-Key"], "test-token");
                    assert_eq!(body, expected_body);
                    axum::Json(response)
                }
            },
        ),
    );
    let api_url = support::api_mock::spawn(app);
    let temp = tempfile::tempdir().unwrap();
    let mut cli = nrz();
    cli.current_dir(temp.path())
        .env("NRZ_API_URL", api_url)
        .args(["--json", "--token", "test-token", "services", command]);
    if update {
        cli.arg(SERVICE);
    }
    cli.args([
        "--project-id",
        PROJECT,
        "--name",
        "web",
        "--source-id",
        SOURCE,
    ])
    .args(flags)
    .output()
    .unwrap()
}

fn service_response(source_ref: &str, root_directory: &str) -> serde_json::Value {
    json!({
        "serviceId": SERVICE,
        "name": "web",
        "sourceId": SOURCE,
        "sourceRef": source_ref,
        "rootDirectory": root_directory
    })
}

#[test]
fn services_create_and_update_omit_unspecified_ref_and_root() {
    for (command, source_ref, root) in [("create", "trunk", "."), ("update", "release", "apps/web")]
    {
        let response = service_response(source_ref, root);
        let output = configure_service(
            command,
            &[],
            json!({ "name": "web", "sourceId": SOURCE }),
            response.clone(),
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(stdout_json(&output), response);
    }
}

#[test]
fn services_create_and_update_send_only_explicit_ref_and_root() {
    for command in ["create", "update"] {
        for flags in [
            vec!["--source-ref", "feature/web"],
            vec!["--root-directory", "apps/web"],
            vec![
                "--source-ref",
                "feature/web",
                "--root-directory",
                "apps/web",
            ],
        ] {
            let mut expected = json!({ "name": "web", "sourceId": SOURCE });
            for flag in flags.as_chunks::<2>().0 {
                expected[if flag[0] == "--source-ref" {
                    "sourceRef"
                } else {
                    "rootDirectory"
                }] = json!(flag[1]);
            }
            let response = service_response("feature/web", "apps/web");
            let output = configure_service(command, &flags, expected, response.clone());
            assert!(output.status.success(), "{command} {flags:?}: {output:?}");
            assert_eq!(stdout_json(&output), response);
        }
    }
}

#[test]
fn services_reject_mismatched_ids_and_explicit_configuration() {
    for command in ["create", "update"] {
        for (field, value) in [
            ("sourceId", PROJECT),
            ("sourceRef", "unexpected"),
            ("rootDirectory", "unexpected"),
            ("serviceId", PROJECT),
        ] {
            if command == "create" && field == "serviceId" {
                continue;
            }
            let mut response = service_response("feature/web", "apps/web");
            response[field] = json!(value);
            let output = configure_service(
                command,
                &[
                    "--source-ref",
                    "feature/web",
                    "--root-directory",
                    "apps/web",
                ],
                json!({
                    "name": "web", "sourceId": SOURCE,
                    "sourceRef": "feature/web", "rootDirectory": "apps/web"
                }),
                response,
            );
            assert_eq!(
                output.status.code(),
                Some(1),
                "{command} {field}: {output:?}"
            );
            assert!(
                stdout_json(&output)["error"]
                    .as_str()
                    .unwrap()
                    .contains("platform returned a different Service or Source configuration")
            );
        }
    }
}
