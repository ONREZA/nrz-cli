use super::*;
use axum::{
    Json, Router,
    extract::{Path as RoutePath, Query, State},
    routing::{get, post},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT: &str = "00000000-0000-0000-0000-000000000002";
const SERVICE: &str = "00000000-0000-0000-0000-000000000004";
const OTHER_SERVICE: &str = "00000000-0000-0000-0000-000000000005";

fn config_context(service: &str, environment: Option<&str>) -> Value {
    json!({"workspaceId":PROJECT,"productId":PROJECT,"serviceId":service,"sourceId":PROJECT,
        "workspaceBindingGeneration":"1","environment":environment.map(|id|json!({
            "workspaceId":PROJECT,"productId":PROJECT,"productEnvironmentId":id,
            "workspaceEnvironmentId":PROJECT,"workspaceBindingGeneration":"1"}))})
}

fn row(value: &str, environment: Option<&str>, secret: bool) -> Value {
    json!({"id":PROJECT,"key":"TOKEN","value":value,"isSecret":secret,"note":null,
        "environmentId":environment,"workspaceEnvVarId":null,"scopeType":"ALL",
        "environmentIds":[],"environmentTypes":[],"previewBranch":null,
        "updatedAt":"2026-11-20T00:00:00Z","inherited":false})
}

fn context_json(service: &str) -> Value {
    json!({"workspaceId":PROJECT,"workspaceSlug":"test","productId":PROJECT,
        "productEnvironmentId":ENVIRONMENT,"workspaceEnvironmentId":PROJECT,"workspaceBindingGeneration":"1",
        "serviceId":service,"sourceId":PROJECT,"projectId":PROJECT,"projectName":"test",
        "environmentId":ENVIRONMENT,"environmentName":"Production","environmentType":"PRODUCTION",
        "sourceRef":null,"selectionSource":"EXPLICIT"})
}

fn services() -> Value {
    let services = [SERVICE, OTHER_SERVICE].map(|id| json!({
        "serviceId":id, "name":id, "sourceId":PROJECT, "sourceRef":"main", "rootDirectory":"",
        "sourceType":"CLI", "gitUrl":null, "sourceUrl":null, "deletionRequestedAt":null, "isDefault":false
    }));
    json!({"canManage":true,"sources":[],"services":services})
}

#[test]
fn list_preserves_canonical_scope_and_redacts_secret_plaintext() {
    let response: ServiceConfigResponse = serde_json::from_value(json!({
        "context":config_context(SERVICE,Some(ENVIRONMENT)),"variables":[],
        "effective":[row("must-never-be-output",Some(ENVIRONMENT),true)]
    }))
    .unwrap();
    let response: EnvListResponse = response.into();
    let encoded = serde_json::to_value(&response).unwrap();
    assert_eq!(encoded["total"], 1);
    assert_eq!(encoded["serviceId"], SERVICE);
    assert_eq!(encoded["environmentId"], ENVIRONMENT);
    assert_eq!(encoded["envVars"][0]["environmentId"], ENVIRONMENT);
    assert_eq!(encoded["envVars"][0]["value"], Value::Null);
    assert!(encoded["envVars"][0].get("createdAt").is_none());
    assert!(!encoded.to_string().contains("must-never-be-output"));
    assert_eq!(format_scope(&response.env_vars[0]), ENVIRONMENT);
    let mut inherited: nrz_api::Config200ResponseEffective =
        serde_json::from_value(row("plain", None, false)).unwrap();
    inherited.inherited = true;
    assert_eq!(format_scope(&inherited), "DEFAULT (inherited)");
}

#[test]
fn default_list_includes_scoped_legacy_definitions_and_excludes_overrides() {
    let mut legacy = row("secret", None, true);
    legacy["scopeType"] = json!("SELECTED");
    legacy["environmentIds"] = json!([ENVIRONMENT]);
    let response: ServiceConfigResponse = serde_json::from_value(json!({
        "context":config_context(SERVICE,None),
        "variables":[legacy, row("override", Some(ENVIRONMENT),false)],
        "effective":[]
    }))
    .unwrap();
    let listed = EnvListResponse::from(response);
    assert_eq!(listed.total, 1);
    assert_eq!(
        listed.env_vars[0].environment_ids[0].to_string(),
        ENVIRONMENT
    );
    assert_eq!(listed.env_vars[0].value, None);
}

#[derive(Default)]
struct Fixture {
    values: BTreeMap<(String, Option<String>), String>,
    writes: Vec<(String, String, Value)>,
    resolutions: Vec<Value>,
}
type SharedFixture = Arc<Mutex<Fixture>>;

async fn config_get(
    State(state): State<SharedFixture>,
    RoutePath((project, service)): RoutePath<(String, String)>,
    Query(query): Query<BTreeMap<String, String>>,
) -> Json<Value> {
    assert_eq!(project, PROJECT);
    let environment = query.get("environmentId").cloned();
    let state = state.lock().unwrap();
    let value = state
        .values
        .get(&(service.clone(), environment.clone()))
        .or_else(|| state.values.get(&(service.clone(), None)));
    Json(
        json!({"context":config_context(&service,environment.as_deref()),"variables":value.map(|value|vec![row(value,environment.as_deref(),false)]).unwrap_or_default(),
        "effective":value.map(|value|vec![row(value,environment.as_deref(),false)]).unwrap_or_default()}),
    )
}
async fn config_put(
    State(state): State<SharedFixture>,
    RoutePath((project, service)): RoutePath<(String, String)>,
    Json(body): Json<Value>,
) -> Json<Value> {
    assert_eq!(project, PROJECT);
    let environment = body["environmentId"].as_str().map(str::to_owned);
    let mut state = state.lock().unwrap();
    state.values.insert(
        (service.clone(), environment.clone()),
        body["variables"][0]["value"].as_str().unwrap().to_string(),
    );
    state.writes.push(("PUT".into(), service.clone(), body));
    Json(
        json!({"context":config_context(&service,environment.as_deref()),"message":"Service configuration updated"}),
    )
}
async fn config_delete(
    State(state): State<SharedFixture>,
    RoutePath((project, service, key)): RoutePath<(String, String, String)>,
    Json(body): Json<Value>,
) -> Json<Value> {
    assert_eq!(project, PROJECT);
    assert_eq!(key, "TOKEN");
    let environment = body["environmentId"].as_str().map(str::to_owned);
    let mut state = state.lock().unwrap();
    state.values.remove(&(service.clone(), environment.clone()));
    state.writes.push(("DELETE".into(), service.clone(), body));
    Json(
        json!({"context":config_context(&service,environment.as_deref()),"message":"Service configuration updated"}),
    )
}
fn router(state: SharedFixture) -> Router {
    Router::new().route("/v1/projects/{id}/services",get(||async {Json(services())}))
        .route("/v1/projects/{id}/services/{service}/config",get(config_get).put(config_put))
        .route("/v1/projects/{id}/services/{service}/config/{key}",axum::routing::delete(config_delete))
        .route("/v1/projects/{id}/execution-context/resolve",post(|State(state):State<SharedFixture>,Json(body):Json<Value>|async move {
            assert!(["Production",ENVIRONMENT].contains(&body["environment"].as_str().unwrap()));
            let service=body["serviceId"].as_str().unwrap().to_string();
            state.lock().unwrap().resolutions.push(body);
            Json(json!({"protocolVersion":execution_context::EXECUTION_CONTEXT_PROTOCOL,"context":context_json(&service)}))
        })).with_state(state)
}
fn set_command(environment: Option<&str>, all: bool, value: &str) -> EnvCommand {
    EnvCommand::Set {
        key: "TOKEN".into(),
        value: Some(value.into()),
        stdin: false,
        from_file: None,
        secret: false,
        plain: true,
        note: Some("test note".into()),
        environment: environment.map(str::to_owned),
        all,
        replace_scope: false,
        change_category: false,
        yes: false,
    }
}
async fn command(
    client: &ApiClient,
    dir: &Path,
    service: Option<&str>,
    command: EnvCommand,
) -> anyhow::Result<()> {
    run_for_client(
        client,
        PROJECT,
        EnvArgs {
            project_id: Some(PROJECT.into()),
            service: service.map(|id| id.parse().unwrap()),
            command,
        },
        true,
        &ProjectConfig::default(),
        dir,
    )
    .await
}

#[tokio::test]
async fn exact_service_default_and_override_requests_preserve_other_scopes() {
    let state = Arc::new(Mutex::new(Fixture::default()));
    let (client, server) = crate::test_support::serve_api(router(state.clone())).await;
    let dir = tempfile::tempdir().unwrap();
    command(
        &client,
        dir.path(),
        Some(SERVICE),
        set_command(None, true, "default"),
    )
    .await
    .unwrap();
    command(
        &client,
        dir.path(),
        Some(SERVICE),
        set_command(Some("Production"), false, "override"),
    )
    .await
    .unwrap();
    command(
        &client,
        dir.path(),
        Some(OTHER_SERVICE),
        set_command(None, true, "other service"),
    )
    .await
    .unwrap();
    command(
        &client,
        dir.path(),
        Some(SERVICE),
        EnvCommand::Delete {
            key: "TOKEN".into(),
            environment: None,
            all: true,
            yes: true,
        },
    )
    .await
    .unwrap();
    command(
        &client,
        dir.path(),
        Some(SERVICE),
        EnvCommand::List {
            environment: Some(ENVIRONMENT.into()),
            all: false,
        },
    )
    .await
    .unwrap();
    let state = state.lock().unwrap();
    assert_eq!(state.values.get(&(SERVICE.into(), None)), None);
    assert_eq!(
        state
            .values
            .get(&(SERVICE.into(), Some(ENVIRONMENT.into())))
            .map(String::as_str),
        Some("override")
    );
    assert_eq!(
        state
            .values
            .get(&(OTHER_SERVICE.into(), None))
            .map(String::as_str),
        Some("other service")
    );
    assert_eq!(
        state.writes[0],
        (
            "PUT".into(),
            SERVICE.into(),
            json!({"environmentId":null,"variables":[{"key":"TOKEN","value":"default","isSecret":false,"note":"test note"}],"replaceScope":false,"changeCategory":false,"confirmed":false})
        )
    );
    assert_eq!(state.writes[1].2["environmentId"], ENVIRONMENT);
    assert_eq!(
        state.writes[3],
        (
            "DELETE".into(),
            SERVICE.into(),
            json!({"environmentId":null,"confirmed":true})
        )
    );
    assert_eq!(
        state.resolutions.len(),
        2,
        "--all must not resolve an Environment"
    );
    assert!(
        state
            .resolutions
            .iter()
            .all(|body| body["serviceId"] == SERVICE)
    );
    server.abort();
}

#[tokio::test]
async fn linked_service_selects_defaults_and_explicit_service_wins() {
    let state = Arc::new(Mutex::new(Fixture::default()));
    let (client, server) = crate::test_support::serve_api(router(state.clone())).await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".onreza")).unwrap();
    std::fs::write(
        dir.path().join(".onreza/environment.json"),
        json!({"version":1,"environmentId":ENVIRONMENT,"serviceId":OTHER_SERVICE}).to_string(),
    )
    .unwrap();
    command(&client, dir.path(), None, set_command(None, true, "linked"))
        .await
        .unwrap();
    command(
        &client,
        dir.path(),
        Some(SERVICE),
        set_command(None, true, "explicit"),
    )
    .await
    .unwrap();
    let state = state.lock().unwrap();
    assert_eq!(state.writes[0].1, OTHER_SERVICE);
    assert_eq!(state.writes[1].1, SERVICE);
    assert!(state.resolutions.is_empty());
    server.abort();
}

