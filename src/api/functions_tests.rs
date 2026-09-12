use super::ApiClient;
use axum::{
    Json, Router,
    extract::Path,
    routing::{get, post},
};
use serde_json::{Value, json};

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT: &str = "00000000-0000-0000-0000-000000000002";
const FUNCTION: &str = "00000000-0000-0000-0000-000000000003";
const REVISION: &str = "00000000-0000-0000-0000-000000000004";

#[tokio::test]
async fn functions_and_rules_use_typed_requests_and_responses() {
    let prefix = "/v1/projects/{id}/function-activations/environments/{environment_id}";
    let app = Router::new()
        .route(&format!("{prefix}/functions"), get(|Path((project, environment)): Path<(String, String)>| async move {
            assert_eq!((project.as_str(), environment.as_str()), (PROJECT, ENVIRONMENT));
            Json(json!({"functions":[],"publishAttempts":[]}))
        }))
        .route(&format!("{prefix}/functions/{{function_id}}/revisions/{{revision_id}}/test-invoke"), post(
            |Path((project, environment, function, revision)): Path<(String,String,String,String)>, Json(body):Json<Value>| async move {
                assert_eq!((project.as_str(), environment.as_str(), function.as_str(), revision.as_str()), (PROJECT, ENVIRONMENT, FUNCTION, REVISION));
                assert_eq!(body, json!({"method":"POST","path":"/test","host":"test-invoke.onreza.internal","headers":[["accept","application/json"]]}));
                Json(json!({"invocation":{"invocationId":"request","ok":true,"response":{"status":200,"headers":[["content-type","application/json"]],"bodyBase64":"e30="}},
                    "debugTrace":{"serverTiming":null}, "revision":{"id":REVISION,"functionId":FUNCTION,"sourceSnapshotId":REVISION}}))
            }))
        .route(&format!("{prefix}/functions/publish"), post(|Json(body):Json<Value>| async move {
            assert_eq!(body, json!({"origin":"CLI","edgeRulesForce":true}));
            Json(json!({"projectId":PROJECT,"environmentId":ENVIRONMENT,"functionCount":0,"edgeRuleSetPublished":true,"warnings":[]}))
        }))
        .route(&format!("{prefix}/edge-rules"), get(|| async {
            Json(json!({"ruleSet":null,"generatedRuleSets":[],"effectiveImageSources":[]}))
        }))
        .route(&format!("{prefix}/edge-rules/status"), post(|Json(body):Json<Value>| async move {
            assert_eq!(body, json!({}));
            Json(json!({"environmentId":ENVIRONMENT,"status":"absent","active":{"present":false},"local":{"present":false}}))
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    assert!(
        client
            .functions(PROJECT, ENVIRONMENT)
            .await
            .unwrap()
            .functions
            .is_empty()
    );
    let invocation = client.test_invoke_function(PROJECT, ENVIRONMENT, FUNCTION, REVISION,
        serde_json::from_value(json!({"method":"POST","path":"/test","host":"test-invoke.onreza.internal","headers":[["accept","application/json"]]})).unwrap()
    ).await.unwrap();
    assert_eq!(
        invocation.invocation.response.unwrap().headers.unwrap(),
        vec![("content-type".to_owned(), "application/json".to_owned())]
    );
    assert!(
        client
            .publish_edge_rules(
                PROJECT,
                ENVIRONMENT,
                serde_json::from_value(json!({"origin":"CLI","edgeRulesForce":true})).unwrap()
            )
            .await
            .unwrap()
            .edge_rule_set_published
    );
    assert!(
        client
            .active_edge_rules(PROJECT, ENVIRONMENT)
            .await
            .unwrap()
            .rule_set
            .is_none()
    );
    let status = client
        .edge_rules_status(
            PROJECT,
            ENVIRONMENT,
            nrz_api::StatusRequestBody {
                edge_rules: None,
                local_invalid: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(status.status.to_string(), "absent");
    server.abort();
}
