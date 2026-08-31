use super::dependency_scripts::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
fn classifies_canonical_bun_install_commands() {
    for command in [
        "bun install",
        "bun i --frozen-lockfile",
        "CI=1 /usr/local/bin/bun install && echo ready",
    ] {
        assert!(is_bun_install_command(command), "command: {command}");
    }
    for command in [
        "npm install",
        "echo ready && bun install",
        "bun run install",
        "bun install --ignore-scripts",
    ] {
        assert!(!is_bun_install_command(command), "command: {command}");
    }
}

#[cfg(unix)]
#[test]
fn bun_dependency_scripts_run_without_mutating_metadata() {
    let fixture = BunFixture::new("blocked");

    run_bun_dependency_scripts(fixture.root.path(), true, &fixture.environment(), None).unwrap();

    assert!(fixture.root.path().join("postinstall-marker").exists());
    assert_eq!(
        fs::read(fixture.root.path().join("package.json")).unwrap(),
        b"{\"private\":true}\n"
    );
    assert_eq!(
        fs::read(fixture.root.path().join("bun.lock")).unwrap(),
        b"original lock\n"
    );
}

#[cfg(unix)]
#[test]
fn bun_dependency_scripts_skip_when_none_are_blocked() {
    let fixture = BunFixture::new("none");

    run_bun_dependency_scripts(fixture.root.path(), true, &fixture.environment(), None).unwrap();

    assert!(!fixture.root.path().join("postinstall-marker").exists());
}

#[cfg(unix)]
#[test]
fn bun_dependency_script_failure_restores_metadata() {
    let fixture = BunFixture::new("failure");

    let error = run_bun_dependency_scripts(fixture.root.path(), true, &fixture.environment(), None)
        .expect_err("failed dependency script must fail the install phase");

    assert!(
        error
            .to_string()
            .contains("install command failed with exit code 23")
    );
    assert_eq!(
        fs::read(fixture.root.path().join("package.json")).unwrap(),
        b"{\"private\":true}\n"
    );
    assert_eq!(
        fs::read(fixture.root.path().join("bun.lock")).unwrap(),
        b"original lock\n"
    );
}

#[cfg(unix)]
struct BunFixture {
    root: tempfile::TempDir,
    bin: tempfile::TempDir,
    mode: String,
}

#[cfg(unix)]
impl BunFixture {
    fn new(mode: &str) -> Self {
        let root = tempdir().unwrap();
        let bin = tempdir().unwrap();
        fs::write(root.path().join("package.json"), "{\"private\":true}\n").unwrap();
        fs::write(root.path().join("bun.lock"), "original lock\n").unwrap();
        let bun = bin.path().join("bun");
        fs::write(
            &bun,
            r#"#!/bin/sh
set -eu
case "$1 $2" in
  "pm untrusted")
    if [ "$FAKE_BUN_MODE" = "none" ]; then
      echo "Found 0 untrusted dependencies with scripts."
    else
      echo "These dependencies had their lifecycle scripts blocked during install."
    fi
    ;;
  "pm trust")
    printf '%s\n' 'changed package' > package.json
    printf '%s\n' 'changed lock' > bun.lock
    : > postinstall-marker
    if [ "$FAKE_BUN_MODE" = "failure" ]; then
      exit 23
    fi
    ;;
  *) exit 64 ;;
esac
"#,
        )
        .unwrap();
        fs::set_permissions(&bun, fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            root,
            bin,
            mode: mode.into(),
        }
    }

    fn environment(&self) -> Vec<(String, String)> {
        vec![
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", self.bin.path().display()),
            ),
            ("FAKE_BUN_MODE".into(), self.mode.clone()),
        ]
    }
}
