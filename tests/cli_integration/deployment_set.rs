use super::{nrz, stdout_json, support};
use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;

#[test]
fn admitted_sets_report_one_json_error_with_recovery_details() {
    for local in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let project = Uuid::now_v7();
        let environment = Uuid::now_v7();
        let service = Uuid::now_v7();
        let deployment = Uuid::now_v7();
        let operation = Uuid::now_v7();
        let key = Uuid::now_v7();
        let checkout = root.path().canonicalize().unwrap();
        let context = json!({"workspaceId": project, "productId": project, "productEnvironmentId": environment,
            "workspaceEnvironmentId": environment, "workspaceBindingGeneration": "1", "serviceId": service, "sourceId": project});
        let request = if local {
            json!({"protocolVersion": "execution-context-v3", "environmentId": environment, "expectedGeneration": "7",
                "idempotencyKey": key, "selection": {"kind": "SERVICES"}, "services": [{"mode": "LOCAL", "serviceId": service,
                "branch": "main", "commitSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "expectedContext": context}]})
        } else {
            json!({"projectId": project, "environmentId": environment, "expectedGeneration": "7", "idempotencyKey": key,
                "selection": {"kind": "SERVICES", "services": [{"serviceId": service}]}, "envVars": {}})
        };
        let receipt = root
            .path()
            .join(format!(".onreza/deployment-sets/{key}.json"));
        std::fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let mappings = if local {
            json!({service.to_string(): checkout})
        } else {
            json!({})
        };
        std::fs::write(&receipt, json!({"projectId": project, "environmentId": environment, "environmentSelector": "production",
            "local": mappings, "all": false, "selectedServices": [service], "request": request}).to_string()).unwrap();
        let observed = Arc::new(AtomicUsize::new(0));
        let calls = observed.clone();
        let admission = json!({"operationId": operation, "generation": "8", "status": "PENDING", "replayed": true,
            "deployments": [{"id": deployment, "serviceId": service, "status": "BUILDING"}]});
        let app = Router::new()
            .route(if local { "/v1/projects/{id}/deployments/admit-set" } else { "/v1/deployments/set" }, post(move || {
                let admission = admission.clone();
                async move { Json(admission) }
            }))
            .route("/v1/deployments/{id}/runner-context", get(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                async { (StatusCode::FORBIDDEN, Json(json!({"code": "FORBIDDEN", "message": "runner snapshot unavailable", "details": {"permission": "deployments.read"}}))) }
            }))
            .route("/v1/deployments/{id}/status", get(move || async move {
                Json(json!({"id": deployment, "status": "building", "url": null, "production": false, "error": null,
                    "errorCode": null, "errorDetails": null, "runtimeArtifactGraphDigest": null, "runtimeArtifactGraph": null,
                    "createdAt": "2026-10-14T00:00:00Z", "readyAt": null}))
            }));
        let api = support::api_mock::spawn(app);
        let mut command = nrz();
        command
            .current_dir(root.path())
            .env("NRZ_API_URL", api)
            .env_remove("NRZ_RUNNER")
            .env_remove("NRZ_EDGE_BUILD_HANDOFF")
            .env_remove("ONREZA_OUTPUT_DIR")
            .args([
                "--json",
                "--token",
                "test-token",
                "deploy-set",
                "--project-id",
                &project.to_string(),
                "--environment",
                "production",
                "--service-id",
                &service.to_string(),
                "--idempotency-key",
                &key.to_string(),
                "--wait-timeout",
                "1",
            ]);
        if local {
            command.args(["--local", &format!("{service}={}", checkout.display())]);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let terminal = stdout_json(&output);
        assert_eq!(
            terminal["code"],
            if local {
                "FORBIDDEN"
            } else {
                "DEPLOY_WAIT_TIMEOUT"
            }
        );
        assert_eq!(terminal["details"]["operationId"], operation.to_string());
        assert_eq!(terminal["details"]["idempotencyKey"], key.to_string());
        assert_eq!(
            terminal["details"]["receipt"],
            receipt.to_string_lossy().as_ref()
        );
        if local {
            assert_eq!(observed.load(Ordering::SeqCst), 1);
            assert_eq!(terminal["details"]["permission"], "deployments.read");
        } else {
            assert_eq!(terminal["details"]["deploymentId"], deployment.to_string());
        }
        assert!(receipt.exists());
    }
}

#[test]
fn fresh_sets_publish_exact_selection_and_git_identity_then_replay_the_saved_request() {
    for local in [false, true] {
        for all in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let checkout = root.path().join("checkout");
            std::fs::create_dir(&checkout).unwrap();
            let git = |args: &[&str]| {
                let mut command = std::process::Command::new("git");
                for (name, _) in std::env::vars_os() {
                    if name.to_string_lossy().starts_with("GIT_") {
                        command.env_remove(name);
                    }
                }
                let output = command
                    .current_dir(&checkout)
                    .args([
                        "-c",
                        "core.hooksPath=/dev/null",
                        "-c",
                        "commit.gpgsign=false",
                        "-c",
                        "user.name=CLI Test",
                        "-c",
                        "user.email=cli@example.test",
                    ])
                    .args(args)
                    .output()
                    .unwrap();
                assert!(output.status.success(), "{output:?}");
                String::from_utf8(output.stdout).unwrap().trim().to_string()
            };
            if local {
                git(&["init", "--initial-branch=feature/private"]);
                git(&["commit", "--allow-empty", "-m", "initial source"]);
            }
            let commit = local.then(|| git(&["log", "-1", "--format=%H"]));
            let project = Uuid::now_v7();
            let environment = Uuid::now_v7();
            let first = Uuid::now_v7();
            let second = Uuid::now_v7();
            let retired = Uuid::now_v7();
            let first_job = Uuid::now_v7();
            let second_job = Uuid::now_v7();
            let operation = Uuid::now_v7();
            let key = Uuid::now_v7();
            let mut membership = if all {
                vec![first, second]
            } else {
                vec![second]
            };
            membership.sort();
            let listing_calls = Arc::new(AtomicUsize::new(0));
            let listing = listing_calls.clone();
            let services = json!({"canManage":true,"sources":[],"services": ([first, second, retired].map(|id| json!({
                "serviceId":id,"name":id.to_string(),"sourceId":project,"sourceRef":"server-branch","rootDirectory":".",
                "sourceType":if local && id == second {"CLI"} else {"GIT"},"gitUrl":null,"sourceUrl":null,
                "deletionRequestedAt":if id == retired {Some("2026-10-14T00:00:00Z")} else {None},"isDefault":id == first})))});
            let resolver_calls = Arc::new(AtomicUsize::new(0));
            let resolver = resolver_calls.clone();
            let bodies = Arc::new(Mutex::new(Vec::<Value>::new()));
            let admissions = bodies.clone();
            let runner_calls = Arc::new(AtomicUsize::new(0));
            let runner = runner_calls.clone();
            let admission = json!({"operationId":operation,"generation":"42","status":"ACCEPTED","replayed":false,
                "deployments": membership.iter().map(|id| json!({"serviceId":id,"id":if *id==first {first_job} else {second_job},
                    "status":if local && *id==second {"BUILDING"} else {"LIVE"}})).collect::<Vec<_>>()});
            let app = Router::new()
                .route("/v1/projects/{id}/services", get(move || {
                    let services=services.clone();
                    let call=listing.fetch_add(1,Ordering::SeqCst);
                    async move {
                        if call==0 {(StatusCode::OK,Json(services))} else {
                            (StatusCode::NOT_FOUND,Json(json!({"code":"NOT_FOUND","message":"membership changed"})))
                        }
                    }
                }))
                .route("/v1/projects/{id}/execution-context/resolve", post(move |Json(body):Json<Value>| {
                    resolver.fetch_add(1,Ordering::SeqCst);
                    async move {
                        assert_eq!(body["environment"],"production");
                        assert_eq!(body["selectionSource"],"EXPLICIT");
                        Json(json!({"protocolVersion":"execution-context-v3","context":{
                            "workspaceId":project,"productId":project,"productEnvironmentId":environment,
                            "workspaceEnvironmentId":environment,"workspaceBindingGeneration":"9","serviceId":body["serviceId"],"sourceId":project,
                            "workspaceSlug":"workspace","projectId":project,"projectName":"project","environmentId":environment,
                            "environmentName":"Production","environmentType":"PRODUCTION","sourceRef":"server-branch","selectionSource":"EXPLICIT"}}))
                    }
                }))
                .route("/v1/environments/{id}/serving", get(move |axum::extract::Path(id):axum::extract::Path<Uuid>| async move {
                    assert_eq!(id,environment);
                    Json(json!({"environmentId":environment,"address":null,"desired":{"generation":"41","operationId":null,"releaseId":null,
                        "phase":"IDLE","result":null,"activeEverywhere":false},"observed":{"known":false,"releaseId":null,"deploymentId":null,"members":null}}))
                }))
                .route(if local {"/v1/projects/{id}/deployments/admit-set"} else {"/v1/deployments/set"}, post(move |Json(body):Json<Value>| {
                    let mut admission=admission.clone();
                    let mut received=admissions.lock().unwrap();
                    admission["replayed"]=json!(!received.is_empty());
                    received.push(body);
                    async move {Json(admission)}
                }))
                .route("/v1/deployments/{id}/runner-context", get(move || {
                    runner.fetch_add(1,Ordering::SeqCst);
                    async {(StatusCode::FORBIDDEN,Json(json!({"code":"FORBIDDEN","message":"runner snapshot unavailable"})))}
                }))
                .route("/v1/deployments/{id}/status", get(move |axum::extract::Path(id):axum::extract::Path<Uuid>| async move {
                    Json(json!({"id":id,"status":"live","url":null,"production":false,"error":null,"errorCode":null,"errorDetails":null,
                        "runtimeArtifactGraphDigest":null,"runtimeArtifactGraph":null,"createdAt":"2026-10-14T00:00:00Z","readyAt":null}))
                }));
            let api = support::api_mock::spawn(app);
            let run = |select_all: bool, selected: &[Uuid]| {
                let mut command = nrz();
                command
                    .current_dir(root.path())
                    .env("NRZ_API_URL", &api)
                    .env("GIT_DIR", root.path().join("foreign.git"))
                    .env("GIT_WORK_TREE", root.path().join("foreign-worktree"))
                    .env_remove("NRZ_RUNNER")
                    .env_remove("NRZ_EDGE_BUILD_HANDOFF")
                    .env_remove("ONREZA_OUTPUT_DIR")
                    .args([
                        "--json",
                        "--token",
                        "test-token",
                        "deploy-set",
                        "--project-id",
                        &project.to_string(),
                        "--environment",
                        "production",
                        "--idempotency-key",
                        &key.to_string(),
                        "--wait-timeout",
                        "1",
                    ]);
                if select_all {
                    command.arg("--all");
                } else {
                    for id in selected {
                        command.args(["--service-id", &id.to_string()]);
                    }
                }
                if local {
                    command.args(["--local", &format!("{second}={}", checkout.display())]);
                }
                command.output().unwrap()
            };
            let original = run(all, &membership);
            let terminal = stdout_json(&original);
            if local {
                assert_eq!(terminal["code"], "FORBIDDEN", "{original:?}");
            } else {
                assert!(original.status.success(), "{original:?}");
                assert_eq!(terminal["status"], "live");
                assert_eq!(terminal["operation"]["operationId"], operation.to_string());
            }
            let expected = if local {
                json!({"protocolVersion":"execution-context-v3","environmentId":environment,"expectedGeneration":"41","idempotencyKey":key,
                    "selection":{"kind":if all {"ALL"} else {"SERVICES"}},"services":membership.iter().map(|id| {
                        if *id==second {json!({"mode":"LOCAL","serviceId":second,"branch":"feature/private","commitSha":commit,
                            "expectedContext":{"workspaceId":project,"productId":project,"productEnvironmentId":environment,
                            "workspaceEnvironmentId":environment,"workspaceBindingGeneration":"9","serviceId":second,"sourceId":project}})}
                        else {json!({"mode":"REMOTE","serviceId":id})}
                    }).collect::<Vec<_>>()})
            } else {
                json!({"projectId":project,"environmentId":environment,"expectedGeneration":"41","idempotencyKey":key,"envVars":{},
                "selection":if all {json!({"kind":"ALL"})} else {json!({"kind":"SERVICES","services":[{"serviceId":second}]})}})
            };
            assert_eq!(
                bodies.lock().unwrap().as_slice(),
                std::slice::from_ref(&expected)
            );
            if local {
                git(&["commit", "--allow-empty", "-m", "changed after admission"]);
            }
            let retry = run(all, &membership);
            if local {
                assert_eq!(stdout_json(&retry)["code"], "FORBIDDEN");
            } else {
                assert!(retry.status.success(), "{retry:?}");
            }
            assert_eq!(
                bodies.lock().unwrap().as_slice(),
                [expected.clone(), expected]
            );
            assert_eq!(listing_calls.load(Ordering::SeqCst), 1);
            assert_eq!(
                resolver_calls.load(Ordering::SeqCst),
                if local { 2 } else { 1 }
            );
            assert_eq!(
                runner_calls.load(Ordering::SeqCst),
                if local { 2 } else { 0 }
            );
            for (select_all, selected) in [(!all, membership.as_slice()), (false, &[first][..])] {
                let rejected = run(select_all, selected);
                assert_eq!(rejected.status.code(), Some(1), "{rejected:?}");
                assert!(
                    stdout_json(&rejected)["error"]
                        .as_str()
                        .unwrap()
                        .contains("stored admission selects a different Service set")
                );
            }
            assert_eq!(
                bodies.lock().unwrap().len(),
                2,
                "changed selection must not submit another admission"
            );
        }
    }
}

#[test]
fn fresh_sets_reject_deleted_and_unknown_services_before_admission() {
    let project = Uuid::now_v7();
    let active = Uuid::now_v7();
    let retired = Uuid::now_v7();
    let unexpected = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&unexpected);
    let app = Router::new()
        .route("/v1/projects/{id}/services", get(move || async move {
            Json(json!({"canManage":true,"sources":[],"services":([active,retired].map(|id|json!({
                "serviceId":id,"sourceId":project,"name":id.to_string(),"sourceRef":"main","rootDirectory":".",
                "sourceType":"GIT","gitUrl":null,"sourceUrl":null,"isDefault":id==active,
                "deletionRequestedAt":if id==retired {Some("2026-10-14T00:00:00Z")} else {None}
            })))}))
        }))
        .fallback(move || {
            observed.fetch_add(1,Ordering::SeqCst);
            async {(StatusCode::BAD_REQUEST,Json(json!({"code":"INVALID_INPUT","message":"unexpected admission work"})))}
        });
    let api = support::api_mock::spawn(app);
    for selected in [retired, Uuid::now_v7()] {
        let directory = tempfile::tempdir().unwrap();
        let output = nrz()
            .current_dir(directory.path())
            .env("NRZ_API_URL", &api)
            .args([
                "--json",
                "--token",
                "test-token",
                "deploy-set",
                "--project-id",
                &project.to_string(),
                "--environment",
                "production",
                "--service-id",
                &selected.to_string(),
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            stdout_json(&output)["error"]
                .as_str()
                .unwrap()
                .contains("selected Service is unavailable")
        );
    }
    assert_eq!(
        unexpected.load(Ordering::SeqCst),
        0,
        "unavailable Services must not resolve or admit a deployment"
    );
}
