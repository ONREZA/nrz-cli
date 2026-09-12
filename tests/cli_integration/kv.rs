use super::*;

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
fn kv_env_namespaces_local_state() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "preview", "set", "shared", "preview"]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "production", "set", "shared", "production"]);
    cmd.assert().success();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "preview", "get", "shared"]);
    cmd.assert().success().stdout(contains("preview"));

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["kv", "--env", "production", "get", "shared"]);
    cmd.assert().success().stdout(contains("production"));
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