#[tokio::test]
async fn ambiguous_service_and_failed_mutation_do_not_report_success() {
    let state = Arc::new(Mutex::new(Fixture::default()));
    let (client, server) = crate::test_support::serve_api(router(state.clone())).await;
    let dir = tempfile::tempdir().unwrap();
    let error = command(&client, dir.path(), None, set_command(None, true, "value"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Select a Service"));
    assert!(state.lock().unwrap().writes.is_empty());
    server.abort();
    let app = Router::new()
        .route(
            "/v1/projects/{id}/services",
            get(|| async { Json(services()) }),
        )
        .route(
            "/v1/projects/{id}/services/{service}/config",
            axum::routing::put(|| async {
                (
                    axum::http::StatusCode::FORBIDDEN,
                    Json(json!({"code":"FORBIDDEN","message":"denied"})),
                )
            }),
        );
    let (client, server) = crate::test_support::serve_api(app).await;
    assert!(
        command(
            &client,
            dir.path(),
            Some(SERVICE),
            set_command(None, true, "value")
        )
        .await
        .is_err()
    );
    server.abort();
}

#[test]
fn safety_prompt_names_only_the_selected_definition() {
    assert_eq!(
        describe_safety_change("TOKEN", true, None, true, true),
        "Update TOKEN in the Service default: replace the existing legacy scope and change category to SECRET?"
    );
    assert!(
        describe_safety_change(
            "TOKEN",
            false,
            Some(ENVIRONMENT.parse().unwrap()),
            false,
            true
        )
        .contains(&format!("Environment override {ENVIRONMENT}"))
    );
    assert!(confirm_safety(true, false, true, "Update TOKEN?").is_err());
    confirm_safety(true, true, true, "Update TOKEN?").unwrap();
    confirm_safety(false, false, true, "Update TOKEN?").unwrap();
}

#[cfg(unix)]
#[test]
fn env_exec_runs_selected_snapshot_and_preserves_child_failure() {
    let materialized: crate::execution_context::MaterializedExecutionContext = serde_json::from_value(json!({
        "protocolVersion":"execution-context-v3",
        "context":{
            "workspaceId":"workspace-1","workspaceSlug":"workspace",
            "productId":PROJECT,"productEnvironmentId":ENVIRONMENT,"workspaceEnvironmentId":"workspace-environment-1",
            "workspaceBindingGeneration":"1","serviceId":"service-1","sourceId":"source-1",
            "projectId":PROJECT,"projectName":"project",
            "environmentId":ENVIRONMENT,"environmentName":"Preview","environmentType":"PREVIEW",
            "sourceRef":"feature","selectionSource":"EXPLICIT"
        },
        "variables":{
            "ONREZA_TEST_SNAPSHOT_VALUE":"selected snapshot value",
            "NRZ_TOKEN":"private-token","NRZ_WORKSPACE":"private-workspace","NRZ_CUSTOM":"private-custom"
        },
        "secretKeys":[],
        "snapshot":{
            "fingerprint":format!("v1:{}", "a".repeat(64)),
            "resolvedAt":"2026-09-12T00:00:00Z","source":"DESIRED_STATE","deploymentId":null
        }
    })).unwrap();
    let literal_argument = "literal $(touch forbidden) ' \"";
    for exit_code in [0, 7] {
        let output = tempfile::tempdir().unwrap();
        let path = output.path().join("child-output.txt");
        let result = exec_with_environment(
            vec![
                "sh".into(),
                "-c".into(),
                "test -z \"${NRZ_TOKEN+x}${NRZ_WORKSPACE+x}${NRZ_CUSTOM+x}\" || exit 91; printf '%s|%s' \"$ONREZA_TEST_SNAPSHOT_VALUE\" \"$2\" > \"$1\"; exit \"$3\"".into(),
                "env-exec-test".into(),
                path.to_str().unwrap().into(),
                literal_argument.into(),
                exit_code.to_string(),
            ],
            &materialized,
        );
        if exit_code == 0 {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().to_string().contains("exit status: 7"));
        }
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            format!("selected snapshot value|{literal_argument}"),
            "exit_code={exit_code}"
        );
    }
}

#[test]
fn admitted_environment_accepts_present_required_and_absent_optional_declarations() {
    let config: ProjectConfig = toml::from_str(
        "[env.declarations]\nREQUIRED = { visibility = \"sensitive\", required = true }\nOPTIONAL = { visibility = \"plain\", required = false }\n",
    ).unwrap();
    let variables = std::collections::HashMap::from([("REQUIRED".into(), "snapshot-value".into())]);
    validate_materialized_env_for_deploy(&variables, false, &config).unwrap();
}
