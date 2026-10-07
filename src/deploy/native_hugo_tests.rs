use super::*;

fn hugo_binary() -> std::path::PathBuf {
    std::env::var_os("NRZ_HUGO_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "hugo".into())
}

fn module_proxy(root: &std::path::Path) -> String {
    let directory = root.join("proxy/github.com/onreza-fixture/theme/@v");
    std::fs::create_dir_all(&directory).unwrap();
    let module = "module github.com/onreza-fixture/theme\n\ngo 1.18\n";
    std::fs::write(directory.join("v1.0.0.mod"), module).unwrap();
    std::fs::write(
        directory.join("v1.0.0.info"),
        r#"{"Version":"v1.0.0","Time":"2026-01-01T00:00:00Z"}"#,
    )
    .unwrap();
    std::fs::write(directory.join("list"), "v1.0.0\n").unwrap();
    let files = serde_json::json!({
        "go.mod": module,
        "static/module.txt": "FROZEN_MODULE_RESOURCE",
        "hugo.toml": "[module]\n",
    });
    let result = std::process::Command::new("python3")
        .args(["-I", "-c", "import sys,json,zipfile\nwith zipfile.ZipFile(sys.argv[1], 'w') as archive:\n for path,contents in json.loads(sys.argv[2]).items(): archive.writestr('github.com/onreza-fixture/theme@v1.0.0/'+path, contents)\n"])
        .arg(directory.join("v1.0.0.zip"))
        .arg(files.to_string()).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    url::Url::from_directory_path(root.join("proxy"))
        .unwrap()
        .to_string()
}

fn module_site(project: &std::path::Path, proxy: &str, direct_version: bool) {
    copy_fixture(&fixture_root().join("hugo-static"), project);
    let mut config = std::fs::read_to_string(project.join("hugo.toml")).unwrap();
    config.push_str(&format!("\n[module]\nproxy='{proxy}'\nprivate=''\nnoProxy='none'\n[[module.imports]]\npath='github.com/onreza-fixture/theme'\n"));
    if direct_version {
        config.push_str("version='v1.0.0'\n");
    }
    std::fs::write(project.join("hugo.toml"), config).unwrap();
}

pub(super) fn hugo_environment(project: &std::path::Path) -> Vec<(String, String)> {
    let binary = hugo_binary();
    let mut paths = vec![
        binary
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_owned(),
    ];
    if let Some(go) = std::env::var_os("NRZ_GO_BIN") {
        paths.push(std::path::PathBuf::from(go).parent().unwrap().to_owned());
    }
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    vec![
        (
            "PATH".into(),
            std::env::join_paths(paths)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ),
        ("GOSUMDB".into(), "off".into()),
        (
            "HUGO_CACHEDIR".into(),
            project.join("hugo-cache").to_string_lossy().into_owned(),
        ),
    ]
}

fn module_inputs(project: &std::path::Path) -> Vec<Option<Vec<u8>>> {
    ["go.mod", "go.sum", "hugo.direct.sum"]
        .iter()
        .map(|path| match std::fs::read(project.join(path)) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("{error}"),
        })
        .collect()
}

