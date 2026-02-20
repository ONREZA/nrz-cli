use super::vite_config::*;

#[test]
fn parse_vite_out_dir_double_quotes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        r#"
export default defineConfig({
  build: {
    outDir: "custom-dist",
  },
})
"#,
    )
    .unwrap();
    assert_eq!(
        parse_vite_out_dir(dir.path()),
        Some("custom-dist".to_string())
    );
}

#[test]
fn parse_vite_out_dir_single_quotes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.js"),
        "export default { build: {\n    outDir: 'build',\n  } }",
    )
    .unwrap();
    assert_eq!(parse_vite_out_dir(dir.path()), Some("build".to_string()));
}

#[test]
fn parse_vite_out_dir_mts_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.mts"),
        "export default { build: { outDir: \"output\" } }",
    )
    .unwrap();
    assert_eq!(parse_vite_out_dir(dir.path()), Some("output".to_string()));
}

#[test]
fn parse_vite_out_dir_mjs_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.mjs"),
        "export default { build: { outDir: 'public' } }",
    )
    .unwrap();
    assert_eq!(parse_vite_out_dir(dir.path()), Some("public".to_string()));
}

#[test]
fn no_vite_config_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(parse_vite_out_dir(dir.path()).is_none());
}

#[test]
fn vite_config_without_out_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        "export default defineConfig({ plugins: [react()] })",
    )
    .unwrap();
    assert!(parse_vite_out_dir(dir.path()).is_none());
}

#[test]
fn has_vite_config_true() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vite.config.ts"), "").unwrap();
    assert!(has_vite_config(dir.path()));
}

#[test]
fn has_vite_config_false() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!has_vite_config(dir.path()));
}

#[test]
fn parse_out_dir_with_spaces() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        "export default { build: {\n    outDir :  \"my-dist\" ,\n} }",
    )
    .unwrap();
    assert_eq!(parse_vite_out_dir(dir.path()), Some("my-dist".to_string()));
}

#[test]
fn ts_config_takes_priority_over_js() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vite.config.ts"),
        "export default { build: { outDir: \"from-ts\" } }",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("vite.config.js"),
        "export default { build: { outDir: \"from-js\" } }",
    )
    .unwrap();
    assert_eq!(parse_vite_out_dir(dir.path()), Some("from-ts".to_string()));
}
