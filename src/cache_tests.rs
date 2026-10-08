use std::path::PathBuf;
use std::process::Command;

#[test]
fn cache_root_from_environment() {
    const EXPECTED: &str = "NRZ_TEST_CACHE_ROOT";
    if let Some(expected) = std::env::var_os(EXPECTED) {
        let expected = (expected != "<missing>").then(|| PathBuf::from(expected));
        assert_eq!(super::cache::default_root(), expected);
        return;
    }

    let cases: &[(&[(&str, &str)], &str)] = &[
        (&[], "<missing>"),
        (&[("USERPROFILE", "/profile")], "/profile/.cache"),
        (
            &[("HOME", "/home"), ("USERPROFILE", "/profile")],
            if cfg!(target_os = "macos") {
                "/home/Library/Caches"
            } else {
                "/home/.cache"
            },
        ),
        (
            &[
                ("HOME", "/home"),
                ("XDG_CACHE_HOME", "/xdg"),
                ("LOCALAPPDATA", "/local"),
            ],
            if cfg!(windows) {
                "/local"
            } else if cfg!(target_os = "macos") {
                "/home/Library/Caches"
            } else {
                "/xdg"
            },
        ),
        (
            &[("XDG_CACHE_HOME", "/xdg")],
            if cfg!(windows) { "<missing>" } else { "/xdg" },
        ),
        (
            &[("LOCALAPPDATA", "/local")],
            if cfg!(windows) { "/local" } else { "<missing>" },
        ),
    ];
    for (variables, expected) in cases {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "cache_tests::cache_root_from_environment"]);
        for variable in ["HOME", "USERPROFILE", "LOCALAPPDATA", "XDG_CACHE_HOME"] {
            command.env_remove(variable);
        }
        command
            .envs(variables.iter().copied())
            .env(EXPECTED, expected);
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{variables:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
