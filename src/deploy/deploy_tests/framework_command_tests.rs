use super::*;

// ── is_nextjs_project ───────────────────────────────────────

#[test]
fn is_nextjs_detects_next_in_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"next":"^16.2.0","react":"^19.0.0"}}"#,
    )
    .unwrap();
    assert!(is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_detects_next_in_dev_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"next":"16.2.0"}}"#,
    )
    .unwrap();
    assert!(is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_false_for_non_next_project() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^19.0.0","vite":"^6.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_false_without_package_json() {
    let dir = tempdir().unwrap();
    assert!(!is_nextjs_project(dir.path()));
}

#[test]
fn nextjs_prebuild_clears_stale_adapter_descriptor() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"next":"15.5.0"}}"#,
    )
    .unwrap();
    let descriptor = dir.path().join(".onreza/next-adapter-output.json");
    fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
    fs::write(&descriptor, "{}").unwrap();

    clear_nextjs_descriptor_before_build(dir.path()).unwrap();

    assert!(!descriptor.exists());
}

// ── is_sveltekit_with_adapter_auto ───────────────────────────

#[test]
fn sveltekit_adapter_auto_false_for_non_sveltekit() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^19.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_node_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-node":"^5.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-node';\nexport default { kit: { adapter: adapter() } };",
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_true_with_adapter_auto_config() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-auto":"^3.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("svelte.config.js"),
        "import adapter from '@sveltejs/adapter-auto';\nexport default { kit: { adapter: adapter() } };",
    )
    .unwrap();
    assert!(is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_true_when_no_config() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}}"#,
    )
    .unwrap();
    assert!(is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_vercel_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-vercel":"^6.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_cloudflare_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-cloudflare":"^7.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn sveltekit_adapter_auto_false_when_adapter_netlify_installed() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"^2.0.0"}, "devDependencies":{"@sveltejs/adapter-netlify":"^6.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_sveltekit_with_adapter_auto(dir.path()));
}

#[test]
fn diagnostic_payload_mentions_standalone() {
    let dir = tempdir().unwrap();
    let detection = make_detection("payload", None);
    let msg = framework_process_diagnostic("payload", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("standalone"));
}

#[cfg(unix)]
#[test]
fn run_command_streaming_emits_coded_error_on_nonzero_exit() {
    let dir = tempdir().unwrap();
    let err = run_command_streaming(
        "exit 2",
        dir.path(),
        false,
        crate::output::Phase::Build,
        "debug",
        &[],
        None,
    )
    .expect_err("non-zero exit must fail");

    let msg = err.to_string();
    assert!(
        msg.contains("exit code 2"),
        "human message should keep the exit code detail: {msg}"
    );
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("non-zero exit must carry a CodedError so Builder classifies it as user-fault");
    assert_eq!(coded.code, "BUILD_EXIT_CODE");
}

#[cfg(unix)]
#[test]
fn run_command_streaming_emits_phase_specific_code_for_install() {
    let dir = tempdir().unwrap();
    let err = run_command_streaming(
        "exit 1",
        dir.path(),
        true, // JSON mode: exercises the second bail! site
        crate::output::Phase::Install,
        "user",
        &[],
        None,
    )
    .expect_err("non-zero exit must fail");
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("CodedError expected in JSON-mode path as well");
    assert_eq!(coded.code, "INSTALL_EXIT_CODE");
}

#[cfg(unix)]
#[test]
fn run_command_streaming_does_not_hang_on_orphaned_grandchild() {
    // The build command exits immediately but leaves a backgrounded grandchild
    // holding the stdout pipe write-end open. Before the process-group reap, the
    // stdout reader join blocked until the grandchild died — turning a
    // successful build into a 15-minute "build timeout". The call must now
    // return promptly because we SIGKILL the whole group after `wait()`.
    use std::sync::mpsc;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let dir = tempdir().unwrap();
        let result = run_command_streaming(
            "sleep 30 & echo started",
            dir.path(),
            true, // JSON mode: exercises the piped stdout/stderr reader-join path
            crate::output::Phase::Build,
            "debug",
            &[],
            None,
        );
        let _ = tx.send(result.is_ok());
    });

    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(ok) => assert!(ok, "command itself should succeed"),
        Err(_) => panic!(
            "run_command_streaming hung waiting for EOF on a pipe held open by an orphaned grandchild"
        ),
    }
    worker.join().unwrap();
}
