use super::apply_application_runtime_manifest;
use crate::build::manifest::{LayerTarget, Manifest};
use crate::detect::types::{RuntimeInfo, RuntimeType};

fn authored_server(family: &str, static_first: bool) -> Manifest {
    let compute = serde_json::json!({
        "name": "server", "target": "COMPUTE", "directory": ".", "entry": "server.js",
        "runtime": {"applicationRuntime": {"family": family, "args": ["$(id)", "two words"]}}
    });
    let assets = serde_json::json!({"name": "assets", "target": "STATIC", "directory": "public"});
    let layers = if static_first {
        vec![assets, compute]
    } else {
        vec![compute, assets]
    };
    serde_json::from_value(serde_json::json!({"version": 1, "layers": layers, "routes": []}))
        .unwrap()
}

fn authored_server_with_target(family: &str, static_first: bool, target: &str) -> Manifest {
    let mut manifest = authored_server(family, static_first);
    manifest
        .layers
        .iter_mut()
        .find(|layer| layer.target == LayerTarget::Compute)
        .unwrap()
        .runtime
        .as_mut()
        .unwrap()
        .build_runtime_version = Some(target.into());
    manifest
}

fn selected_primary_cases() -> [(
    nrz_source_bundle::ApplicationRuntimeFamily,
    &'static str,
    &'static str,
); 4] {
    use nrz_source_bundle::ApplicationRuntimeFamily;
    [
        (ApplicationRuntimeFamily::Node, "node-24", "selected.js"),
        (ApplicationRuntimeFamily::Bun, "bun-1.3.11", "selected.ts"),
        (
            ApplicationRuntimeFamily::Python,
            "python-3.14",
            ".onreza/python/launch.py",
        ),
        (
            ApplicationRuntimeFamily::Executable,
            "native-linux-x86_64-glibc",
            "bin/selected",
        ),
    ]
}

#[test]
fn sole_compute_runtime_selection_ignores_static_layer_position() {
    let runtime = RuntimeInfo {
        runtime_type: RuntimeType::Node,
        version: None,
    };
    for static_first in [false, true] {
        for family in ["NODE", "BUN"] {
            let manifest = authored_server(family, static_first);
            let target = crate::deploy::canonical_build_runtime_target(
                &runtime,
                false,
                Some(&manifest),
                None,
                Some("NODE_24"),
                true,
            )
            .unwrap()
            .unwrap();
            if family == "NODE" {
                assert_eq!(target, "node-24");
            } else {
                assert!(target.starts_with("bun-"), "{target}");
            }
        }
    }
}

