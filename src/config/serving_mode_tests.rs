use super::*;
use nrz_source_bundle::{ApplicationRuntimeFamily as Family, BuildToolchainFamily};

fn bind(config: &ProjectConfig) -> anyhow::Result<nrz_source_bundle::SourceBuildContext> {
    let fs = crate::detect::fs::VirtualFs::from_json(
        r#"{"files":{"index.html":"<html></html>","main.py":"print('ready')","main.js":"console.log('ready')","server":"native"}}"#,
    ).unwrap();
    let mut detection = crate::detect::detect_with_fs(&fs);
    crate::detect::application_runtime::resolve_and_bind_source_build_context(
        &fs,
        &mut detection,
        config,
        config.project.framework.as_deref(),
        None,
    )
}

#[test]
fn child_static_selection_clears_inherited_process_namespace() {
    for family in [
        Family::Node,
        Family::Bun,
        Family::Python,
        Family::Executable,
    ] {
        let mut parent = ProjectConfig::default();
        parent.build.toolchain = Some(BuildToolchainFamily::Bun);
        parent.build.command = Some("parent build".into());
        parent.deploy.compute = Some("process".into());
        parent.deploy.runtime = Some(family);
        parent.deploy.entry = Some("parent-entry".into());
        parent.deploy.args = Some(vec!["parent argument".into()]);
        parent.deploy.python_version =
            (family == Family::Python).then_some(nrz_source_bundle::PythonMinor::Python312);
        let mut child = ProjectConfig::default();
        child.deploy.compute = Some("static".into());
        let merged = parent.merge_child(child);
        let context = bind(&merged)
            .unwrap_or_else(|error| panic!("child STATIC inherited {family:?}: {error:#}"));
        assert_eq!(context.application_runtime, None);
        assert_eq!(context.build_toolchain.family, BuildToolchainFamily::Bun);
        assert_eq!(merged.build.command.as_deref(), Some("parent build"));
        assert_eq!(merged.deploy.entry, None);
        assert_eq!(merged.deploy.args, None);
        assert_eq!(merged.deploy.python_version, None);
    }
}

#[test]
fn child_process_launch_replaces_inherited_static_compute() {
    let mut parent = ProjectConfig::default();
    parent.deploy.compute = Some("static".into());
    for (family, entry) in [
        (Family::Node, "main.js"),
        (Family::Bun, "main.js"),
        (Family::Python, "main.py"),
        (Family::Executable, "server"),
    ] {
        let mut child = ProjectConfig::default();
        child.deploy.runtime = Some(family);
        child.deploy.entry = Some(entry.into());
        child.deploy.args = Some(vec![]);
        let merged = parent.merge_child(child);
        assert_eq!(merged.deploy.compute, None);
        assert_eq!(
            bind(&merged).unwrap().application_runtime.unwrap().family,
            family
        );
    }
    for field in ["entry", "args", "module", "python_version", "application"] {
        let mut child = ProjectConfig::default();
        match field {
            "entry" => child.deploy.entry = Some("main.js".into()),
            "args" => child.deploy.args = Some(vec![]),
            "module" => child.deploy.module = Some("main".into()),
            "python_version" => {
                child.deploy.python_version = Some(nrz_source_bundle::PythonMinor::Python312)
            }
            _ => {
                child.deploy.application = Some("main:app".into());
                child.deploy.server = Some("asgi".into());
            }
        }
        assert_eq!(
            parent.merge_child(child).deploy.compute,
            None,
            "child {field} inherited STATIC"
        );
    }
}

#[test]
fn child_python_launch_after_static_parent_does_not_inherit_inactive_same_family_fields() {
    let mut parent = ProjectConfig::default();
    parent.deploy.compute = Some("static".into());
    parent.deploy.runtime = Some(Family::Python);
    parent.deploy.entry = Some("inactive-parent.py".into());
    parent.deploy.args = Some(vec!["inactive argument".into()]);
    parent.deploy.python_version = Some(nrz_source_bundle::PythonMinor::Python312);
    for explicit_runtime in [false, true] {
        let mut child = ProjectConfig::default();
        child.deploy.runtime = explicit_runtime.then_some(Family::Python);
        child.deploy.module = Some("main".into());
        let expected = bind(&child).unwrap().application_runtime.unwrap();
        let merged = parent.merge_child(child);
        assert_eq!(merged.deploy.compute, None);
        assert_eq!(merged.deploy.entry, None);
        assert_eq!(merged.deploy.args, None);
        assert_eq!(merged.deploy.python_version, None);
        let context = bind(&merged).unwrap();
        let serving = context.application_runtime.unwrap();
        assert_eq!(serving.family, Family::Python);
        assert_eq!(serving, expected);
    }
}

#[test]
fn child_compute_process_clears_static_parent_launch_defaults_but_keeps_process_defaults() {
    for parent_compute in ["static", "process"] {
        let mut parent = ProjectConfig::default();
        parent.deploy.compute = Some(parent_compute.into());
        parent.deploy.runtime = Some(Family::Python);
        parent.deploy.module = Some("main".into());
        parent.deploy.python_version = Some(nrz_source_bundle::PythonMinor::Python312);
        parent.deploy.args = Some(vec!["parent argument".into()]);
        let mut child = ProjectConfig::default();
        child.deploy.compute = Some("process".into());
        let merged = parent.merge_child(child);
        if parent_compute == "static" {
            assert_eq!(merged.deploy.selected_runtime_family(), None);
            assert_eq!(merged.deploy.module, None);
            assert_eq!(merged.deploy.args, None);
        } else {
            assert_eq!(merged.deploy.runtime, parent.deploy.runtime);
            assert_eq!(merged.deploy.module, parent.deploy.module);
            assert_eq!(merged.deploy.args, parent.deploy.args);
            assert_eq!(
                bind(&merged)
                    .unwrap()
                    .application_runtime
                    .unwrap()
                    .python_version,
                parent.deploy.python_version
            );
        }
    }
}

#[test]
fn authored_child_static_process_fields_remain_invalid() {
    for field in [
        "runtime",
        "entry",
        "args",
        "module",
        "application",
        "server",
        "python_version",
    ] {
        let mut parent = ProjectConfig::default();
        parent.deploy.runtime = Some(Family::Python);
        parent.deploy.module = Some("parent".into());
        let mut child = ProjectConfig::default();
        child.deploy.compute = Some("static".into());
        match field {
            "runtime" => child.deploy.runtime = Some(Family::Node),
            "entry" => child.deploy.entry = Some("main.js".into()),
            "args" => child.deploy.args = Some(vec![]),
            "module" => child.deploy.module = Some("main".into()),
            "application" => child.deploy.application = Some("main:app".into()),
            "server" => child.deploy.server = Some("asgi".into()),
            _ => child.deploy.python_version = Some(nrz_source_bundle::PythonMinor::Python312),
        }
        let error = bind(&parent.merge_child(child)).unwrap_err();
        assert!(
            error.to_string().contains("STATIC serving conflicts"),
            "child {field}: {error:#}"
        );
    }
}
