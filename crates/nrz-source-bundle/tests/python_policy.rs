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
        assert_eq!(PythonMinor::from_target(&minor.target()), Some(minor));
        assert_eq!(
            PythonMinor::from_profile_name(&minor.profile_name()),
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
        declaration.validate_target(Some(&minor.target())).unwrap();
        for other in PythonMinor::ALL {
            declaration.validate_target(Some(&other.target())).unwrap();
        }
    }
}

#[test]
fn bun_build_declarations_admit_canonical_versions_without_a_support_policy() {
    let declaration: ApplicationRuntimeDeclaration =
        serde_json::from_value(json!({"family":"BUN","args":[]})).unwrap();
    for target in ["bun-0.8.0", "bun-1.4.2", "bun-1.0.0", "bun-2.0.0"] {
        declaration.validate_target(Some(target)).unwrap();
    }
    for target in [
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

#[test]
fn python_declarations_accept_canonical_minors_outside_the_toolchain_catalog() {
    for version in ["2.7", "3.11", "3.15", "4.0", "123456789.987654321"] {
        let minor = PythonMinor::from_version(version).unwrap();
        assert_eq!(minor.version(), version);
        assert_eq!(PythonMinor::from_target(&minor.target()), Some(minor));
        assert_eq!(
            PythonMinor::from_profile_name(&minor.profile_name()),
            Some(minor)
        );
        assert_eq!(serde_json::to_value(minor).unwrap(), json!(version));
        assert_eq!(
            serde_json::from_value::<PythonMinor>(json!(version)).unwrap(),
            minor
        );
        let dependency = format!(".onreza/python/{version}/site-packages/demo.py");
        assert_eq!(PythonMinor::for_dependency_path(&dependency), Some(minor));
        assert!(
            PythonMinor::for_dependency_path(&format!(
                ".onreza/python/{version}/site-packages-other/demo.py"
            ))
            .is_none()
        );
        let declaration: ApplicationRuntimeDeclaration =
            serde_json::from_value(json!({"family":"PYTHON","pythonVersion":version,"args":[]}))
                .unwrap();
        declaration.validate_target(Some("python-3.14")).unwrap();
    }
    for version in [
        "3",
        "3.11.1",
        "03.11",
        "3.011",
        "+3.11",
        "3.-11",
        "3.11 ",
        "3.1000000000",
    ] {
        assert!(PythonMinor::from_version(version).is_none(), "{version}");
        assert!(
            serde_json::from_value::<PythonMinor>(json!(version)).is_err(),
            "{version}"
        );
    }
}

#[test]
fn python_declaration_hints_are_bounded_metadata_independent_of_actual_targets() {
    for hint in ["2.7", "3.11", "3.14.9", "4.x", "  user supplied text  "] {
        let value = json!({"family":"PYTHON", "pythonVersion":hint, "args":[]});
        let declaration: ApplicationRuntimeDeclaration =
            serde_json::from_value(value.clone()).unwrap();
        declaration.validate_target(Some("python-3.14")).unwrap();
        assert_eq!(serde_json::to_value(declaration).unwrap(), value);
    }
    for hint in [
        String::new(),
        "x".repeat(129),
        "я".repeat(65),
        "3.14\0".into(),
    ] {
        let value = json!({"family":"PYTHON", "pythonVersion":hint, "args":[]});
        assert!(serde_json::from_value::<ApplicationRuntimeDeclaration>(value).is_err());
    }
    let declaration: ApplicationRuntimeDeclaration = serde_json::from_value(
        json!({"family":"PYTHON", "pythonVersion":"я".repeat(64), "args":[]}),
    )
    .unwrap();
    declaration.validate().unwrap();
}