#[test]
fn sole_compute_manifest_freezes_admitted_target_with_static_assets() {
    for static_first in [false, true] {
        let mut manifest = authored_server("NODE", static_first);
        let assets = manifest
            .layers
            .iter()
            .find(|layer| layer.target == LayerTarget::Static)
            .unwrap()
            .clone();
        apply_application_runtime_manifest(&mut manifest, None, Some("node-24"), "other").unwrap();
        let result = serde_json::to_value(&manifest).unwrap();
        let server = result["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|layer| layer["name"] == "server")
            .unwrap();
        assert_eq!(server["runtime"]["buildRuntimeVersion"], "node-24");
        assert_eq!(
            server["runtime"]["applicationRuntime"],
            serde_json::json!({"family": "NODE", "args": ["$(id)", "two words"]})
        );
        assert_eq!(
            serde_json::to_value(
                manifest
                    .layers
                    .iter()
                    .find(|layer| layer.target == LayerTarget::Static)
                    .unwrap()
            )
            .unwrap(),
            serde_json::to_value(&assets).unwrap()
        );
    }
}

#[test]
fn unresolved_sole_node_manifest_keeps_intent_with_static_assets() {
    for static_first in [false, true] {
        let mut manifest = authored_server("NODE", static_first);
        let before = serde_json::to_value(&manifest).unwrap();
        apply_application_runtime_manifest(&mut manifest, None, None, "other").unwrap();
        assert_eq!(serde_json::to_value(manifest).unwrap(), before);
    }
}

#[test]
fn static_assets_do_not_allow_replacing_an_explicit_primary_target() {
    for static_first in [false, true] {
        let mut manifest = authored_server("NODE", static_first);
        manifest
            .layers
            .iter_mut()
            .find(|layer| layer.target == LayerTarget::Compute)
            .unwrap()
            .runtime
            .as_mut()
            .unwrap()
            .build_runtime_version = Some("node-22".into());
        let before = serde_json::to_value(&manifest).unwrap();
        assert!(
            apply_application_runtime_manifest(&mut manifest, None, Some("node-24"), "other")
                .is_err()
        );
        assert_eq!(serde_json::to_value(manifest).unwrap(), before);
    }
}

#[test]
fn static_assets_do_not_supply_targets_to_multiple_compute_owners() {
    for static_first in [false, true] {
        let mut manifest = authored_server("NODE", static_first);
        manifest.layers.push(serde_json::from_value(serde_json::json!({
            "name": "python", "target": "COMPUTE", "directory": "python", "entry": "server.py",
            "runtime": {"applicationRuntime": {"family": "PYTHON", "args": []}, "buildRuntimeVersion": "python-3.12"}
        })).unwrap());
        let before = serde_json::to_value(&manifest).unwrap();
        assert!(
            apply_application_runtime_manifest(&mut manifest, None, Some("node-24"), "other")
                .is_err()
        );
        assert_eq!(serde_json::to_value(&manifest).unwrap(), before);

        manifest
            .layers
            .iter_mut()
            .find(|layer| layer.name == "server")
            .unwrap()
            .runtime
            .as_mut()
            .unwrap()
            .build_runtime_version = Some("node-22".into());
        let before = serde_json::to_value(&manifest).unwrap();
        apply_application_runtime_manifest(&mut manifest, None, Some("node-24"), "other").unwrap();
        assert_eq!(serde_json::to_value(manifest).unwrap(), before);
    }
}

#[test]
fn static_primary_keeps_a_typed_compute_owners_frozen_target() {
    for (family, target) in [
        ("NODE", "node-22"),
        ("PYTHON", "python-3.12"),
        ("EXECUTABLE", "native-linux-x86_64-glibc"),
    ] {
        for static_first in [false, true] {
            let mut manifest = authored_server(family, static_first);
            manifest
                .layers
                .iter_mut()
                .find(|layer| layer.target == LayerTarget::Compute)
                .unwrap()
                .runtime
                .as_mut()
                .unwrap()
                .build_runtime_version = Some(target.into());
            let before = serde_json::to_value(&manifest).unwrap();
            let runtime = RuntimeInfo {
                runtime_type: RuntimeType::Static,
                version: None,
            };
            assert!(
                crate::deploy::canonical_build_runtime_target(
                    &runtime,
                    false,
                    Some(&manifest),
                    None,
                    Some("NODE_24"),
                    true
                )
                .unwrap()
                .is_none()
            );
            apply_application_runtime_manifest(&mut manifest, None, None, "hugo").unwrap();
            assert_eq!(serde_json::to_value(manifest).unwrap(), before);
        }
    }
}

#[test]
fn a_same_family_sibling_keeps_its_own_frozen_launch_arguments() {
    let mut manifest = authored_server_with_target("NODE", true, "node-24");
    manifest.layers.push(serde_json::from_value(serde_json::json!({
        "name": "api", "target": "COMPUTE", "directory": "api", "entry": "server.js",
        "runtime": {"applicationRuntime": {"family": "NODE", "args": ["--api"]}, "buildRuntimeVersion": "node-24"}
    })).unwrap());
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: Some("server.js".into()),
        args: vec!["$(id)".into(), "two words".into()],
    };
    let before = serde_json::to_value(&manifest).unwrap();
    apply_application_runtime_manifest(&mut manifest, Some(&declaration), Some("node-24"), "other")
        .unwrap();
    assert_eq!(serde_json::to_value(manifest).unwrap(), before);
}

