use super::*;
use nrz_source_bundle::{BuildToolchainFamily, PythonMinor, SourceBuildContext};
use serde_json::json;

fn context(compiler: &str, application: &str) -> SourceBuildContext {
    serde_json::from_value(json!({
        "schemaVersion": 1,
        "buildToolchain": {"family": compiler},
        "applicationRuntime": {"family": application, "entry": "server.js", "args": []}
    }))
    .unwrap()
}

#[test]
fn admitted_targets_replace_requested_lines_without_mutating_source_intent() {
    let original = context("NODE", "BUN");
    let resolved = resolve_platform_runtime(&original, "node-26", "bun-1.4.3", Some("26")).unwrap();
    assert_eq!(resolved.application_target.as_deref(), Some("bun-1.4.3"));
    assert_eq!(resolved.node_version.as_deref(), Some("NODE_26"));
    assert_eq!(resolved.context, original);

    let mut original = context("PYTHON", "PYTHON");
    original.build_toolchain.python_version = Some(PythonMinor::Python312.version());
    original
        .application_runtime
        .as_mut()
        .unwrap()
        .python_version = Some(PythonMinor::Python312.version());
    let resolved = resolve_platform_runtime(&original, "python-3.14", "python-3.13", None).unwrap();
    assert_eq!(
        resolved.context.build_toolchain.python_version,
        Some(PythonMinor::Python314.version())
    );
    assert_eq!(
        resolved.context.application_runtime.unwrap().python_version,
        Some(PythonMinor::Python312.version())
    );
    assert_eq!(
        original.build_toolchain.python_version,
        Some(PythonMinor::Python312.version())
    );
    assert_eq!(
        original.application_runtime.unwrap().python_version,
        Some(PythonMinor::Python312.version())
    );
}

#[test]
fn policy_resolution_keeps_compiler_and_application_families_independent_and_frozen() {
    let original = context("BUN", "NODE");
    let resolved = resolve_platform_runtime(&original, "bun-1.4.3", "node-26", Some("24")).unwrap();
    assert_eq!(
        resolved.context.build_toolchain.family,
        BuildToolchainFamily::Bun
    );
    assert_eq!(resolved.node_version.as_deref(), Some("NODE_26"));
    assert!(resolve_platform_runtime(&original, "node-26", "node-26", Some("26")).is_err());
    assert!(resolve_platform_runtime(&original, "bun-1.4.3", "python-3.14", Some("26")).is_err());
}

#[test]
fn platform_bun_application_uses_admitted_target_with_node_compiler_metadata() {
    let original = context("NODE", "BUN");
    let target = canonical_build_runtime_target(
        &RuntimeInfo {
            runtime_type: RuntimeType::Node,
            version: None,
        },
        true,
        Some("bun-1.4.3"),
        None,
        original.application_runtime.as_ref(),
        Some("NODE_26"),
        false,
    )
    .unwrap();
    assert_eq!(target.as_deref(), Some("bun-1.4.3"));
}

#[cfg(unix)]
#[tokio::test]
async fn local_bun_uses_measured_version_and_platform_probe_enforces_admission() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("bun");
    std::fs::write(&executable, "#!/bin/sh\nprintf '1.4.3\\n'\n").unwrap();
    std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let environment = vec![("PATH".to_string(), directory.path().display().to_string())];
    assert_eq!(
        measured_bun_target(&environment).await.unwrap(),
        "bun-1.4.3"
    );
    assert!(
        qualify_javascript_binary("bun", Some("bun-1.4.2"), &environment)
            .await
            .is_err()
    );
    assert!(
        qualify_javascript_binary("bun", Some("bun-1.4.3"), &environment)
            .await
            .is_ok()
    );
}

