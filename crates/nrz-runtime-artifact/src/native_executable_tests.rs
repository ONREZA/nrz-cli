use super::qualified_system_path;

#[test]
fn qualified_system_paths_use_unix_loader_components() {
    for path in ["/lib", "/lib64", "/usr/lib/x86_64-linux-gnu", "/usr/lib64"] {
        assert!(qualified_system_path(path));
    }
    for path in [
        "/library",
        "/lib/../../private",
        "/usr/lib/../../private",
        "/private",
    ] {
        assert!(!qualified_system_path(path));
    }
}
