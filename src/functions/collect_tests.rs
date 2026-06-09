use std::fs;
use std::path::Path;

use super::collect;
use crate::functions::run_policy_preview;
use nrz_fn_policy::PolicyStatus;

fn write(dir: &Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn discovers_branded_entries_under_default_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "functions/billing-webhook.nrz-fn.ts",
        r#"
export const config = {
  name: "billing-webhook",
} as const;
export default { fetch() { return new Response("ok"); } };
"#,
    );
    write(root, "functions/plain.ts", "export const ignored = true;\n");
    write(
        root,
        "functions/node_modules/dep/ignored.nrz-fn.js",
        "export const config = {};\n",
    );
    write(
        root,
        "functions/.cache/ignored.nrz-fn.ts",
        "export const config = {};\n",
    );

    let collected = collect(root).unwrap();

    assert_eq!(collected.functions.len(), 1);
    let function = &collected.functions[0];
    assert_eq!(function.name, "billing-webhook");
    assert_eq!(function.entrypoint, "functions/billing-webhook.nrz-fn.ts");
    let keys: Vec<&str> = function.sources.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["functions/billing-webhook.nrz-fn.ts"]);
}

#[test]
fn config_name_overrides_file_name() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/BillingWebhook.nrz-fn.ts",
        r#"
export const config = {
  name: "billing-webhook",
  triggers: [],
} as const;
export default {};
"#,
    );

    let collected = collect(tmp.path()).unwrap();

    assert_eq!(collected.functions.len(), 1);
    assert_eq!(collected.functions[0].name, "billing-webhook");
    assert_eq!(
        collected.functions[0].entrypoint,
        "functions/BillingWebhook.nrz-fn.ts"
    );
}

#[test]
fn rejects_duplicate_function_names() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/api.nrz-fn.ts",
        r#"
export const config = {
  name: "api",
} as const;
export default {};
"#,
    );
    write(
        tmp.path(),
        "functions/other.nrz-fn.ts",
        r#"
export const config = {
  name: "api",
} as const;
export default {};
"#,
    );

    let err = collect(tmp.path()).unwrap_err();
    assert!(
        err.to_string()
            .contains("duplicate ONREZA Function name 'api'")
    );
    assert!(err.to_string().contains("functions/api.nrz-fn.ts"));
    assert!(err.to_string().contains("functions/other.nrz-fn.ts"));
}

#[test]
fn empty_project_means_no_functions() {
    let tmp = tempfile::tempdir().unwrap();
    let collected = collect(tmp.path()).unwrap();
    assert!(collected.is_empty());
}

#[test]
fn rejects_unbranded_function_source() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/index.ts",
        "export const config = {};\n",
    );

    let collected = collect(tmp.path()).unwrap();
    assert!(collected.is_empty());
}

#[test]
fn rejects_invalid_function_name_segment() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/Billing.nrz-fn.ts",
        "export const config = {};\n",
    );

    let err = collect(tmp.path()).unwrap_err();
    assert!(err.to_string().contains("lowercase letters"));
}

#[test]
fn rejects_file_larger_than_contract_limit() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/large.nrz-fn.ts",
        &"x".repeat(128 * 1024 + 1),
    );

    let err = collect(tmp.path()).unwrap_err();
    assert!(err.to_string().contains("exceeds 131072 bytes"));
}

#[test]
fn rejects_missing_config_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/api.nrz-fn.ts",
        "export default { fetch() { return new Response('ok'); } };\n",
    );

    let err = collect(tmp.path()).unwrap_err();
    assert!(format!("{err:#}").contains("export const config"));
}

#[test]
fn rejects_user_imports() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/api.nrz-fn.ts",
        "export const config = {};\nimport './lib.ts';\nexport default {};\n",
    );

    let err = collect(tmp.path()).unwrap_err();
    assert!(err.to_string().contains("imports './lib.ts'"));
}

#[test]
fn preview_passes_for_clean_entry() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/api.nrz-fn.ts",
        "export const config = {};\nexport default { fetch() { return new Response('ok'); } };\n",
    );
    let collected = collect(tmp.path()).unwrap();
    let function = &collected.functions[0];
    let report = run_policy_preview(&function.entrypoint, &function.sources).unwrap();
    assert_eq!(report.status, PolicyStatus::Passed);
    assert!(report.violations.is_empty());
}

#[test]
fn preview_flags_denied_capability() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/api.nrz-fn.ts",
        "export const config = {};\nexport default { fetch() { return Bun.sql`select 1`; } };\n",
    );
    let collected = collect(tmp.path()).unwrap();
    let function = &collected.functions[0];
    let report = run_policy_preview(&function.entrypoint, &function.sources).unwrap();
    assert_eq!(report.status, PolicyStatus::Failed);
    assert!(
        report
            .violations
            .iter()
            .any(|v| v.capability == "Bun ambient runtime API")
    );
}
