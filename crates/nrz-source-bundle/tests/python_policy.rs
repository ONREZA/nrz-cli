use nrz_source_bundle::{ApplicationRuntimeDeclaration, PythonMinor};
use serde_json::json;

#[test]
fn python_identities_match_the_generated_catalog_and_retain_exact_patch_pins() {
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../assets/runtime-toolchains.json")).unwrap();
    let supported = PythonMinor::ALL.map(|minor| minor.version());
    assert_eq!(catalog["python"]["supported"], json!(supported));
    assert_eq!(
        catalog["python"]["default"],
        PythonMinor::default().version()
    );
    for minor in PythonMinor::ALL {
        assert_eq!(PythonMinor::from_target(minor.target()), Some(minor));
        assert_eq!(
            PythonMinor::from_profile_name(minor.profile_name()),
            Some(minor)
        );
        assert_eq!(
            PythonMinor::for_dependency_path(&format!("{}/demo.py", minor.site_packages_root())),
            Some(minor)
        );
        assert!(
            PythonMinor::for_dependency_path(&format!(
                "{}-foreign/demo.py",
                minor.site_packages_root()
            ))
            .is_none()
        );
        let patch = minor.exact_version();
        let tail = patch
            .strip_prefix(&format!("{}.", minor.version()))
            .unwrap();
        assert!(!tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_digit()));
        assert_eq!(catalog["python"]["versions"][minor.version()], patch);
        let declaration: ApplicationRuntimeDeclaration = serde_json::from_value(
            json!({"family":"PYTHON","pythonVersion":minor.version(),"args":[]}),
        )
        .unwrap();
        declaration.validate_target(Some(minor.target())).unwrap();
        for other in PythonMinor::ALL {
            assert_eq!(
                declaration.validate_target(Some(other.target())).is_ok(),
                other == minor
            );
        }
    }
}

#[test]
fn new_bun_build_declarations_admit_only_canonical_major_one_targets() {
    let declaration: ApplicationRuntimeDeclaration =
        serde_json::from_value(json!({"family":"BUN","args":[]})).unwrap();
    for target in ["bun-1.4.2", "bun-1.0.0"] {
        declaration.validate_target(Some(target)).unwrap();
    }
    for target in [
        "bun-2.0.0",
        "bun-1",
        "bun-1.x.2",
        "bun-01.4.2",
        "bun-1.04.2",
        "bun-1.4.2-dev",
    ] {
        assert!(
            declaration.validate_target(Some(target)).is_err(),
            "{target}"
        );
    }
}