#[test]
fn identified_primary_launch_arguments_cannot_override_the_declaration() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: Some("server.js".into()),
        args: vec!["--primary".into()],
    };
    let mut manifest = authored_server("NODE", true);
    manifest
        .layers
        .retain(|layer| layer.target == LayerTarget::Compute);
    let before = serde_json::to_value(&manifest).unwrap();
    assert!(
        apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other"
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(manifest).unwrap(), before);
}

#[test]
fn typed_same_family_sibling_launch_requires_its_own_target() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: Some("server.js".into()),
        args: vec!["$(id)".into(), "two words".into()],
    };
    let mut manifest = authored_server_with_target("NODE", true, "node-24");
    manifest.layers.push(
        serde_json::from_value(serde_json::json!({
            "name": "api", "target": "COMPUTE", "directory": "api", "entry": "server.js",
            "runtime": {"applicationRuntime": {"family": "NODE", "args": ["--api"]}}
        }))
        .unwrap(),
    );
    let before = serde_json::to_value(&manifest).unwrap();
    assert!(
        apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other"
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(manifest).unwrap(), before);
}

#[test]
fn a_primary_in_a_subdirectory_must_match_its_declared_launch() {
    for (target, args) in [
        ("node-24", vec!["--wrong"]),
        ("node-22", vec!["$(id)", "two words"]),
    ] {
        let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
            family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
            python_version: None,
            entry: Some("api/server.js".into()),
            args: vec!["$(id)".into(), "two words".into()],
        };
        let mut manifest = authored_server("NODE", true);
        let server = manifest
            .layers
            .iter_mut()
            .find(|layer| layer.target == LayerTarget::Compute)
            .unwrap();
        server.directory = "api".into();
        let runtime = server.runtime.as_mut().unwrap();
        runtime.build_runtime_version = Some(target.into());
        runtime.application_runtime.as_mut().unwrap().args =
            args.into_iter().map(str::to_owned).collect();
        let before = serde_json::to_value(&manifest).unwrap();
        assert!(
            apply_application_runtime_manifest(
                &mut manifest,
                Some(&declaration),
                Some("node-24"),
                "other"
            )
            .is_err(),
            "primary in api was exempted for {target}"
        );
        assert_eq!(serde_json::to_value(manifest).unwrap(), before);
    }
}

#[test]
fn identical_intent_typed_siblings_cannot_inherit_the_primary_target() {
    let mut accepted = Vec::new();
    for primary_entry in [Some("server.js"), None] {
        let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
            family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
            python_version: None,
            entry: primary_entry.map(str::to_owned),
            args: vec!["$(id)".into(), "two words".into()],
        };
        let mut manifest = authored_server_with_target("NODE", true, "node-24");
        manifest.layers.push(
            serde_json::from_value(serde_json::json!({
                "name": "api", "target": "COMPUTE", "directory": "api", "entry": "index.js",
                "runtime": {"applicationRuntime": {"family": "NODE", "args": declaration.args}}
            }))
            .unwrap(),
        );
        let before = serde_json::to_value(&manifest).unwrap();
        if apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other",
        )
        .is_ok()
        {
            accepted.push(primary_entry);
        } else {
            assert_eq!(serde_json::to_value(&manifest).unwrap(), before);
        }
        manifest
            .layers
            .iter_mut()
            .find(|layer| layer.name == "api")
            .unwrap()
            .runtime
            .as_mut()
            .unwrap()
            .build_runtime_version = Some("node-22".into());
        let before = serde_json::to_value(&manifest).unwrap();
        apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other",
        )
        .unwrap();
        assert_eq!(serde_json::to_value(manifest).unwrap(), before);
    }
    assert!(
        accepted.is_empty(),
        "typed siblings without targets were accepted with primary entry {accepted:?}"
    );
}

