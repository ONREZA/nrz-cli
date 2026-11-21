use super::{
    application_runtime::resolve_and_bind_detection,
    fs::{Fs, LocalFs, VirtualFs},
};

fn detected(fs: &dyn Fs) -> serde_json::Value {
    let mut detection = super::detect_with_fs(fs);
    resolve_and_bind_detection(fs, &mut detection).unwrap();
    serde_json::to_value(detection).unwrap()
}

#[test]
fn authored_recipe_facts_match_local_and_remote_without_source_contents() {
    for family in ["node", "bun"] {
        for key in ["output_directory", "output_dir"] {
            for output in [" ./compiled/ ", "", "   "] {
                let config = format!(
                    "[build]\ntoolchain='{family}'\n{key}='{output}'\n[deploy]\nruntime='{family}'\nentry='server.js'\nargs=[]\n[env]\nSECRET='private-value'\n"
                );
                let package = r#"{"scripts":{"build":""},"privateSecret":"package-private-value"}"#;
                let files = serde_json::json!({
                    "onreza.toml": config,
                    "package.json": package,
                    "server.js": "// application source"
                });
                let remote =
                    VirtualFs::from_json(&serde_json::json!({"files": files}).to_string()).unwrap();
                let project = tempfile::tempdir().unwrap();
                for (path, source) in files.as_object().unwrap() {
                    std::fs::write(project.path().join(path), source.as_str().unwrap()).unwrap();
                }
                let local = detected(&LocalFs::new(project.path()));
                let remote = detected(&remote);
                assert_eq!(local, remote);
                assert_eq!(
                    remote["sourceBuildFacts"],
                    serde_json::json!({
                        "declaredOutputDirectory": output,
                        "packageBuild": {"status":"valid", "hasBuildScript": true}
                    })
                );
                let serialized = remote.to_string();
                for secret in [
                    "private-value",
                    "package-private-value",
                    "application source",
                ] {
                    assert!(!serialized.contains(secret));
                }
            }
        }
    }
}

#[test]
fn package_build_facts_validate_only_the_fields_previously_read_by_server() {
    for (package, expected) in [
        (
            r#"{}"#,
            serde_json::json!({"status":"valid", "hasBuildScript":false}),
        ),
        (
            r#"{"scripts":{}}"#,
            serde_json::json!({"status":"valid", "hasBuildScript":false}),
        ),
        (
            r#"{"scripts":{"start":7,"test":false}}"#,
            serde_json::json!({"status":"valid", "hasBuildScript":false}),
        ),
        (
            r#"{"scripts":{"build":""}}"#,
            serde_json::json!({"status":"valid", "hasBuildScript":true}),
        ),
        (
            r#"{"scripts":{"build":null}}"#,
            serde_json::json!({"status":"invalid"}),
        ),
        (
            r#"{"scripts":{"build":7}}"#,
            serde_json::json!({"status":"invalid"}),
        ),
        (
            r#"{"scripts":null}"#,
            serde_json::json!({"status":"invalid"}),
        ),
        (
            r#"{"scripts":[]}"#,
            serde_json::json!({"status":"valid", "hasBuildScript":false}),
        ),
        (
            r#"[]"#,
            serde_json::json!({"status":"valid", "hasBuildScript":false}),
        ),
        (r#"{"privateSecret":"SECRET""#, serde_json::Value::Null),
        (r#"{"unused":1e400}"#, serde_json::Value::Null),
    ] {
        let fs = VirtualFs::from_json(
            &serde_json::json!({"files": {
                "onreza.toml":"[deploy]\nruntime='node'\nentry='server.js'\nargs=[]\n",
                "package.json": package,
                "server.js": "// entry"
            }})
            .to_string(),
        )
        .unwrap();
        let baseline = VirtualFs::from_json(
            r#"{"files":{"onreza.toml":"[deploy]\nruntime='node'\nentry='server.js'\nargs=[]\n"}}"#,
        )
        .unwrap();
        let mut detection = super::detect_with_fs(&baseline);
        resolve_and_bind_detection(&baseline, &mut detection).unwrap();
        let facts = super::source_build_facts::collect(
            &fs,
            &detection,
            &crate::config::ProjectConfig::default(),
            true,
        )
        .unwrap();
        let result = serde_json::to_value(facts).unwrap();
        assert_eq!(result["packageBuild"], expected, "{package}");
        assert!(!result.to_string().contains("SECRET"));
    }
}

#[test]
fn facts_distinguish_unknown_content_from_an_authored_file_without_output() {
    for (config, expected) in [
        (None, serde_json::json!({})),
        (
            Some(""),
            serde_json::json!({"declaredOutputDirectory": null}),
        ),
    ] {
        let mut files = serde_json::json!({
            "server.js": "// entry",
            "package.json": r#"{"scripts":{"start":"node server.js"}}"#,
        });
        if let Some(config) = config {
            files["onreza.toml"] = config.into();
        }
        let fs = VirtualFs::from_json(&serde_json::json!({"files": files}).to_string()).unwrap();
        let result = detected(&fs);
        let facts = result["sourceBuildFacts"].as_object().unwrap();
        assert_eq!(
            facts.get("declaredOutputDirectory"),
            expected.get("declaredOutputDirectory")
        );
        assert_eq!(facts["packageBuild"]["hasBuildScript"], false);
    }
}

#[test]
fn non_javascript_build_or_serving_paths_do_not_acquire_recipe_validation() {
    for config in [
        "[deploy]\ncompute='static'\n",
        "[build]\ntoolchain='python'\n[deploy]\nruntime='node'\nentry='server.js'\n",
        "[build]\ntoolchain='node'\n[deploy]\nruntime='python'\nentry='server.py'\n",
    ] {
        let fs = VirtualFs::from_json(
            &serde_json::json!({"files": {
                "onreza.toml": config,
                "package.json": "{}",
                "server.js": "// entry",
                "server.py": "print('ready')"
            }})
            .to_string(),
        )
        .unwrap();
        assert!(detected(&fs).get("sourceBuildFacts").is_none(), "{config}");
    }
}
