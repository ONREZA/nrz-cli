use super::*;
use crate::functions_runtime::RuntimeResolver;

async fn runtime() -> CachedRuntime {
    let path = std::env::var_os("NRZ_TEST_FUNCTIONS_RUNTIME").expect("native runtime path");
    RuntimeResolver::with_local_path(Some(path.into()))
        .unwrap()
        .resolve()
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires NRZ_TEST_FUNCTIONS_RUNTIME pointing to a qualified native runtime"]
async fn native_inspection_evaluates_typescript_config_and_real_handler_exports() {
    let runtime = runtime().await;
    let result = inspect(&runtime, "api.nrz-fn.ts", r#"
        const prefix: string = "billing";
        const base = { name: `${prefix}-webhook` };
        export const config = { ...base, triggers: [{ name: "run", type: "manual" }] } satisfies Record<string, unknown>;
        export default { manual() { throw new Error("inspection must not call a user handler"); } };
    "#).await.unwrap();
    assert_eq!(result.config["name"], "billing-webhook");
    assert_eq!(result.config["triggers"][0]["type"], "manual");
    assert_eq!(result.handlers, ["manual"]);
    let result = inspect(
        &runtime,
        "empty-config.nrz-fn.js",
        "export default async function () {};",
    )
    .await
    .unwrap();
    assert_eq!(result.config, json!({}));
    assert!(result.handlers.iter().any(|name| name == "fetch"));
}

#[tokio::test]
#[ignore = "requires NRZ_TEST_FUNCTIONS_RUNTIME pointing to a qualified native runtime"]
async fn native_inspection_rejects_invalid_exports_config_and_trigger_handlers() {
    let runtime = runtime().await;
    for (source, diagnostic) in [
        ("export default {};", "at least one supported handler"),
        (
            "await Promise.resolve(); export const config = {};",
            "must export a default",
        ),
        ("export default { manual: true };", "must be callable"),
        (
            "export const config = { value: undefined }; export default () => {};",
            "JSON values only",
        ),
        (
            "export const config = { triggers: [{ name: 'task', type: 'queue' }] }; export default { manual() {} };",
            "requires a 'queue' handler",
        ),
        (
            "throw new Error('initialization failed'); export default () => {};",
            "initialization failed",
        ),
    ] {
        let error = inspect(&runtime, "fixture.nrz-fn.ts", source)
            .await
            .unwrap_err();
        assert!(error.chain().any(|cause| {
            cause
                .downcast_ref::<crate::output::CodedError>()
                .is_some_and(|error| error.code == "INVALID_CONFIG")
        }));
        assert!(
            format!("{error:#}").contains(diagnostic),
            "unexpected diagnostic: {error:#}"
        );
    }
}

#[tokio::test]
#[ignore = "requires NRZ_TEST_FUNCTIONS_RUNTIME pointing to a qualified native runtime"]
async fn native_inspection_has_no_ambient_credentials_or_fetch_binding() {
    let runtime = runtime().await;
    let result = inspect(
        &runtime,
        "fixture.nrz-fn.ts",
        "export const config = { leaked: process.env.NRZ_TOKEN ?? null }; export default () => {};",
    )
    .await
    .unwrap();
    assert_eq!(result.config["leaked"], Value::Null);
    let error = inspect(
        &runtime,
        "fixture.nrz-fn.ts",
        "await fetch('https://inspection.invalid'); export default () => {};",
    )
    .await
    .unwrap_err();
    assert!(format!("{error:#}").contains("disabled"));
}

#[tokio::test]
#[ignore = "requires NRZ_TEST_FUNCTIONS_RUNTIME pointing to a qualified native runtime"]
async fn native_inspection_stops_a_module_that_never_finishes_initializing() {
    let runtime = runtime().await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        inspect(
            &runtime,
            "fixture.nrz-fn.ts",
            "await new Promise(() => {}); export default () => {};",
        ),
    )
    .await
    .expect("inspection must respect its control deadline");
    let error = result.unwrap_err();
    assert!(format!("{error:#}").contains("control timeout"));
    assert!(
        !error
            .chain()
            .any(|cause| cause.is::<crate::output::CodedError>())
    );
}

#[tokio::test]
#[ignore = "requires NRZ_TEST_FUNCTIONS_RUNTIME pointing to a qualified native runtime"]
async fn native_preflight_publishes_evaluated_metadata_and_the_exact_inspected_source() {
    let runtime = runtime().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("API.nrz-fn.ts");
    let source = "const name = ['billing', 'task'].join('-'); export const config = { name, triggers: [{ name: 'run', type: 'manual' }] }; export default { manual() {} };";
    std::fs::write(&path, source).unwrap();
    let mut collected = crate::functions::collect(directory.path()).unwrap();
    std::fs::write(&path, "throw new Error('edited after discovery');").unwrap();
    crate::functions_runtime::preflight::preflight_with_runtime(&mut collected, &runtime)
        .await
        .unwrap();
    let payload =
        crate::functions::build_payload("DEPLOYMENT", &collected, None, false, vec![]).unwrap();
    assert_eq!(payload.functions[0].declaration.name, "billing-task");
    assert_eq!(payload.functions[0].source.content_text, source);
    assert_eq!(
        payload.functions[0].handlers,
        [nrz_api::FunctionHandler::Manual]
    );
    assert_eq!(payload.functions[0].declaration.triggers[0].name, "run");
}

#[tokio::test]
#[ignore = "requires NRZ_TEST_FUNCTIONS_RUNTIME pointing to a qualified native runtime"]
async fn native_preflight_rejects_duplicate_evaluated_names_before_publish() {
    let runtime = runtime().await;
    let directory = tempfile::tempdir().unwrap();
    for name in ["one", "two"] {
        std::fs::write(
            directory.path().join(format!("{name}.nrz-fn.ts")),
            "export const config = { name: 'same' }; export default () => {};",
        )
        .unwrap();
    }
    let mut collected = crate::functions::collect(directory.path()).unwrap();
    let error =
        crate::functions_runtime::preflight::preflight_with_runtime(&mut collected, &runtime)
            .await
            .err()
            .expect("duplicate name should fail");
    assert!(error.to_string().contains("duplicate ONREZA Function name"));
    assert!(
        crate::functions::build_payload("DEPLOYMENT", &collected, None, false, vec![]).is_err()
    );
}
