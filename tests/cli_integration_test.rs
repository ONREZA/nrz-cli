//! CLI integration tests
//!
//! Note: binary is run through a pipe, so stdout.is_terminal() = false
//! and JSON mode activates automatically. All assertions check JSON in stdout.

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

/// Get the binary command
fn nrz() -> Command {
    Command::cargo_bin("nrz").unwrap()
}

#[test]
fn help_returns_exit_0() {
    let mut cmd = nrz();
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn kv_set_and_get_in_tempdir() {
    let temp = tempfile::tempdir().unwrap();

    // Set a key (JSON: {"status":"ok"})
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "mykey", "myvalue"]);
    cmd.assert().success().stdout(contains("\"status\""));

    // Get the key (JSON: {"key":"mykey","value":"myvalue"})
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "mykey"]);
    cmd.assert().success().stdout(contains("myvalue"));
}

#[test]
fn kv_get_nonexistent_key() {
    let temp = tempfile::tempdir().unwrap();

    // JSON: {"key":"nonexistent","value":null}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "nonexistent"]);
    cmd.assert().success().stdout(contains("null"));
}

#[test]
fn kv_set_with_ttl() {
    let temp = tempfile::tempdir().unwrap();

    // Set with TTL (JSON: {"status":"ok"})
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "tempkey", "tempvalue", "--ttl", "3600"]);
    cmd.assert().success().stdout(contains("\"status\""));

    // Verify it's set
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "tempkey"]);
    cmd.assert().success().stdout(contains("tempvalue"));
}

#[test]
fn kv_list_keys() {
    let temp = tempfile::tempdir().unwrap();

    // Set multiple keys
    for i in 1..=3 {
        let mut cmd = nrz();
        cmd.current_dir(&temp)
            .args(["kv", "set", &format!("key{i}"), &format!("value{i}")]);
        cmd.assert().success();
    }

    // List keys (JSON: {"keys":[...]})
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "list"]);
    cmd.assert().success().stdout(contains("key1"));
}

#[test]
fn kv_delete_key() {
    let temp = tempfile::tempdir().unwrap();

    // Set and then delete
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "delkey", "delvalue"]);
    cmd.assert().success();

    // JSON: {"status":"ok"}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "delete", "delkey"]);
    cmd.assert().success().stdout(contains("\"status\""));

    // Verify it's gone (JSON: {"key":"delkey","value":null})
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["kv", "get", "delkey"]);
    cmd.assert().success().stdout(contains("null"));
}

#[test]
fn db_execute_creates_database() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)",
    ]);
    cmd.assert().success();

    let db_path = temp.path().join(".onreza").join("data").join("dev.db");
    assert!(db_path.exists(), "Database file should be created");
}

#[test]
fn db_execute_and_info() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
    ]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "INSERT INTO users (name) VALUES ('Alice'), ('Bob')",
    ]);
    cmd.assert().success();

    // JSON: {"path":"...","size":N,"tables":[{"name":"users","rows":2}]}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "info"]);
    cmd.assert()
        .success()
        .stdout(contains("users"))
        .stdout(contains("\"tables\""));
}

#[test]
fn db_info_shows_empty_when_not_created() {
    let temp = tempfile::tempdir().unwrap();

    // JSON: {"path":"...","size":0,"tables":[]}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "info"]);
    cmd.assert()
        .success()
        .stdout(contains("\"size\":0"))
        .stdout(contains("\"tables\":[]"));
}

#[test]
fn db_query_with_results() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "CREATE TABLE items (id INTEGER PRIMARY KEY, val TEXT)",
    ]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "INSERT INTO items (val) VALUES ('hello'), ('world')",
    ]);
    cmd.assert().success();

    // JSON: {"columns":["id","val"],"rows":[[1,"hello"],[2,"world"]]}
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "SELECT * FROM items ORDER BY id"]);
    cmd.assert()
        .success()
        .stdout(contains("\"columns\""))
        .stdout(contains("hello"))
        .stdout(contains("world"));
}

#[test]
fn dev_without_package_json_fails() {
    let temp = tempfile::tempdir().unwrap();

    assert!(!temp.path().join("package.json").exists());

    // JSON error in stdout: {"error":"...package.json..."}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["dev"]);
    cmd.assert().failure().stdout(contains("package.json"));
}

#[test]
fn dev_with_unknown_framework_fails() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies":{"express":"^4.0"}}"#,
    )
    .unwrap();

    // JSON error in stdout: {"error":"could not detect framework..."}
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["dev"]);
    cmd.assert()
        .failure()
        .stdout(contains("could not detect framework"));
}

#[test]
fn dev_with_custom_command_works_without_detection() {
    let temp = tempfile::tempdir().unwrap();

    fs::write(temp.path().join("package.json"), r#"{"name":"test"}"#).unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["dev", "--command", "echo test"]);

    let output = cmd.output().unwrap();

    // Should not contain "could not detect framework" error
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("could not detect framework"),
        "Should not fail on framework detection when --command is provided"
    );
}
