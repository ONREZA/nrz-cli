use super::*;
use axum::{Router, http::StatusCode, routing::get};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
async fn saved_set_retry_does_not_resolve_changed_service_membership() {
    let directory = tempfile::tempdir().unwrap();
    let receipt = directory.path().join("command.json");
    let project_id = Uuid::now_v7();
    let key = Uuid::now_v7();
    let pending = PendingCommand {
        project_id,
        environment_id: Uuid::now_v7(),
        environment_selector: "production".into(),
        all: true,
        selected_services: vec![Uuid::now_v7()],
        local: BTreeMap::new(),
        request: json!({"idempotencyKey": key, "expectedGeneration": "7", "selection": {"kind": "ALL"}}),
    };
    std::fs::write(&receipt, serde_json::to_vec(&pending).unwrap()).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let app = Router::new().route(
        "/v1/projects/{id}/services",
        get(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            async { StatusCode::NOT_FOUND }
        }),
    );
    let (client, server) = crate::test_support::serve_api(app).await;
    let mut args = DeploymentSetArgs {
        project_id,
        environment: "production".into(),
        all: true,
        service_ids: vec![],
        local: vec![],
        idempotency_key: Some(key),
        expected_generation: None,
        wait_timeout: 1,
    };
    let result = prepare_command(&args, &client, key, &receipt, BTreeMap::new()).await;
    server.abort();
    assert_eq!(result.unwrap().request, pending.request);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    args.environment = "another".into();
    assert!(
        prepare_command(&args, &client, key, &receipt, BTreeMap::new())
            .await
            .is_err()
    );
}