#[test]
fn fresh_local_bun_uses_measured_target_while_retained_claim_stays_immutable() {
    let original = context("BUN", "BUN");
    let mut manifest = crate::build::manifest::generate_compute_manifest("server.js");
    manifest.layers[0].runtime = Some(crate::build::manifest::RuntimeConfig {
        network: Some(nrz_source_bundle::ComputeNetwork::public_default()),
        application_runtime: Some(original.application_runtime.as_ref().unwrap().intent()),
        build_runtime_version: Some("bun-1.4.2".into()),
        ..Default::default()
    });
    let runtime = RuntimeInfo {
        runtime_type: RuntimeType::Bun,
        version: None,
    };
    let retained = canonical_build_runtime_target(
        &runtime,
        false,
        None,
        Some(&manifest),
        original.application_runtime.as_ref(),
        None,
        false,
    )
    .unwrap();
    assert_eq!(retained.as_deref(), Some("bun-1.4.2"));
    let before = serde_json::to_value(&manifest).unwrap();
    crate::deploy::apply_application_runtime_manifest(
        &mut manifest,
        original.application_runtime.as_ref(),
        retained.as_deref(),
        "other",
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&manifest).unwrap(), before);
    let mut siblings: crate::build::manifest::Manifest =
        serde_json::from_value(before.clone()).unwrap();
    let mut second = siblings.layers[0].clone();
    second.name = "second".into();
    second.entry = Some("second.js".into());
    second.runtime.as_mut().unwrap().build_runtime_version = Some("bun-1.4.1".into());
    siblings.layers.push(second);
    let mut runtime_only = original.application_runtime.as_ref().unwrap().clone();
    runtime_only.entry = None;
    let selected = canonical_build_runtime_target(
        &runtime,
        false,
        None,
        Some(&siblings),
        Some(&runtime_only),
        None,
        false,
    )
    .unwrap();
    assert_eq!(selected, None);
    let before = serde_json::to_value(&siblings).unwrap();
    crate::deploy::apply_application_runtime_manifest(
        &mut siblings,
        Some(&runtime_only),
        selected.as_deref(),
        "other",
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&siblings).unwrap(), before);
    let mut untyped_siblings: crate::build::manifest::Manifest =
        serde_json::from_value(before).unwrap();
    for layer in &mut untyped_siblings.layers {
        layer.runtime.as_mut().unwrap().application_runtime = None;
    }
    let selected = canonical_build_runtime_target(
        &runtime,
        true,
        None,
        Some(&untyped_siblings),
        None,
        None,
        false,
    )
    .unwrap();
    assert_eq!(selected, None);
    let before = serde_json::to_value(&untyped_siblings).unwrap();
    crate::deploy::apply_application_runtime_manifest(
        &mut untyped_siblings,
        None,
        selected.as_deref(),
        "other",
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&untyped_siblings).unwrap(), before);
    let fresh = canonical_build_runtime_target(
        &runtime,
        false,
        Some("bun-1.4.3"),
        Some(&manifest),
        original.application_runtime.as_ref(),
        None,
        false,
    )
    .unwrap();
    assert_eq!(fresh.as_deref(), Some("bun-1.4.3"));
    crate::deploy::apply_rebuilt_application_runtime_manifest(
        &mut manifest,
        original.application_runtime.as_ref(),
        fresh.as_deref(),
        "other",
    )
    .unwrap();
    assert_eq!(
        manifest.layers[0]
            .runtime
            .as_ref()
            .unwrap()
            .build_runtime_version
            .as_deref(),
        Some("bun-1.4.3")
    );
    let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
        std::path::PathBuf::from("."),
        Default::default(),
    );
    effective.mark_primary_runtime_rebuilt();
    assert!(effective.primary_runtime_rebuilt());
    assert!(effective.platform_source_build_context().is_none());
}