#[tokio::test]
#[ignore = "requires the qualified real Hugo and Go compilers; resolves local immutable module proxy"]
async fn real_hugo_default_rejects_unfrozen_module_inputs_without_rewriting_sources() {
    let proxy_root = tempfile::tempdir().unwrap();
    let proxy = module_proxy(proxy_root.path());
    for (case, module, direct) in [
        ("missing-module", None, true),
        (
            "missing-require",
            Some("module example.com/site\n\ngo 1.18\n"),
            false,
        ),
        (
            "missing-sums",
            Some(
                "module example.com/site\n\ngo 1.18\n\nrequire github.com/onreza-fixture/theme v1.0.0\n",
            ),
            false,
        ),
        (
            "missing-direct-sums",
            Some("module example.com/site\n\ngo 1.18\n"),
            true,
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        module_site(project.path(), &proxy, direct);
        if let Some(module) = module {
            file(project.path(), "go.mod", module);
        }
        let before = module_inputs(project.path());
        let plan = recipe_commands(project.path(), NativeRecipe::HugoStatic, false).unwrap();
        let result = crate::deploy::run_native_build_step(
            &plan,
            NativeRecipe::HugoStatic,
            project.path(),
            true,
            &hugo_environment(project.path()),
            None,
            false,
        )
        .await;
        assert!(
            result.is_err(),
            "{case}: default Hugo accepted unfrozen dependency inputs"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("dependency inputs"),
            "{case}"
        );
        assert_eq!(
            module_inputs(project.path()),
            before,
            "{case}: authored inputs mutated"
        );
    }
}

#[tokio::test]
#[ignore = "requires qualified real Hugo and Go; builds ordinary and frozen-module sites through the deploy lifecycle"]
async fn real_hugo_frozen_modules_and_plain_sites_preserve_inputs_and_assets() {
    let proxy_root = tempfile::tempdir().unwrap();
    let proxy = module_proxy(proxy_root.path());
    for case in ["plain", "module", "direct-version"] {
        let project = tempfile::tempdir().unwrap();
        if case == "plain" {
            copy_fixture(&fixture_root().join("hugo-static"), project.path());
        } else {
            module_site(project.path(), &proxy, case == "direct-version");
            file(
                project.path(),
                "go.mod",
                "module example.com/site\n\ngo 1.18\n",
            );
            // Fixture setup intentionally resolves inputs once. Deployment must
            // use the resulting reviewed bytes without extending these locks.
            let setup = std::process::Command::new(hugo_binary())
                .args(["--environment", "production", "--destination", "public"])
                .envs(hugo_environment(project.path()))
                .current_dir(project.path())
                .output()
                .unwrap();
            assert!(
                setup.status.success(),
                "{}",
                String::from_utf8_lossy(&setup.stderr)
            );
        }
        let before = module_inputs(project.path());
        let workspace = proxy_root.path().join("outside.work");
        std::fs::write(&workspace, "invalid workspace directive\n").unwrap();
        file(project.path(), "public/stale.txt", "STALE_OUTPUT");
        let plan = recipe_commands(project.path(), NativeRecipe::HugoStatic, false).unwrap();
        let mut environment = hugo_environment(project.path());
        // This is an input override, not a mocked compiler. The recipe must
        // force the workspace off before Hugo invokes Go.
        environment.push((
            "HUGO_MODULE_WORKSPACE".into(),
            workspace.to_string_lossy().into_owned(),
        ));
        crate::deploy::run_native_build_step(
            &plan,
            NativeRecipe::HugoStatic,
            project.path(),
            true,
            &environment,
            None,
            false,
        )
        .await
        .unwrap();
        assert_eq!(module_inputs(project.path()), before, "{case}");
        assert!(project.path().join("public/about/index.html").is_file());
        assert_eq!(
            std::fs::read(project.path().join("public/fixture.svg")).unwrap(),
            std::fs::read(project.path().join("static/fixture.svg")).unwrap()
        );
        assert!(!project.path().join("public/stale.txt").exists());
        if case != "plain" {
            assert_eq!(
                std::fs::read_to_string(project.path().join("public/module.txt")).unwrap(),
                "FROZEN_MODULE_RESOURCE"
            );
        }
        if case == "module" {
            file(
                project.path(),
                "layouts/home.html",
                "{{ invalid template call",
            );
            let failed = crate::deploy::run_native_build_step(
                &plan,
                NativeRecipe::HugoStatic,
                project.path(),
                true,
                &environment,
                None,
                false,
            )
            .await;
            assert!(failed.is_err(), "invalid authored template must fail");
            assert_eq!(
                module_inputs(project.path()),
                before,
                "failed compiler mutated authored inputs"
            );
        }
    }
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "requires real Hugo; preserves contained asset symlinks through the isolated source tree"]
async fn real_hugo_source_mirror_preserves_contained_links_and_rejects_escape() {
    let project = tempfile::tempdir().unwrap();
    copy_fixture(&fixture_root().join("hugo-static"), project.path());
    file(project.path(), "payload/resource.txt", "CONTAINED_ASSET");
    std::os::unix::fs::symlink("resource.txt", project.path().join("payload/link.txt")).unwrap();
    let plan = recipe_commands(project.path(), NativeRecipe::HugoStatic, false).unwrap();
    // Hugo ignores some symlink resources. Preserve the underlying source
    // semantics rather than inventing a new renderer behavior for those files.
    let mut command = plan.build.clone();
    let inputs =
        crate::deploy::native_build::HugoModuleInputs::freeze(project.path(), &mut command)
            .unwrap();
    let source = std::path::Path::new(command.arguments.last().unwrap());
    assert_eq!(
        std::fs::read_link(source.join("payload/link.txt")).unwrap(),
        std::path::Path::new("resource.txt")
    );
    assert_eq!(
        std::fs::read_to_string(source.join("payload/link.txt")).unwrap(),
        "CONTAINED_ASSET"
    );
    inputs.verify().unwrap();
    crate::deploy::run_native_build_step(
        &plan,
        NativeRecipe::HugoStatic,
        project.path(),
        true,
        &hugo_environment(project.path()),
        None,
        false,
    )
    .await
    .unwrap();
    assert!(project.path().join("public/about/index.html").is_file());
    assert_eq!(
        std::fs::read(project.path().join("public/fixture.svg")).unwrap(),
        std::fs::read(project.path().join("static/fixture.svg")).unwrap()
    );
    let external = tempfile::tempdir().unwrap();
    file(external.path(), "resource.txt", "OUTSIDE_SOURCE");
    std::os::unix::fs::symlink(
        external.path().join("resource.txt"),
        project.path().join("static/escape.txt"),
    )
    .unwrap();
    let failed = crate::deploy::run_native_build_step(
        &plan,
        NativeRecipe::HugoStatic,
        project.path(),
        true,
        &hugo_environment(project.path()),
        None,
        false,
    )
    .await;
    assert!(
        failed
            .unwrap_err()
            .to_string()
            .contains("symlinks must be relative and contained")
    );
    assert_eq!(
        std::fs::read_to_string(external.path().join("resource.txt")).unwrap(),
        "OUTSIDE_SOURCE"
    );
}

#[test]
#[ignore = "requires real Hugo and Go; proves the default recipe blocks module toolchain downloads"]
fn real_hugo_default_forbids_module_toolchain_downloads() {
    let proxy_root = tempfile::tempdir().unwrap();
    let proxy = module_proxy(proxy_root.path());
    let project = tempfile::tempdir().unwrap();
    module_site(project.path(), &proxy, false);
    file(
        project.path(),
        "go.mod",
        "module example.com/site\n\ngo 1.999.0\n\nrequire github.com/onreza-fixture/theme v1.0.0\n",
    );
    let before = module_inputs(project.path());
    let mut plan = recipe_commands(project.path(), NativeRecipe::HugoStatic, false).unwrap();
    crate::deploy::plan::clear_native_build_output(project.path(), &plan.output_directory).unwrap();
    let inputs =
        crate::deploy::native_build::HugoModuleInputs::freeze(project.path(), &mut plan.build)
            .unwrap();
    let environment = crate::deploy::merge_command_environment(
        &[
            hugo_environment(project.path()),
            vec![
                ("GOTOOLCHAIN".into(), "auto".into()),
                (
                    "GOENV".into(),
                    project
                        .path()
                        .join("host-go-env")
                        .to_string_lossy()
                        .into_owned(),
                ),
            ],
        ]
        .concat(),
        &plan.environment,
    );
    let output = std::process::Command::new(hugo_binary())
        .args(&plan.build.arguments)
        .envs(environment)
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("GOTOOLCHAIN=local"), "{stderr}");
    assert!(!stderr.contains("downloading go1.999.0"), "{stderr}");
    inputs.verify().unwrap();
    assert_eq!(module_inputs(project.path()), before);
}

