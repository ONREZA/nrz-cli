use nrz_api::{Config200Response, ConfigRequestBody, ConfigRequestBody2};
use serde_json::json;
use validator::Validate;

const ENVIRONMENT: &str = "00000000-0000-0000-0000-000000000002";

#[test]
fn service_config_writes_require_explicit_default_or_environment_target() {
    for environment in [json!(null), json!(ENVIRONMENT)] {
        let body =
            json!({"environmentId":environment,"variables":[{"key":"TOKEN","isSecret":true}]});
        let typed: ConfigRequestBody = serde_json::from_value(body.clone()).unwrap();
        assert!(typed.validate().is_ok());
        assert_eq!(serde_json::to_value(typed).unwrap(), body);
        let body = json!({"environmentId":environment,"confirmed":true});
        let typed: ConfigRequestBody2 = serde_json::from_value(body.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), body);
    }
    assert!(
        serde_json::from_value::<ConfigRequestBody>(json!({
            "variables":[{"key":"TOKEN","isSecret":true}]
        }))
        .is_err()
    );
    assert!(serde_json::from_value::<ConfigRequestBody2>(json!({"confirmed":true})).is_err());
}

#[test]
fn service_config_preserves_omitted_secret_value_and_nullable_note() {
    for note in [
        json!({}),
        json!({"note":null}),
        json!({"note":"explanation"}),
    ] {
        let mut variable = json!({"key":"TOKEN","isSecret":true});
        variable
            .as_object_mut()
            .unwrap()
            .extend(note.as_object().unwrap().clone());
        let body = json!({"environmentId":null,"variables":[variable],"confirmed":true,
            "changeCategory":true,"replaceScope":true});
        let typed: ConfigRequestBody = serde_json::from_value(body.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), body);
    }
}

#[test]
fn service_config_delete_preserves_required_confirmation_literal() {
    for body in [
        json!({"environmentId":null}),
        json!({"environmentId":null,"confirmed":false}),
    ] {
        assert!(serde_json::from_value::<ConfigRequestBody2>(body).is_err());
    }
    let invalid = ConfigRequestBody2 {
        confirmed: false,
        ..Default::default()
    };
    assert!(serde_json::to_value(invalid).is_err());
}

#[test]
fn service_config_generated_validation_keeps_variable_limits_and_key_rules() {
    let empty: ConfigRequestBody =
        serde_json::from_value(json!({"environmentId":null,"variables":[]})).unwrap();
    assert!(empty.validate().is_err());
    let invalid: ConfigRequestBody = serde_json::from_value(json!({"environmentId":null,
        "variables":[{"key":"invalid-key","value":"sensitive-value","isSecret":true}]}))
    .unwrap();
    assert!(invalid.validate().is_err());
}

#[test]
fn service_config_response_keeps_secret_redaction_and_inheritance_fields() {
    let response = json!({
        "context":{"workspaceId":ENVIRONMENT,"productId":ENVIRONMENT,"serviceId":ENVIRONMENT,
            "sourceId":ENVIRONMENT,"workspaceBindingGeneration":"1","environment":null},
        "variables":[],
        "effective":[{"id":ENVIRONMENT,"key":"TOKEN","value":null,"isSecret":true,
            "note":null,"environmentId":null,"workspaceEnvVarId":null,"scopeType":"ALL",
            "environmentIds":[],"environmentTypes":[],"previewBranch":null,
            "updatedAt":"2026-11-20T00:00:00Z","inherited":true}]
    });
    let typed: Config200Response = serde_json::from_value(response.clone()).unwrap();
    assert!(typed.effective[0].value.is_none());
    assert!(typed.effective[0].inherited);
    assert_eq!(serde_json::to_value(typed).unwrap(), response);
    let mut missing = response;
    missing["context"]
        .as_object_mut()
        .unwrap()
        .remove("environment");
    assert!(serde_json::from_value::<Config200Response>(missing).is_err());
}