#[test]
fn static_serving_intent_retains_frozen_targets_without_static_output_files() {
    let runtime = RuntimeInfo {
        runtime_type: RuntimeType::Static,
        version: None,
    };
    let mut changed = Vec::new();
    for (family, target) in [
        ("NODE", "node-22"),
        ("PYTHON", "python-3.12"),
        ("EXECUTABLE", "native-linux-x86_64-glibc"),
    ] {
        for with_assets in [false, true] {
            let mut manifest = authored_server(family, true);
            if !with_assets {
                manifest
                    .layers
                    .retain(|layer| layer.target == LayerTarget::Compute);
            }
            manifest
                .layers
                .iter_mut()
                .find(|layer| layer.target == LayerTarget::Compute)
                .unwrap()
                .runtime
                .as_mut()
                .unwrap()
                .build_runtime_version = Some(target.into());
            let before = serde_json::to_value(&manifest).unwrap();
            let selected = crate::deploy::canonical_build_runtime_target(
                &runtime,
                false,
                Some(&manifest),
                None,
                Some("NODE_24"),
                true,
            )
            .unwrap();
            if selected.is_some() {
                changed.push(format!(
                    "{family}/assets={with_assets}/selector={selected:?}"
                ));
            }
            if apply_application_runtime_manifest(&mut manifest, None, selected.as_deref(), "hugo")
                .is_err()
            {
                changed.push(format!("{family}/assets={with_assets}/apply rejected"));
            }
            assert_eq!(serde_json::to_value(manifest).unwrap(), before);
        }
    }
    assert!(
        changed.is_empty(),
        "STATIC metadata owner depends on physical assets: {changed:?}"
    );
}

#[test]
fn untyped_javascript_siblings_keep_the_legacy_shared_target_default() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: Some("server.js".into()),
        args: vec![],
    };
    let mut manifest: Manifest = serde_json::from_value(serde_json::json!({
        "version": 1, "routes": [], "layers": [
            {"name": "main", "target": "COMPUTE", "directory": ".", "entry": "server.js"},
            {"name": "api", "target": "COMPUTE", "directory": "api", "entry": "index.js"}
        ]
    }))
    .unwrap();
    apply_application_runtime_manifest(&mut manifest, Some(&declaration), Some("node-24"), "other")
        .unwrap();
    for layer in manifest.layers {
        let runtime = layer.runtime.unwrap();
        assert_eq!(runtime.application_runtime, Some(declaration.intent()));
        assert_eq!(runtime.build_runtime_version.as_deref(), Some("node-24"));
    }
}

#[test]
fn selected_javascript_primary_rejects_admitted_target_mismatch_with_either_layout() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: Some("server.js".into()),
        args: vec!["$(id)".into(), "two words".into()],
    };
    for runtime_type in [RuntimeType::Node, RuntimeType::Bun] {
        for with_assets in [false, true] {
            let runtime = RuntimeInfo {
                runtime_type,
                version: None,
            };
            let mut manifest = authored_server("NODE", true);
            if !with_assets {
                manifest
                    .layers
                    .retain(|layer| layer.target == LayerTarget::Compute);
            }
            manifest
                .layers
                .iter_mut()
                .find(|layer| layer.target == LayerTarget::Compute)
                .unwrap()
                .runtime
                .as_mut()
                .unwrap()
                .build_runtime_version = Some("node-22".into());
            let before = serde_json::to_value(&manifest).unwrap();
            let selected = crate::deploy::canonical_build_runtime_target(
                &runtime,
                false,
                Some(&manifest),
                Some(&declaration),
                Some("NODE_24"),
                true,
            )
            .unwrap();
            assert_eq!(selected.as_deref(), Some("node-24"));
            assert!(
                apply_application_runtime_manifest(
                    &mut manifest,
                    Some(&declaration),
                    selected.as_deref(),
                    "other"
                )
                .is_err()
            );
            assert_eq!(serde_json::to_value(manifest).unwrap(), before);
        }
    }
}

