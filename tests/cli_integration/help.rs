use super::*;

#[test]
fn help_returns_exit_0() {
    let mut cmd = nrz();
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn global_bool_env_accepts_numeric_values() {
    for (key, value) in [
        ("NRZ_JSON", "1"),
        ("NRZ_JSON", "0"),
        ("NRZ_HUMAN", "1"),
        ("NRZ_HUMAN", "0"),
    ] {
        let output = nrz().env(key, value).arg("--help").output().unwrap();

        assert!(
            output.status.success(),
            "{key}={value}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn nrz_human_env_suppresses_auto_json_mode() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("index.html"), "<h1>hello</h1>").unwrap();

    let output = nrz()
        .current_dir(&temp)
        .env("NRZ_HUMAN", "1")
        .args(["detect"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Framework:"));
}

#[test]
fn root_help_does_not_expose_env_as_global_flag() {
    let output = nrz().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("--env"));
}

#[test]
fn command_scoped_env_flags_are_visible_only_where_used() {
    for args in [
        ["deploy", "--help"].as_slice(),
        ["env", "list", "--help"].as_slice(),
        ["env", "set", "--help"].as_slice(),
        ["env", "validate", "--help"].as_slice(),
        ["env", "exec", "--help"].as_slice(),
        ["kv", "--help"].as_slice(),
    ] {
        let output = nrz().args(args).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("--env"),
            "expected --env in help for args {args:?}"
        );
    }
}

#[test]
fn internal_flags_are_hidden_from_help() {
    let deploy_help = nrz().args(["deploy", "--help"]).output().unwrap();
    assert!(deploy_help.status.success());
    let deploy_stdout = String::from_utf8_lossy(&deploy_help.stdout);
    assert!(!deploy_stdout.contains("--resume-deployment"));

    let detect_help = nrz().args(["detect", "--help"]).output().unwrap();
    assert!(detect_help.status.success());
    let detect_stdout = String::from_utf8_lossy(&detect_help.stdout);
    assert!(!detect_stdout.contains("--stdin"));
    assert!(!detect_stdout.contains("--needed-files"));
}

#[test]
fn project_id_works_after_nested_env_and_domains_subcommands() {
    let env_help = nrz()
        .args(["env", "validate", "--project-id", "proj_123", "--help"])
        .output()
        .unwrap();
    assert!(env_help.status.success());

    let domains_help = nrz()
        .args(["domains", "list", "--project-id", "proj_123", "--help"])
        .output()
        .unwrap();
    assert!(domains_help.status.success());
}
