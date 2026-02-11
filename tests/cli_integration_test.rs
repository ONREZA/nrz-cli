//! CLI integration tests
//!
//! Tests:
//! - nrz --help → exit 0
//! - nrz kv set/get в tempdir
//! - nrz db execute + nrz db info в tempdir
//! - nrz dev без package.json → ошибка

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
    
    // Set a key
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "mykey", "myvalue"]);
    cmd.assert().success().stderr(contains("OK"));

    // Get the key
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "get", "mykey"]);
    cmd.assert().success().stdout(contains("myvalue"));
}

#[test]
fn kv_get_nonexistent_key() {
    let temp = tempfile::tempdir().unwrap();
    
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "get", "nonexistent"]);
    cmd.assert().success().stderr(contains("(not found)"));
}

#[test]
fn kv_set_with_ttl() {
    let temp = tempfile::tempdir().unwrap();
    
    // Set with TTL
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "set", "tempkey", "tempvalue", "--ttl", "3600"]);
    cmd.assert().success().stderr(contains("OK"));

    // Verify it's set
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "get", "tempkey"]);
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

    // List keys
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

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "delete", "delkey"]);
    cmd.assert().success().stderr(contains("deleted"));

    // Verify it's gone
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "get", "delkey"]);
    cmd.assert().success().stderr(contains("(not found)"));
}

#[test]
fn db_execute_creates_database() {
    let temp = tempfile::tempdir().unwrap();
    
    // Execute CREATE TABLE
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)"]);
    cmd.assert().success();

    // Verify database was created
    let db_path = temp.path().join(".onreza").join("data").join("dev.db");
    assert!(db_path.exists(), "Database file should be created");
}

#[test]
fn db_execute_and_info() {
    let temp = tempfile::tempdir().unwrap();
    
    // Create table and insert data
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)"]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "INSERT INTO users (name) VALUES ('Alice'), ('Bob')"]);
    cmd.assert().success();

    // Check info shows the table
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "info"]);
    cmd.assert()
        .success()
        .stderr(contains("database:"))
        .stderr(contains("users"));
}

#[test]
fn db_info_shows_not_created_yet() {
    let temp = tempfile::tempdir().unwrap();
    
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "info"]);
    cmd.assert()
        .success()
        .stderr(contains("not created yet"));
}

#[test]
fn db_query_with_results() {
    let temp = tempfile::tempdir().unwrap();
    
    // Create and insert
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "CREATE TABLE items (id INTEGER PRIMARY KEY, val TEXT)"]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "INSERT INTO items (val) VALUES ('hello'), ('world')"]);
    cmd.assert().success();

    // Query and check output format
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "SELECT * FROM items ORDER BY id"]);
    cmd.assert()
        .success()
        .stderr(contains("id"))
        .stderr(contains("val"))
        .stderr(contains("hello"))
        .stderr(contains("world"));
}

#[test]
fn dev_without_package_json_fails() {
    let temp = tempfile::tempdir().unwrap();
    
    // Ensure no package.json exists
    let pkg_path = temp.path().join("package.json");
    assert!(!pkg_path.exists());

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["dev"]);
    
    // Should fail because package.json is missing
    cmd.assert()
        .failure()
        .stderr(contains("package.json"));
}

#[test]
fn dev_with_unknown_framework_fails() {
    let temp = tempfile::tempdir().unwrap();
    
    // Create package.json with unknown framework
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies":{"express":"^4.0"}}"#
    ).unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["dev"]);
    
    // Should fail because framework is unknown
    cmd.assert()
        .failure()
        .stderr(contains("could not detect framework"));
}

#[test]
fn dev_with_custom_command_works_without_detection() {
    let temp = tempfile::tempdir().unwrap();
    
    // Create empty package.json (no recognizable framework)
    fs::write(
        temp.path().join("package.json"),
        r#"{"name":"test"}"#
    ).unwrap();

    // Use --command flag to bypass detection
    // Note: This would still fail to spawn the actual process,
    // but we're just testing that the command parsing works
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["dev", "--command", "echo test"]);
    
    // The command parsing should succeed even if the dev server spawn fails
    // We expect this to fail at process spawn stage, not at framework detection
    let output = cmd.output().unwrap();
    
    // Should not contain "could not detect framework" error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("could not detect framework"),
        "Should not fail on framework detection when --command is provided"
    );
}