#[tokio::test]
async fn selected_node_primary_build_rejects_conflicting_authored_target() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("index.html"), "<html></html>").unwrap();
    std::fs::write(directory.path().join("server.js"), "console.log('hello')").unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_dirs = Some(vec![".".into()]);
    config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Node);
    config.deploy.entry = Some("server.js".into());
    std::fs::write(
        directory.path().join("onreza.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    let detection = crate::detect::detect_with_framework_override(directory.path(), None);
    assert_eq!(detection.metadata.runtime.runtime_type, RuntimeType::Static);
    let mut manifest = crate::build::manifest::generate_compute_manifest("server.js");
    manifest.layers[0].runtime = Some(crate::build::manifest::RuntimeConfig {
        application_runtime: Some(nrz_source_bundle::ApplicationRuntimeIntent {
            family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
            args: vec![],
        }),
        build_runtime_version: Some("node-22".into()),
        ..Default::default()
    });
    std::fs::create_dir(directory.path().join(".onreza")).unwrap();
    std::fs::write(
        directory.path().join(".onreza/manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
        directory.path().to_owned(),
        config,
    );
    effective.bind_admitted_node_version("NODE_24").unwrap();
    let error = crate::build::run_with_effective_config(
        crate::cli::BuildArgs {
            dir: directory.path().to_string_lossy().into_owned(),
            skip_validation: false,
        },
        true,
        &effective,
        Some(&detection),
        false,
        directory.path(),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<crate::output::CodedError>()
            .unwrap()
            .code
            .as_str(),
        "APPLICATION_RUNTIME_INVALID"
    );
    assert!(
        error
            .to_string()
            .contains("runtime version differs from validated pre-build runtime"),
        "{error:#}"
    );
}

#[test]
fn unknown_primary_identity_does_not_supply_a_typed_sibling_target() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: None,
        args: vec![],
    };
    let mut manifest: Manifest = serde_json::from_value(serde_json::json!({
        "version": 1, "routes": [], "layers": [
            {"name": "main", "target": "COMPUTE", "directory": ".", "entry": "server.js"},
            {"name": "api", "target": "COMPUTE", "directory": "api", "entry": "index.js",
                "runtime": {"applicationRuntime": {"family": "NODE", "args": []}}}
        ]
    }))
    .unwrap();
    assert!(
        apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other"
        )
        .is_err(),
        "ambiguous typed api owner inherited the parent's target"
    );
    assert!(
        manifest.layers[1]
            .runtime
            .as_ref()
            .unwrap()
            .build_runtime_version
            .is_none()
    );
}

#[test]
fn declaration_without_entry_requires_a_compatible_compute_primary() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: None,
        args: vec![],
    };
    for (family, target, args, selected_args) in [
        ("PYTHON", "python-3.14", vec![], vec![]),
        ("NODE", "node-22", vec![], vec![]),
        ("NODE", "node-24", vec!["--independent"], vec!["--selected"]),
    ] {
        let mut manifest: Manifest = serde_json::from_value(serde_json::json!({
            "version": 1, "routes": [], "layers": [
                {"name": "worker", "target": "COMPUTE", "directory": ".", "entry": "worker.py",
                 "runtime": {"applicationRuntime": {"family": family, "args": args},
                             "buildRuntimeVersion": target}}
            ]
        }))
        .unwrap();
        let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
            args: selected_args.into_iter().map(str::to_owned).collect(),
            ..declaration.clone()
        };
        let before = serde_json::to_value(&manifest).unwrap();
        let error = apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other",
        )
        .expect_err("independent worker replaced the selected primary runtime");
        assert!(error.to_string().contains("primary"), "{error:#}");
        assert_eq!(serde_json::to_value(manifest).unwrap(), before);
    }
}

