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
fn preview_allows_destructured_ambient_properties() {
    for source in [
        "export const config = {};\nconst { SHA256 } = Bun;\nexport default { fetch() { return new Response(String(SHA256)); } };\n",
        "export const config = {};\nconst { env } = process;\nexport default { fetch() { return new Response(env.MODE ?? ''); } };\n",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "functions/api.nrz-fn.ts", source);
        let collected = collect(tmp.path()).unwrap();
        let function = &collected.functions[0];

        let report = run_policy_preview(&function.entrypoint, &function.sources).unwrap();

        assert_eq!(report.status, PolicyStatus::Passed, "{source}");
        assert!(report.violations.is_empty(), "{source}");
    }
}

#[test]
fn preview_allows_lexically_shadowed_global_aliases() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "functions/api.nrz-fn.ts",
        "export const config = {};\nfunction capture() { const runtime = globalThis; return 1; }\nfunction echo(runtime) { return runtime; }\nexport default { capture, echo };\n",
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

#[test]
fn preview_flags_denied_globals_shadowed_by_erased_types() {
    for (source, capability) in [
        (
            "export const config = {};\ntype Bun = { sql: unknown };\nexport default { fetch() { return Bun.sql; } };\n",
            "Bun ambient runtime API",
        ),
        (
            "export const config = {};\ninterface process { exit(code: number): never }\nexport default { fetch() { process.exit(1); } };\n",
            "process control",
        ),
        (
            "export const config = {};\ntype Worker = new (path: string) => unknown;\nexport default new Worker('worker.ts');\n",
            "Worker",
        ),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "functions/api.nrz-fn.ts", source);
        let collected = collect(tmp.path()).unwrap();
        let function = &collected.functions[0];

        let report = run_policy_preview(&function.entrypoint, &function.sources).unwrap();

        assert_eq!(report.status, PolicyStatus::Failed, "{source}");
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.capability == capability),
            "{source}"
        );
    }
}

#[test]
fn preview_allows_type_positions_and_runtime_value_bindings() {
    for source in [
        "export const config = {};\nconst Bun = { sql: () => 'local' };\nexport default { fetch() { return Bun.sql(); } };\n",
        "export const config = {};\ntype Worker = string;\nconst label: Worker = 'ok';\nexport default label;\n",
        "export const config = {};\nconst enum Bun { Version = 'local' }\nexport default Bun.Version;\n",
        "export const config = {};\nconst enum process { Exit = 'local' }\nexport default process.Exit;\n",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "functions/api.nrz-fn.ts", source);
        let collected = collect(tmp.path()).unwrap();
        let function = &collected.functions[0];

        let report = run_policy_preview(&function.entrypoint, &function.sources).unwrap();

        assert_eq!(report.status, PolicyStatus::Passed, "{source}");
        assert!(report.violations.is_empty(), "{source}");
    }
}

#[test]
fn preview_flags_ambient_bun_alias_and_global_access() {
    for source in [
        "export const config = {};\nconst B = Bun;\nexport default { fetch() { return B.sql`select 1`; } };\n",
        "export const config = {};\nexport default { fetch() { return globalThis.Bun.sql`select 1`; } };\n",
        "export const config = {};\nexport default { fetch() { return global.Bun.sql`select 1`; } };\n",
        "export const config = {};\nconst root = globalThis;\nexport default { fetch() { return root.Bun.sql`select 1`; } };\n",
        "export const config = {};\nlet root;\nfunction fetch() { return root.Bun.sql`select 1`; }\nroot = globalThis;\nexport default { fetch };\n",
        "export const config = {};\nconst identity = value => value;\nexport default identity(Bun);\n",
        "export const config = {};\nconst key = new URL('https://example.test').hash;\nexport default globalThis[key];\n",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "functions/api.nrz-fn.ts", source);
        let collected = collect(tmp.path()).unwrap();
        let function = &collected.functions[0];
        let report = run_policy_preview(&function.entrypoint, &function.sources).unwrap();
        assert_eq!(report.status, PolicyStatus::Failed, "{source}");
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.capability == "Bun ambient runtime API")
        );
    }
}

#[test]
fn preview_flags_process_control_alias_and_global_access() {
    for source in [
        "export const config = {};\nconst p = process;\nexport default { fetch() { p.exit(1); } };\n",
        "export const config = {};\nexport default { fetch() { globalThis.process['exit'](1); } };\n",
        "export const config = {};\nexport default { fetch() { global.process.exit(1); } };\n",
        "export const config = {};\nconst root = globalThis;\nexport default { fetch() { root.process.exit(1); } };\n",
        "export const config = {};\nfunction expose() { return process; }\nexport default expose;\n",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "functions/api.nrz-fn.ts", source);
        let collected = collect(tmp.path()).unwrap();
        let function = &collected.functions[0];
        let report = run_policy_preview(&function.entrypoint, &function.sources).unwrap();
        assert_eq!(report.status, PolicyStatus::Failed, "{source}");
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.capability == "process control")
        );
    }
}