#[tokio::test]
async fn local_output_binding_replaces_only_fresh_primary_claims() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join(".onreza")).unwrap();
    std::fs::write(directory.path().join("server.js"), "export default 1").unwrap();
    std::fs::write(directory.path().join(".onreza/manifest.json"), serde_json::to_vec(&json!({
        "version":2, "routes":[{"pattern":"^/.*$","layer":"server"}], "layers":[
            {"name":"server", "target":"COMPUTE", "directory":".", "entry":"server.js",
             "runtime":{"network":{"schemaVersion":"COMPUTE_NETWORK_V1","exposure":"PUBLIC","ports":[{"name":"http","protocol":"http","port":3000,"binding":"primary"}],"readiness":{"protocol":"tcp","port":"http"},"allowFrom":[]},"applicationRuntime":{"family":"BUN","args":[]},"buildRuntimeVersion":"bun-1.4.2"}}
        ]
    })).unwrap()).unwrap();
    let original = context("BUN", "BUN");
    let mut config = nrz::config::ProjectConfig::default();
    config.build.output_dirs = Some(vec![".".into()]);
    let mut detection = crate::detect::detect_with_framework_override(directory.path(), None);
    crate::detect::application_runtime::bind_source_build_context(
        &crate::detect::fs::LocalFs::new(directory.path()),
        &mut detection,
        &original,
        &config,
        None,
        None,
    )
    .unwrap();
    for rebuilt in [false, true] {
        let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
            directory.path().to_owned(),
            config.clone(),
        );
        effective.bind_application_runtime_target(Some("bun-1.4.3".into()));
        if rebuilt {
            effective.mark_primary_runtime_rebuilt();
        }
        let manifest = bind_output(directory.path(), &effective, &detection).await;
        assert_eq!(
            manifest.layers[0]
                .runtime
                .as_ref()
                .unwrap()
                .build_runtime_version
                .as_deref(),
            Some(if rebuilt { "bun-1.4.3" } else { "bun-1.4.2" })
        );
    }
}

#[tokio::test]
async fn platform_retained_output_preserves_claims_after_policy_successor_admission() {
    for (family, old_target, new_target, output, retained_claim) in [
        ("BUN", "bun-1.4.2", "bun-1.4.3"),
        ("NODE", "node-24", "node-26"),
        ("PYTHON", "python-3.12", "python-3.14"),
    ]
    .into_iter()
    .flat_map(|(family, old_target, new_target)| {
        [".", "dist"].into_iter().flat_map(move |output| {
            [true, false]
                .into_iter()
                .filter(move |claim| *claim || family != "BUN")
                .map(move |claim| (family, old_target, new_target, output, claim))
        })
    }) {
        let directory = tempfile::tempdir().unwrap();
        let output_dir = directory.path().join(output);
        std::fs::create_dir_all(output_dir.join(".onreza")).unwrap();
        std::fs::write(output_dir.join("server.js"), "export default 1").unwrap();
        let mut authored = json!({
            "version":2, "routes":[{"pattern":"^/.*$","layer":"server"}], "layers":[
                {"name":"server", "target":"COMPUTE", "directory":".", "entry":"server.js",
                 "runtime":{"network":{"schemaVersion":"COMPUTE_NETWORK_V1","exposure":"PUBLIC","ports":[{"name":"http","protocol":"http","port":3000,"binding":"primary"}],"readiness":{"protocol":"tcp","port":"http"},"allowFrom":[]},"applicationRuntime":{"family":family,"args":[]},"buildRuntimeVersion":old_target}}
            ]
        });
        if !retained_claim {
            authored["layers"][0]["runtime"]
                .as_object_mut()
                .unwrap()
                .remove("buildRuntimeVersion");
        }
        std::fs::write(
            output_dir.join(".onreza/manifest.json"),
            serde_json::to_vec(&authored).unwrap(),
        )
        .unwrap();
        let mut original = context("NODE", family);
        original.application_runtime.as_mut().unwrap().entry = Some(
            if output == "." {
                "server.js"
            } else {
                "dist/server.js"
            }
            .into(),
        );
        original
            .application_runtime
            .as_mut()
            .unwrap()
            .python_version = PythonMinor::from_target(old_target).map(PythonMinor::version);
        let resolved =
            resolve_platform_runtime(&original, "node-26", new_target, Some("26")).unwrap();
        let mut config = nrz::config::ProjectConfig::default();
        config.build.output_dirs = Some(vec![output.into()]);
        let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
            directory.path().to_owned(),
            config,
        );
        effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
            source_build_context: Some(original.clone()),
            node_version: Some("NODE_24".into()),
            output_directory: Some(output.into()),
            output_directory_source: Some(nrz::config::BuildSettingSource::User),
            ..Default::default()
        });
        effective.bind_resolved_platform_context(
            resolved.context.clone(),
            resolved.application_target,
            resolved.node_version,
        );
        let mut detection = crate::detect::detect_with_framework_override(directory.path(), None);
        crate::detect::application_runtime::bind_source_build_context(
            &crate::detect::fs::LocalFs::new(directory.path()),
            &mut detection,
            &resolved.context,
            effective.config(),
            None,
            None,
        )
        .unwrap();
        let manifest = bind_output(directory.path(), &effective, &detection).await;
        assert_eq!(
            manifest.layers[0]
                .runtime
                .as_ref()
                .unwrap()
                .build_runtime_version
                .as_deref(),
            Some(if retained_claim {
                old_target
            } else if family == "NODE" {
                "node"
            } else {
                "python"
            })
        );
        assert_eq!(effective.requested_source_build_context(), Some(&original));
        assert_eq!(effective.requested_node_version(), Some("NODE_24"));
        assert!(!effective.primary_runtime_rebuilt());
    }
}