#[tokio::test]
#[ignore = "requires real Hugo; executes the deploy lifecycle with process-isolated TMPDIR inside the project"]
async fn real_hugo_project_tmpdir_does_not_include_its_own_mirror() {
    const WORKER_PROJECT: &str = "NRZ_HUGO_TEMP_WORKER_PROJECT";
    if let Some(project) = std::env::var_os(WORKER_PROJECT) {
        let project = std::path::PathBuf::from(project);
        let before = module_inputs(&project);
        let plan = recipe_commands(&project, NativeRecipe::HugoStatic, false).unwrap();
        crate::deploy::run_native_build_step(
            &plan,
            NativeRecipe::HugoStatic,
            &project,
            true,
            &hugo_environment(&project),
            None,
            false,
        )
        .await
        .unwrap();
        assert_eq!(module_inputs(&project), before);
        assert!(project.join("public/about/index.html").is_file());
        assert_eq!(
            std::fs::read_to_string(project.join("public/authored.txt")).unwrap(),
            "AUTHORED_TMP_ASSET"
        );
        return;
    }
    let project = tempfile::tempdir().unwrap();
    copy_fixture(&fixture_root().join("hugo-static"), project.path());
    file(
        project.path(),
        ".tmp/authored/authored.txt",
        "AUTHORED_TMP_ASSET",
    );
    let mut config = std::fs::read_to_string(project.path().join("hugo.toml")).unwrap();
    config.push_str("\n[module]\n[[module.mounts]]\nsource='.tmp/authored'\ntarget='static'\n");
    std::fs::write(project.path().join("hugo.toml"), config).unwrap();
    // Changing a global process environment is unsafe in a parallel Rust test
    // harness. A separate test process gives the real tempdir factory this env.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "deploy::native_build_tests::hugo_tests::real_hugo_project_tmpdir_does_not_include_its_own_mirror", "--ignored", "--nocapture"])
        .env(WORKER_PROJECT, project.path())
        .env("TMPDIR", project.path().join(".tmp"))
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