#[test]
fn declaration_without_entry_keeps_compatible_owners_and_independent_siblings() {
    let declaration = nrz_source_bundle::ApplicationRuntimeDeclaration {
        family: nrz_source_bundle::ApplicationRuntimeFamily::Node,
        python_version: None,
        entry: None,
        args: vec![],
    };
    for compatible_count in [1, 2] {
        let mut layers = vec![serde_json::json!({
            "name": "worker", "target": "COMPUTE", "directory": "python", "entry": "worker.py",
            "runtime": {"applicationRuntime": {"family": "PYTHON", "args": []},
                        "buildRuntimeVersion": "python-3.14"}
        })];
        for index in 0..compatible_count {
            layers.push(serde_json::json!({
                "name": format!("primary-{index}"), "target": "COMPUTE", "directory": format!("node-{index}"), "entry": "server.js",
                "runtime": {"applicationRuntime": {"family": "NODE", "args": ["$(id)", "two words", format!("--owner-{index}")]},
                            "buildRuntimeVersion": "node-24"}
            }));
        }
        let mut manifest: Manifest = serde_json::from_value(serde_json::json!({
            "version": 1, "routes": [], "layers": layers
        }))
        .unwrap();
        let before = serde_json::to_value(&manifest).unwrap();
        apply_application_runtime_manifest(
            &mut manifest,
            Some(&declaration),
            Some("node-24"),
            "other",
        )
        .unwrap();
        assert_eq!(serde_json::to_value(manifest).unwrap(), before);
    }
}

#[test]
fn untyped_manifest_cannot_bind_a_different_frozen_primary_entry() {
    use nrz_source_bundle::{ApplicationRuntimeDeclaration, ApplicationRuntimeFamily, PythonMinor};
    for (family, target, entry) in selected_primary_cases() {
        let declaration = ApplicationRuntimeDeclaration {
            family,
            python_version: (family == ApplicationRuntimeFamily::Python)
                .then_some(PythonMinor::Python314),
            entry: Some(entry.into()),
            args: vec!["MODULE".into(), "selected".into()],
        };
        for directory in [".", "other"] {
            let mut manifest: Manifest = serde_json::from_value(serde_json::json!({
                "version": 1, "routes": [], "layers": [
                    {"name":"old", "target":"COMPUTE", "directory":directory, "entry":"old.py"}
                ]
            }))
            .unwrap();
            let before = serde_json::to_value(&manifest).unwrap();
            let error = apply_application_runtime_manifest(
                &mut manifest,
                Some(&declaration),
                Some(target),
                "other",
            )
            .unwrap_err();
            assert!(error.to_string().contains("entry"), "{error:#}");
            assert_eq!(serde_json::to_value(manifest).unwrap(), before);
        }
    }
}

#[test]
fn typed_siblings_cannot_replace_a_declared_primary_entry() {
    use nrz_source_bundle::{ApplicationRuntimeDeclaration, ApplicationRuntimeFamily, PythonMinor};
    for (family, target, entry) in selected_primary_cases() {
        let declaration = ApplicationRuntimeDeclaration {
            family,
            python_version: (family == ApplicationRuntimeFamily::Python)
                .then_some(PythonMinor::Python314),
            entry: Some(entry.into()),
            args: vec![],
        };
        for static_first in [false, true] {
            for directory in [".", "other"] {
                let sibling = serde_json::json!({
                    "name": "worker", "target": "COMPUTE", "directory": directory,
                    "entry": if directory == "." { "worker.js" } else { entry.rsplit('/').next().unwrap() }, "runtime": {
                        "applicationRuntime": {"family": "NODE", "args": []},
                        "buildRuntimeVersion": "node-22"
                    }
                });
                // A STATIC layer pointing at the selected file is not a serving owner.
                let assets = serde_json::json!({
                    "name": "assets", "target": "STATIC", "directory": entry
                });
                let layers = if static_first {
                    vec![assets, sibling]
                } else {
                    vec![sibling, assets]
                };
                let mut manifest: Manifest = serde_json::from_value(serde_json::json!({
                    "version": 1, "routes": [], "layers": layers
                }))
                .unwrap();
                let before = serde_json::to_value(&manifest).unwrap();
                let error = apply_application_runtime_manifest(
                    &mut manifest,
                    Some(&declaration),
                    Some(target),
                    "other",
                )
                .expect_err("typed worker silently replaced the selected primary");
                assert!(error.to_string().contains("entry"), "{error:#}");
                assert_eq!(serde_json::to_value(manifest).unwrap(), before);
            }
        }
    }
}

