use std::fs;
use std::path::Path;

use super::collect;

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
fn discovery_preserves_source_for_runtime_name_resolution() {
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
    assert_eq!(collected.functions[0].name, "BillingWebhook");
    assert!(collected.functions[0].inspected.is_none());
    assert_eq!(
        collected.functions[0].entrypoint,
        "functions/BillingWebhook.nrz-fn.ts"
    );
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
fn discovery_does_not_parse_or_execute_user_modules() {
    let tmp = tempfile::tempdir().unwrap();
    let source = "import './helper.ts'; const base = { name: 'api' }; export const config = { ...base }; export default () => {};";
    write(tmp.path(), "functions/Api.nrz-fn.ts", source);
    let collected = collect(tmp.path()).unwrap();
    assert_eq!(
        collected.functions[0].sources["functions/Api.nrz-fn.ts"],
        source
    );
    assert!(collected.functions[0].inspected.is_none());
}
