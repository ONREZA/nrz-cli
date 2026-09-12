use super::*;

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

// ── deploy --app error paths ────────────────────────────────

#[cfg(unix)]
#[test]
fn dev_json_reserves_stdout_for_one_terminal_object() {
    let temp = tempfile::tempdir().unwrap();
    let command = "printf 'child stdout\\n'; printf 'child stderr\\n' >&2";
    let emulator_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let emulator_port = emulator_listener.local_addr().unwrap().port();
    let port = emulator_port
        .checked_sub(1)
        .expect("ephemeral emulator port must have a preceding dev port");
    drop(emulator_listener);

    let output = nrz()
        .current_dir(&temp)
        .args([
            "dev",
            "--local",
            "--command",
            command,
            "--port",
            &port.to_string(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "expected one JSON object, got: {stdout}");
    let result: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(result["status"], "exited");
    assert_eq!(result["projectDir"], temp.path().to_string_lossy().as_ref());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("child stdout"));
    assert!(stderr.contains("child stderr"));
    assert!(stderr.contains("\"p\":\"dev\""));
    assert!(
        fs::read_dir(temp.path().join(".onreza/data"))
            .unwrap()
            .next()
            .is_none(),
        "session bootstrap must not be written inside the project"
    );
}