#[tokio::test]
async fn build_binds_only_the_selected_entry_in_the_output_coordinate_frame() {
    for (selected, manifest_entry, typed, accepted) in [
        ("server.js", "old.js", false, false),
        ("server.js", "server.js", false, true),
        ("nested/server.js", "server.js", false, false),
        ("server.js", "old.js", true, false),
        ("server.js", "server.js", true, true),
        ("nested/server.js", "server.js", true, false),
        ("server.js", "", false, false),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("dist");
        std::fs::create_dir_all(output.join(".onreza")).unwrap();
        std::fs::create_dir_all(output.join("nested")).unwrap();
        for entry in ["server.js", "old.js", "nested/server.js"] {
            std::fs::write(output.join(entry), "console.log('hello')").unwrap();
        }
        let mut manifest = if manifest_entry.is_empty() {
            crate::build::manifest::generate_static_manifest()
        } else {
            crate::build::manifest::generate_compute_manifest(manifest_entry)
        };
        if typed {
            manifest.layers[0].runtime = Some(
                serde_json::from_value(serde_json::json!({
                    "applicationRuntime": {"family": "NODE", "args": []},
                    "buildRuntimeVersion": "node-24"
                }))
                .unwrap(),
            );
        }
        std::fs::write(
            output.join(".onreza/manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let mut config = nrz::config::ProjectConfig::default();
        config.build.output_directory = Some("dist".into());
        config.deploy.runtime = Some(nrz_source_bundle::ApplicationRuntimeFamily::Node);
        config.deploy.entry = Some(selected.into());
        std::fs::write(
            directory.path().join("onreza.toml"),
            toml::to_string(&config).unwrap(),
        )
        .unwrap();
        let detection = crate::detect::detect_with_framework_override(directory.path(), None);
        let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
            directory.path().to_owned(),
            config,
        );
        effective.bind_admitted_node_version("NODE_24").unwrap();
        let result = crate::build::run_with_effective_config(
            crate::cli::BuildArgs {
                dir: directory.path().to_string_lossy().into_owned(),
                skip_validation: false,
            },
            true,
            &effective,
            Some(&detection),
            false,
            directory.path(),
            None,
        )
        .await;
        assert_eq!(
            result.is_ok(),
            accepted,
            "{selected} / {manifest_entry} (typed={typed}): {result:?}"
        );
    }
}

#[tokio::test]
async fn inferred_source_entry_uses_process_output_mapping_before_manifest_binding() {
    let project = tempfile::tempdir().unwrap();
    let output = project.path().join("dist");
    std::fs::create_dir_all(output.join(".onreza")).unwrap();
    std::fs::create_dir_all(output.join("dist")).unwrap();
    for entry in ["server.js", "dist/server.js"] {
        std::fs::write(output.join(entry), "console.log('hello')").unwrap();
    }
    std::fs::write(
        project.path().join("package.json"),
        r#"{"scripts":{"start":"node dist/server.js"}}"#,
    )
    .unwrap();
    std::fs::write(
        output.join(".onreza/manifest.json"),
        serde_json::to_vec(&crate::build::manifest::generate_compute_manifest(
            "server.js",
        ))
        .unwrap(),
    )
    .unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_directory = Some("dist".into());
    std::fs::write(
        project.path().join("onreza.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    let detection = crate::detect::detect_with_framework_override(project.path(), None);
    let mut effective =
        nrz::config::EffectiveProjectConfig::from_project_config(project.path().to_owned(), config);
    effective.bind_admitted_node_version("NODE_24").unwrap();
    let result = crate::build::run_with_effective_config(
        crate::cli::BuildArgs {
            dir: project.path().to_string_lossy().into_owned(),
            skip_validation: false,
        },
        true,
        &effective,
        Some(&detection),
        false,
        project.path(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        result.manifest.unwrap().layers[0].entry.as_deref(),
        Some("server.js")
    );
}