async fn bind_output(
    directory: &std::path::Path,
    effective: &nrz::config::EffectiveProjectConfig,
    detection: &crate::detect::types::DetectionResult,
) -> crate::build::manifest::Manifest {
    crate::build::run_with_effective_config(
        crate::cli::BuildArgs {
            dir: directory.display().to_string(),
            skip_validation: false,
        },
        true,
        effective,
        Some(detection),
        false,
        directory,
        None,
    )
    .await
    .unwrap()
    .manifest
    .unwrap()
}

#[test]
fn local_python_context_records_actual_compiler_independently_of_declared_minors() {
    for hint in ["2.7", "3.11", "3.14.9", "4.x"] {
        let original: SourceBuildContext = serde_json::from_value(json!({
            "schemaVersion": 1,
            "buildToolchain": {"family": "PYTHON", "pythonVersion": hint},
            "applicationRuntime": {"family": "PYTHON", "pythonVersion": hint, "args": []}
        }))
        .unwrap();
        let mut resolved = ResolvedBuildRuntime {
            context: original.clone(),
            application_target: None,
            node_version: None,
            python_interpreter: None,
        };
        let path = std::path::PathBuf::from("/available/python3");
        bind_python_compiler(
            &mut resolved,
            crate::deploy::python_toolchain::PythonInterpreter {
                path: path.clone(),
                minor: PythonMinor::Python314,
                version: "3.14.10".into(),
            },
        );
        assert_eq!(
            resolved.context.build_toolchain.python_version,
            Some(PythonMinor::Python314.version())
        );
        assert_eq!(
            resolved.context.application_runtime.unwrap().python_version,
            Some(hint.to_owned())
        );
        assert_eq!(resolved.python_interpreter, Some(path));
        assert_eq!(original.build_toolchain.python_version.unwrap(), hint);
        assert_eq!(
            original
                .application_runtime
                .unwrap()
                .python_version
                .unwrap(),
            hint
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn local_node_version_outside_the_catalog_is_observed_without_admission_filtering() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("node");
    std::fs::write(&executable, "#!/bin/sh\nprintf 'v23.1.0\\n'\n").unwrap();
    std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let environment = vec![("PATH".to_string(), directory.path().display().to_string())];
    assert_eq!(
        qualify_javascript_binary("node", None, &environment)
            .await
            .unwrap(),
        "v23.1.0"
    );
    assert_eq!(node_build_target("NODE_23").unwrap(), "node-23");
    for target in ["node-23", "node-23.1.0"] {
        assert_eq!(
            qualify_javascript_binary("node", Some(target), &environment)
                .await
                .unwrap(),
            "v23.1.0"
        );
    }
    for target in ["node-24", "node-24.1.1", "node"] {
        assert_eq!(
            qualify_javascript_binary("node", Some(target), &environment)
                .await
                .unwrap(),
            "v23.1.0"
        );
    }
    let resolved =
        resolve_platform_runtime(&context("NODE", "NODE"), "node-23.1.0", "node-23.1.0", None)
            .unwrap();
    assert_eq!(resolved.node_version.as_deref(), Some("NODE_23"));
    assert_eq!(resolved.application_target.as_deref(), Some("node-23.1.0"));
}

#[tokio::test]
async fn code_only_unknown_build_facts_keep_the_runtime_family_without_reading_hints() {
    for (family, target) in [
        (ApplicationRuntimeFamily::Node, "node"),
        (ApplicationRuntimeFamily::Python, "python"),
    ] {
        for hint in ["2.7", "3.11", "3.14.9", "4.x"] {
            let declaration = ApplicationRuntimeDeclaration {
                family,
                python_version: (family == ApplicationRuntimeFamily::Python)
                    .then(|| hint.to_owned()),
                entry: Some("server.js".into()),
                args: vec![],
            };
            let actual = canonical_build_runtime_target(
                &RuntimeInfo {
                    runtime_type: if family == ApplicationRuntimeFamily::Node {
                        RuntimeType::Node
                    } else {
                        RuntimeType::Python
                    },
                    version: Some(hint.into()),
                },
                false,
                None,
                None,
                Some(&declaration),
                Some(hint),
                true,
            )
            .unwrap();
            assert_eq!(actual.as_deref(), Some(target));
        }
    }

    for hint in ["2.7", "4.x"] {
        for dependencies in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            std::fs::write(directory.path().join("main.py"), "print('ready')\n").unwrap();
            let root = ".onreza/python/3.11/site-packages";
            if dependencies {
                std::fs::create_dir_all(directory.path().join(root)).unwrap();
                std::fs::write(directory.path().join(root).join("owned.py"), "VALUE = 1\n")
                    .unwrap();
            }
            let mut config = nrz::config::ProjectConfig::default();
            config.project.framework = Some("python".into());
            config.build.python_version = Some(hint.into());
            config.build.output_directory = Some(".".into());
            config.deploy.runtime = Some(ApplicationRuntimeFamily::Python);
            config.deploy.python_version = Some(hint.into());
            config.deploy.entry = Some("main.py".into());
            crate::deploy::test_support::write_project_config(directory.path(), &config);
            let (command, args) = crate::deploy::test_support::deploy_context(
                directory.path(),
                &config,
                &["--skip-build", "--skip-install"],
            );
            let plan = crate::deploy::test_support::build_plan(&args, &command, &[])
                .await
                .unwrap();
            assert_eq!(
                plan.artifact.runtime.manifest.layers[0]
                    .runtime
                    .as_ref()
                    .unwrap()
                    .build_runtime_version
                    .as_deref(),
                Some("python")
            );
            let source = plan.materialize_source_bundle(true, crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization).unwrap();
            assert_eq!(
                source
                    .logical_manifest
                    .files
                    .iter()
                    .any(|file| file.path == format!("{root}/owned.py")),
                dependencies
            );
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn python_probe_preserves_full_patch_witness_without_a_version_resolver() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("python");
    std::fs::write(&executable, "#!/bin/sh\nprintf '3.14.10\\n'\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let observed = crate::deploy::python_toolchain::probe_interpreter(&executable)
        .await
        .unwrap();
    assert_eq!(observed.version, "3.14.10");
    assert_eq!(observed.minor, PythonMinor::Python314);
}

#[cfg(unix)]
#[tokio::test]
async fn selected_node_declaration_does_not_reject_available_newer_or_older_compiler() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("node");
    let environment = vec![("PATH".into(), root.path().display().to_string())];
    for actual in ["v26.0.0", "v22.14.1"] {
        std::fs::write(
            &executable,
            format!("#!/bin/sh\nprintf '%s\n' '{actual}'\n"),
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            qualify_javascript_binary("node", Some("node-24"), &environment)
                .await
                .unwrap(),
            actual
        );
    }
    assert!(
        qualify_javascript_binary("node", Some("bun-1.4.3"), &environment)
            .await
            .is_err()
    );
    std::fs::write(&executable, "#!/bin/sh\nprintf 'invalid-version\n'\n").unwrap();
    assert!(
        qualify_javascript_binary("node", Some("node-24"), &environment)
            .await
            .is_err()
    );
    std::fs::remove_file(executable).unwrap();
    assert!(
        qualify_javascript_binary("node", Some("node-24"), &environment)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn fresh_typed_output_without_source_launch_needs_no_trusted_serving_target() {
    const CASE: &str = "ONREZA_FRESH_TYPED_OUTPUT_TEST";
    if std::env::var_os(CASE).is_none() {
        let output = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "deploy::application_runtime::tests::fresh_typed_output_without_source_launch_needs_no_trusted_serving_target", "--nocapture"])
            .env(CASE, "1")
            .env_remove("ONREZA_RUNTIME_VERSION")
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("package.json"), r#"{"scripts":{"build":"next build","start":"next start"},"dependencies":{"next":"16.2.9"}}"#).unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.toolchain = Some(BuildToolchainFamily::Bun);
    let mut detection = crate::detect::detect_with_framework_override(directory.path(), None);
    let original = crate::detect::application_runtime::resolve_and_bind_source_build_context(
        &crate::detect::fs::LocalFs::new(directory.path()),
        &mut detection,
        &config,
        None,
        None,
    )
    .unwrap();
    assert_eq!(detection.framework, "nextjs");
    assert_eq!(original.build_toolchain.family, BuildToolchainFamily::Bun);
    assert!(original.application_runtime.is_none());
    for platform_runner in [false, true] {
        let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
            directory.path().to_owned(),
            config.clone(),
        );
        if platform_runner {
            effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
                source_build_context: Some(original.clone()),
                ..Default::default()
            });
        }
        effective.bind_resolved_platform_context(original.clone(), None, None);
        effective.mark_primary_runtime_rebuilt();
        let mut generated = crate::build::manifest::generate_nextjs_standalone_manifest_for_server(
            false,
            "",
            "server.js",
        );
        let selected = manifest_build_runtime_target(
            &detection.metadata.runtime,
            &effective,
            &generated,
            None,
            &[],
        )
        .await
        .unwrap();
        assert_eq!(selected.as_deref(), Some("node"));
        crate::deploy::apply_rebuilt_application_runtime_manifest(
            &mut generated,
            None,
            selected.as_deref(),
            "nextjs",
        )
        .unwrap();
        assert_eq!(
            generated
                .layers
                .last()
                .unwrap()
                .runtime
                .as_ref()
                .unwrap()
                .build_runtime_version
                .as_deref(),
            Some("node")
        );
        for (family, target) in [
            (ApplicationRuntimeFamily::Node, "node-23"),
            (ApplicationRuntimeFamily::Python, "python-3.11"),
        ] {
            for retained_target in [None, Some(target)] {
                let mut manifest = crate::build::manifest::generate_compute_manifest("server.js");
                let runtime = manifest.layers[0].runtime.as_mut().unwrap();
                runtime.application_runtime = Some(nrz_source_bundle::ApplicationRuntimeIntent {
                    family,
                    args: vec![],
                });
                runtime.build_runtime_version = retained_target.map(str::to_owned);
                let before = serde_json::to_value(&manifest).unwrap();
                let target = manifest_build_runtime_target(
                    &RuntimeInfo {
                        runtime_type: RuntimeType::Bun,
                        version: Some("1.4.3".into()),
                    },
                    &effective,
                    &manifest,
                    None,
                    &[],
                )
                .await
                .unwrap();
                assert_eq!(target, None);
                crate::deploy::apply_rebuilt_application_runtime_manifest(
                    &mut manifest,
                    None,
                    target.as_deref(),
                    "other",
                )
                .unwrap();
                if retained_target.is_some() {
                    assert_eq!(serde_json::to_value(&manifest).unwrap(), before);
                } else {
                    let actual = manifest.layers[0]
                        .runtime
                        .as_ref()
                        .unwrap()
                        .build_runtime_version
                        .as_deref();
                    assert_eq!(
                        actual,
                        (family == ApplicationRuntimeFamily::Python).then_some("python")
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn platform_implicit_bun_output_keeps_qualified_compiler_target_without_source_launch() {
    const CASE: &str = "ONREZA_IMPLICIT_BUN_OUTPUT_TEST";
    if std::env::var_os(CASE).is_none() {
        let bun = measured_bun_target(&[]).await.unwrap();
        for compiler in ["BUN", "NODE"] {
            let output = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "deploy::application_runtime::tests::platform_implicit_bun_output_keeps_qualified_compiler_target_without_source_launch", "--nocapture"])
            .env(CASE, compiler)
            .env("ONREZA_BUILD_RUNTIME_VERSION", if compiler == "BUN" { &bun } else { "node-24" })
            .env("ONREZA_IMPLICIT_BUN_EXPECTED_TARGET", &bun)
            .env("ONREZA_BUILD_RUNTIME_FAMILY", "javascript")
            .env("ONREZA_BUILD_NODE_MAJOR", "24")
            .env("ONREZA_RUNTIME_VERSION", "node-24")
            .output()
            .await
            .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    }
    let compiler = std::env::var(CASE).unwrap();
    let original: SourceBuildContext = serde_json::from_value(json!({
        "schemaVersion": 1, "buildToolchain": {"family": compiler}, "applicationRuntime": null
    }))
    .unwrap();
    let mut config = nrz::config::ProjectConfig::default();
    config.build.toolchain = Some(original.build_toolchain.family);
    let mut effective = nrz::config::EffectiveProjectConfig::from_project_config(
        std::path::PathBuf::from("."),
        config,
    );
    effective.apply_platform_runner_settings(&nrz::config::ProjectBuildSettings {
        source_build_context: Some(original.clone()),
        node_version: Some("NODE_24".into()),
        ..Default::default()
    });
    let resolved =
        validate_application_runtime_before_build(&original, &effective, true, true, &[])
            .await
            .unwrap();
    assert_eq!(resolved.context, original);
    assert!(resolved.application_target.is_none());
    effective.bind_resolved_platform_context(
        resolved.context,
        resolved.application_target,
        resolved.node_version,
    );
    effective.mark_primary_runtime_rebuilt();
    let runtime = RuntimeInfo {
        runtime_type: RuntimeType::Bun,
        version: None,
    };
    let mut manifest = crate::build::manifest::generate_compute_manifest("server.js");
    // The compiler was already qualified; binding output must not probe it again.
    let unavailable = vec![("PATH".into(), "/nonexistent-implicit-bun-test".into())];
    let target = manifest_build_runtime_target(
        &runtime,
        &effective,
        &manifest,
        None,
        if compiler == "BUN" { &unavailable } else { &[] },
    )
    .await
    .unwrap();
    assert_eq!(
        target,
        Some(std::env::var("ONREZA_IMPLICIT_BUN_EXPECTED_TARGET").unwrap())
    );
    crate::deploy::apply_rebuilt_application_runtime_manifest(
        &mut manifest,
        None,
        target.as_deref(),
        "tanstack-start",
    )
    .unwrap();
    assert_eq!(
        manifest.layers[0]
            .runtime
            .as_ref()
            .unwrap()
            .build_runtime_version,
        target
    );
    assert!(
        manifest.layers[0]
            .runtime
            .as_ref()
            .unwrap()
            .application_runtime
            .is_none()
    );
    assert_eq!(effective.platform_source_build_context(), Some(&original));
    assert_eq!(effective.node_version(), Some("NODE_24"));

    let node = RuntimeInfo {
        runtime_type: RuntimeType::Node,
        version: None,
    };
    let untyped = crate::build::manifest::generate_compute_manifest("server.js");
    assert_eq!(
        manifest_build_runtime_target(&node, &effective, &untyped, None, &unavailable)
            .await
            .unwrap()
            .as_deref(),
        Some("node")
    );
    let mut siblings = crate::build::manifest::generate_compute_manifest("server.js");
    let mut second = untyped.layers[0].clone();
    second.name = "second".into();
    second.entry = Some("second.js".into());
    siblings.layers.push(second);
    assert!(
        manifest_build_runtime_target(&runtime, &effective, &siblings, None, &unavailable)
            .await
            .unwrap()
            .is_none()
    );

    for family in [
        ApplicationRuntimeFamily::Node,
        ApplicationRuntimeFamily::Bun,
    ] {
        let mut retained = crate::build::manifest::generate_compute_manifest("server.js");
        let owner = retained.layers[0].runtime.as_mut().unwrap();
        owner.application_runtime = Some(nrz_source_bundle::ApplicationRuntimeIntent {
            family,
            args: vec![],
        });
        owner.build_runtime_version = Some(
            if family == ApplicationRuntimeFamily::Node {
                "node-22"
            } else {
                "bun-1.4.2"
            }
            .into(),
        );
        let before = serde_json::to_value(&retained).unwrap();
        let selected =
            manifest_build_runtime_target(&runtime, &effective, &retained, None, &unavailable)
                .await
                .unwrap();
        assert!(selected.is_none());
        crate::deploy::apply_rebuilt_application_runtime_manifest(
            &mut retained,
            None,
            selected.as_deref(),
            "other",
        )
        .unwrap();
        assert_eq!(serde_json::to_value(retained).unwrap(), before);
    }
}
