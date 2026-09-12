use std::path::PathBuf;

use super::RuntimeResolver;
use super::preflight::preflight_with_runtime;

#[tokio::test]
async fn local_runtime_identity_tracks_bytes_without_a_release_or_network_request() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("local-runtime");
    std::fs::write(&path, b"local-engine-one").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let resolver = RuntimeResolver::with_local_path(Some(path.clone())).unwrap();
    let first = resolver.resolve().await.unwrap();
    assert!(first.runtime_release_id.starts_with("local-sha256:"));
    assert_eq!(first.path, path.canonicalize().unwrap());
    assert_eq!(
        resolver.status().await.unwrap().runtime_release_id,
        first.runtime_release_id
    );
    std::fs::write(&path, b"local-engine-two").unwrap();
    let second = resolver.resolve().await.unwrap();
    assert_ne!(first.runtime_release_id, second.runtime_release_id);
}

#[tokio::test]
async fn explicit_missing_local_runtime_does_not_fall_back_to_a_release() {
    let directory = tempfile::tempdir().unwrap();
    let resolver =
        RuntimeResolver::with_local_path(Some(directory.path().join("missing"))).unwrap();
    assert!(
        resolver
            .resolve()
            .await
            .unwrap_err()
            .to_string()
            .contains("does not exist")
    );
    assert!(
        resolver
            .status()
            .await
            .unwrap_err()
            .to_string()
            .contains("does not exist")
    );
}

#[tokio::test]
#[ignore = "requires NRZ_TEST_FUNCTIONS_RUNTIME pointing to a qualified native runtime"]
async fn native_runtime_loads_captured_source_and_cannot_resolve_uncaptured_siblings() {
    let path =
        PathBuf::from(std::env::var_os("NRZ_TEST_FUNCTIONS_RUNTIME").expect("native runtime path"));
    let runtime = RuntimeResolver::with_local_path(Some(path))
        .unwrap()
        .resolve()
        .await
        .unwrap();
    let source = "export const config = {}; export default { manual() {} };";
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("functions")).unwrap();
    std::fs::write(directory.path().join("functions/fixture.nrz-fn.ts"), source).unwrap();
    let mut functions = crate::functions::collect(directory.path()).unwrap();
    std::fs::remove_dir_all(directory.path().join("functions")).unwrap();
    let report = preflight_with_runtime(&mut functions, &runtime)
        .await
        .unwrap();
    assert_eq!(report.functions_loaded, 1);
    functions.functions[0].sources.insert(
        "functions/fixture.nrz-fn.ts".into(),
        "import './uncaptured.ts'; export default {};".into(),
    );
    let error = match preflight_with_runtime(&mut functions, &runtime).await {
        Ok(_) => panic!("runtime loaded a missing snapshot dependency"),
        Err(error) => error,
    };
    assert!(format!("{error:#}").contains("uncaptured.ts"));
}
