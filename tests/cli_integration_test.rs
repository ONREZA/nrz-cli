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
fn dev_without_command_fails() {
    let temp = tempfile::tempdir().unwrap();

    // No onreza.toml, no --command flag → error
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["dev"]);
    cmd.assert()
        .failure()
        .stdout(contains("no dev command specified"));
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

// --- db execute: multi-statement, --file, stdin ---

#[test]
fn db_execute_multi_statement() {
    let temp = tempfile::tempdir().unwrap();

    let sql = "CREATE TABLE t1 (id INTEGER PRIMARY KEY, name TEXT); \
               INSERT INTO t1 (name) VALUES ('alice'); \
               INSERT INTO t1 (name) VALUES ('bob');";

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "execute", sql]);
    cmd.assert().success().stdout(contains("changes"));

    // Verify both rows were inserted
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "SELECT COUNT(*) as cnt FROM t1"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2"), "expected 2 rows, got: {stdout}");
}

#[test]
fn db_execute_file_flag() {
    let temp = tempfile::tempdir().unwrap();

    let schema = "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT);\n\
                  INSERT INTO users (email) VALUES ('test@example.com');";
    let schema_path = temp.path().join("schema.sql");
    fs::write(&schema_path, schema).unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "--file", schema_path.to_str().unwrap()]);
    cmd.assert().success().stdout(contains("changes"));

    // Verify data was inserted
    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "SELECT * FROM users"]);
    cmd.assert().success().stdout(contains("test@example.com"));
}

#[test]
fn db_execute_stdin() {
    let temp = tempfile::tempdir().unwrap();

    let sql = "CREATE TABLE items (id INTEGER PRIMARY KEY, val TEXT);\n\
               INSERT INTO items (val) VALUES ('hello');";

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "execute", "-"]);
    cmd.write_stdin(sql);
    cmd.assert().success().stdout(contains("changes"));
}

#[test]
fn db_execute_stdin_implicit() {
    let temp = tempfile::tempdir().unwrap();

    // When no sql arg and stdin is piped, should read from stdin
    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "execute"]);
    cmd.write_stdin("CREATE TABLE auto (id INTEGER PRIMARY KEY);");
    cmd.assert().success().stdout(contains("changes"));
}

#[test]
fn db_execute_sql_with_comments() {
    let temp = tempfile::tempdir().unwrap();

    let sql = "-- This is a comment\nCREATE TABLE commented (id INTEGER PRIMARY KEY);";

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "execute", sql]);
    cmd.assert().success();

    // Verify table was actually created
    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "SELECT name FROM sqlite_master WHERE type='table' AND name='commented'",
    ]);
    cmd.assert().success().stdout(contains("commented"));
}

#[test]
fn db_execute_file_takes_priority_over_positional() {
    let temp = tempfile::tempdir().unwrap();

    let schema_path = temp.path().join("schema.sql");
    fs::write(
        &schema_path,
        "CREATE TABLE from_file (id INTEGER PRIMARY KEY);",
    )
    .unwrap();

    // --file should take priority, positional arg should be ignored
    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "CREATE TABLE from_arg (id INT)",
        "--file",
        schema_path.to_str().unwrap(),
    ]);
    cmd.assert().success();

    // Verify from_file table exists
    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "SELECT name FROM sqlite_master WHERE type='table' AND name='from_file'",
    ]);
    cmd.assert().success().stdout(contains("from_file"));

    // Verify from_arg table does NOT exist
    let mut cmd = nrz();
    cmd.current_dir(&temp).args([
        "db",
        "execute",
        "SELECT COUNT(*) as cnt FROM sqlite_master WHERE type='table' AND name='from_arg'",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0"),
        "from_arg table should not exist, got: {stdout}"
    );
}

#[test]
fn db_execute_file_not_found() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "--file", "nonexistent.sql"]);
    cmd.assert().failure().stdout(contains("error"));
}

#[test]
fn db_execute_empty_file() {
    let temp = tempfile::tempdir().unwrap();

    let schema_path = temp.path().join("empty.sql");
    fs::write(&schema_path, "   \n  ").unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["db", "execute", "--file", schema_path.to_str().unwrap()]);
    cmd.assert().failure().stdout(contains("empty"));
}

#[test]
fn db_execute_batch_json_has_batch_field() {
    let temp = tempfile::tempdir().unwrap();

    let sql = "CREATE TABLE b1 (id INT); INSERT INTO b1 VALUES (1);";

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["db", "execute", sql]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"batch\":true"),
        "batch JSON should have batch field, got: {stdout}"
    );
}
