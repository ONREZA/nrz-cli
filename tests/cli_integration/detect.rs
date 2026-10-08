use super::*;

#[test]
fn detect_preserves_bun_start_runtime_before_build() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"main":"other.js","scripts":{"start":"bun run src/server.ts --port 8080"}}"#,
    )
    .unwrap();
    let output = nrz()
        .current_dir(&temp)
        .args(["detect", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        result["metadata"]["sourceBuildContext"]["applicationRuntime"],
        serde_json::json!({"family":"BUN","entry":"src/server.ts","args":["--port","8080"]})
    );
}

#[test]
fn remote_runtime_declaration_is_independent_of_installer_and_lockfile() {
    for (manager, start, config, family, entry, args) in [
        (
            "npm@11.0.0",
            "bun run source.ts",
            "",
            "BUN",
            "source.ts",
            json!([]),
        ),
        (
            "bun@1.4.2",
            "node source.js",
            "",
            "NODE",
            "source.js",
            json!([]),
        ),
        (
            "npm@11.0.0",
            "bun run source.ts",
            "[deploy]\nruntime='bun'\nentry='output.js'\nargs=['--literal','a b']\n",
            "BUN",
            "output.js",
            json!(["--literal", "a b"]),
        ),
    ] {
        let package =
            json!({"packageManager":manager,"main":"unrelated.js","scripts":{"start":start}});
        let manifest = json!({"tree":["package.json","onreza.toml","bun.lock","package-lock.json"],"files":{"package.json":package.to_string(),"onreza.toml":config}});
        let output = nrz()
            .args(["detect", "--stdin", "--json"])
            .write_stdin(manifest.to_string())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            result["metadata"]["sourceBuildContext"]["applicationRuntime"],
            json!({"family":family,"entry":entry,"args":args})
        );
    }
}

#[test]
fn remote_runtime_rejects_flags_shells_and_conflicting_declarations() {
    for (start, config) in [
        ("node --experimental-transform-types source.ts", ""),
        ("bun source.ts && node other.js", ""),
        ("cross-env NODE_ENV=production bun source.ts", ""),
        ("bun source.ts", "[deploy]\nruntime='node'\n"),
        (
            "node --loader tsx source.ts",
            "[deploy]\nruntime='bun'\nentry='output.js'\nargs=[]\n",
        ),
    ] {
        let manifest = json!({"tree":["package.json","onreza.toml"],"files":{"package.json":json!({"scripts":{"start":start}}).to_string(),"onreza.toml":config}});
        let output = nrz()
            .args(["detect", "--stdin", "--json"])
            .write_stdin(manifest.to_string())
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(error["code"], "APPLICATION_RUNTIME_INVALID");
    }
}

#[test]
fn full_runtime_declaration_replaces_unsupported_start_syntax() {
    let manifest = json!({"tree":["package.json","onreza.toml"],"files":{
        "package.json":json!({"scripts":{"start":"node --loader tsx src/server.ts"}}).to_string(),
        "onreza.toml":"[deploy]\nruntime='node'\nentry='dist/server.js'\nargs=['--port','8080']\n"
    }});
    let output = nrz()
        .args(["detect", "--stdin", "--json"])
        .write_stdin(manifest.to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        result["metadata"]["sourceBuildContext"]["applicationRuntime"],
        json!({"family":"NODE","entry":"dist/server.js","args":["--port","8080"]})
    );
}

#[test]
fn detect_nextjs_project() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"nextjs\""))
        .stdout(contains("\"name\":\"Next.js\""));
}

#[test]
fn detect_astro_project() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"astro": "4.0.0"}}"#,
    )
    .unwrap();
    fs::write(temp.path().join("pnpm-lock.yaml"), "").unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"astro\""))
        .stdout(contains("\"pnpm\""));
}

#[test]
fn detect_static_html_site() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("index.html"),
        "<html><body>hello</body></html>",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"static-html\""));
}

#[test]
fn detect_unknown_project() {
    let temp = tempfile::tempdir().unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"other\""));
}

#[test]
fn detect_slug_only() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"nuxt": "3.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp)
        .args(["detect", "--slug-only", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"nuxt\""));
}

#[test]
fn detect_with_package_manager_field() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"packageManager": "bun@1.0.0", "dependencies": {"vite": "5.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"vite\""))
        .stdout(contains("\"bun\""));
}

#[test]
fn detect_suggested_compute_static_for_vite() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"vite": "5.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"suggestedCompute\":\"STATIC\""));
}

#[test]
fn detect_suggested_compute_process_for_nextjs() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"next": "14.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_suggested_compute_process_for_remix() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"@remix-run/react": "2.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"remix\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_remix_spa_mode_is_static() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"@remix-run/react": "2.0.0", "react": "18.0.0"}}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("vite.config.ts"),
        r#"import { vitePlugin as remix } from "@remix-run/dev";
export default defineConfig({ plugins: [remix({ ssr: false })] })"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"remix\""))
        .stdout(contains("\"suggestedCompute\":\"STATIC\""));
}

#[test]
fn detect_react_router_v7() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"devDependencies": {"@react-router/dev": "7.0.0"}, "dependencies": {"react-router": "7.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"react-router\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_hono_is_process() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"hono": "4.0.0"}}"#,
    )
    .unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("src/server.ts"),
        r#"import { Hono } from "hono";"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"hono\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_elysia_is_process() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"elysia": "1.0.0"}}"#,
    )
    .unwrap();
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("src/server.ts"),
        r#"import { Elysia } from "elysia";"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"elysia\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_stdin_hono_uses_entry_content_signal() {
    let manifest = json!({
        "tree": ["package.json", "src/server.ts"],
        "files": {
            "package.json": r#"{"dependencies":{"hono":"4.0.0"}}"#,
            "src/server.ts": r#"import { Hono } from "hono";"#
        }
    });

    let mut cmd = nrz();
    cmd.args(["detect", "--stdin", "--json"])
        .write_stdin(manifest.to_string());
    cmd.assert()
        .success()
        .stdout(contains("\"framework\":\"hono\""))
        .stdout(contains("\"suggestedCompute\":\"PROCESS\""));
}

#[test]
fn detect_stdin_validation_error_has_machine_readable_code() {
    let mut cmd = nrz();
    cmd.args(["detect", "--stdin", "--json"])
        .write_stdin(r#"{"tree":["../outside"],"files":{}}"#);

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let value = stdout_json(&output);
    assert_eq!(value["code"], "DETECTION_INPUT_INVALID");
    assert!(
        value["error"]
            .as_str()
            .is_some_and(|error| error.contains("path must be relative"))
    );
}

#[test]
fn detect_needed_files_includes_server_entry_candidates() {
    let mut cmd = nrz();
    cmd.args(["detect", "--needed-files", "--json"]);
    cmd.assert()
        .success()
        .stdout(contains("\"src/server.ts\""))
        .stdout(contains("\"nitro.config.ts\""));
}

#[test]
fn tree_aware_needed_files_preserves_local_go_detection() {
    for (path, tree) in [
        ("server.go", ["go.mod", "server.go"]),
        ("main_linux_amd64.go", ["go.mod", "main_linux_amd64.go"]),
        ("cmd/server/main.go", ["go.mod", "cmd/server/main.go"]),
        ("server.go", [r".\go.mod", r".\server.go"]),
        ("cmd/server/main.go", ["./go.mod", r"cmd\server\.\main.go"]),
        ("cmd/server/main.go", ["go.mod", "cmd//server/main.go"]),
        ("cmd/server/main.go", ["go.mod", "cmd/./server/main.go"]),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let contents = std::collections::BTreeMap::from([
            ("go.mod", "module example.org/server\n"),
            (
                path,
                "//go:build linux && amd64\n\npackage main\nfunc main() {}\n",
            ),
        ]);
        for (name, content) in &contents {
            let target = temp.path().join(name);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, content).unwrap();
        }
        let output = nrz()
            .args(["detect", "--needed-files", "--stdin", "--json"])
            .write_stdin(json!({"tree":tree,"files":{}}).to_string())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let requested = stdout_json(&output);
        assert_eq!(
            requested["files"],
            json!(
                ["go.mod", path]
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>()
            )
        );
        let fetched = requested["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| {
                let name = name.as_str().unwrap();
                (name, contents[name])
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let remote = nrz()
            .args(["detect", "--stdin", "--json"])
            .write_stdin(json!({"tree":tree,"files":fetched}).to_string())
            .output()
            .unwrap();
        let local = nrz()
            .current_dir(&temp)
            .args(["detect", "--json"])
            .output()
            .unwrap();
        assert!(remote.status.success() && local.status.success());
        let remote = stdout_json(&remote);
        let local = stdout_json(&local);
        assert_eq!(remote, local, "{path}");
        assert_eq!(remote["framework"], "go");
        assert_eq!(remote["suggestedCompute"], "PROCESS");
        assert_eq!(
            remote["metadata"]["sourceBuildContext"]["applicationRuntime"]["family"],
            "EXECUTABLE"
        );
    }
}

#[test]
fn tree_aware_needed_files_reports_invalid_or_over_budget_manifests() {
    for manifest in [
        json!({"tree":["../outside.go"],"files":{}}),
        json!({"tree":(0..257).map(|index|format!("cmd/server_{index}/main.go")).collect::<Vec<_>>(),"files":{}}),
        json!({"files":{"package.json":"{}","./package.json":"{\"main\":\"server.js\"}"}}),
        json!({"files":{"package.json":"{}",".\\package.json":"{}"}}),
        json!({"tree":["src/"],"files":{"src":"not a directory"}}),
        json!({"files":{".":"not a root directory"}}),
    ] {
        let output = nrz()
            .args(["detect", "--needed-files", "--stdin", "--json"])
            .write_stdin(manifest.to_string())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(stdout_json(&output)["code"], "DETECTION_INPUT_INVALID");
    }
    let output = nrz()
        .args(["detect", "--needed-files", "--stdin", "--json"])
        .write_stdin(" ".repeat(4 * 1024 * 1024 + 1))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(stdout_json(&output)["code"], "DETECTION_INPUT_TOO_LARGE");
}

#[test]
fn tree_aware_content_fetch_preserves_constraints_and_framework_precedence() {
    let main = "package main\nfunc main() {}\n";
    let module = ("go.mod", "module example.org/server\n");
    let cases = [
        (
            "windows filename",
            vec![module, ("cmd/server/main_windows.go", main)],
            "other",
        ),
        (
            "windows header",
            vec![
                module,
                (
                    "cmd/server/main.go",
                    "//go:build windows\n\npackage main\nfunc main() {}\n",
                ),
            ],
            "other",
        ),
        (
            "library",
            vec![module, ("cmd/server/library.go", "package library\n")],
            "other",
        ),
        (
            "cgo",
            vec![
                module,
                (
                    "cmd/server/main.go",
                    "package main\nimport \"C\"\nfunc main() {}\n",
                ),
            ],
            "other",
        ),
        (
            "nested package",
            vec![module, ("cmd/server/deep/main.go", main)],
            "other",
        ),
        (
            "ambiguous packages",
            vec![
                module,
                ("cmd/first/main.go", main),
                ("cmd/second/serve.go", main),
            ],
            "go",
        ),
        (
            "tool metadata",
            vec![
                module,
                ("cmd/server/main.go", main),
                (
                    "package.json",
                    r#"{"devDependencies":{"prettier":"3.0.0"}}"#,
                ),
            ],
            "go",
        ),
        (
            "javascript entry",
            vec![
                module,
                ("cmd/server/main.go", main),
                ("server.js", "console.log('server');\n"),
                ("package.json", r#"{"scripts":{"start":"npm run boot"}}"#),
            ],
            "other",
        ),
        (
            "python application",
            vec![
                module,
                ("cmd/server/main.go", main),
                ("requirements.txt", "fastapi>=0.100\n"),
                ("main.py", "from fastapi import FastAPI\napp = FastAPI()\n"),
            ],
            "fastapi",
        ),
        (
            "flutter web",
            vec![
                module,
                ("cmd/server/main.go", main),
                (
                    "pubspec.yaml",
                    "name: fixture\ndependencies:\n  flutter:\n    sdk: flutter\n",
                ),
                ("web/index.html", "<html></html>"),
            ],
            "flutter",
        ),
        (
            "hugo generator",
            vec![
                module,
                ("cmd/server/main.go", main),
                ("hugo.toml", "baseURL = 'https://example.org/'\n"),
            ],
            "hugo",
        ),
    ];
    for (label, files, framework) in cases {
        let temp = tempfile::tempdir().unwrap();
        let contents = files
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        for (name, content) in &contents {
            let target = temp.path().join(name);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, content).unwrap();
        }
        let tree = contents.keys().copied().collect::<Vec<_>>();
        let query = nrz()
            .args(["detect", "--needed-files", "--stdin", "--json"])
            .write_stdin(json!({"tree":tree,"files":{}}).to_string())
            .output()
            .unwrap();
        assert!(query.status.success(), "{label}");
        let request = stdout_json(&query);
        let fetched = request["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|path| {
                let path = path.as_str().unwrap();
                (path, contents[path])
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let remote = nrz()
            .args(["detect", "--stdin", "--json"])
            .write_stdin(json!({"tree":tree,"files":fetched}).to_string())
            .output()
            .unwrap();
        let local = nrz()
            .current_dir(&temp)
            .args(["detect", "--json"])
            .output()
            .unwrap();
        assert!(
            remote.status.success() && local.status.success(),
            "{label}: {} {}",
            String::from_utf8_lossy(&remote.stdout),
            String::from_utf8_lossy(&local.stdout)
        );
        assert_eq!(stdout_json(&remote), stdout_json(&local), "{label}");
        assert_eq!(stdout_json(&remote)["framework"], framework, "{label}");
    }
}

#[test]
fn detect_save_writes_framework_to_toml() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"astro": "4.0.0"}}"#,
    )
    .unwrap();
    // Create onreza.toml (--save requires it)
    fs::write(
        temp.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\n",
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--save", "--json"]);
    cmd.assert().success();

    let content = fs::read_to_string(temp.path().join("onreza.toml")).unwrap();
    assert!(
        content.contains("framework = \"astro\""),
        "onreza.toml should contain framework: {content}"
    );
}

#[test]
fn detect_save_without_onreza_toml_fails_honestly() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"dependencies": {"vite": "5.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = nrz();
    cmd.current_dir(&temp).args(["detect", "--save", "--json"]);
    cmd.assert()
        .failure()
        .stdout(contains("cannot save detected framework"))
        .stdout(contains("onreza.toml not found"));

    assert!(!temp.path().join("onreza.toml").exists());
}

#[test]
fn detect_nonexistent_directory_returns_error() {
    let mut cmd = nrz();
    cmd.args(["detect", "--json", "/tmp/nrz_test_nonexistent_dir_12345"]);
    cmd.assert().failure();
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("error"),
        "should return JSON error for nonexistent dir: {stdout}"
    );
}

#[test]
fn configured_server_entry_is_a_generic_process_signal_without_runtime_family() {
    let config = "[deploy]\nentry='server.js'\n";
    let manifest = json!({"tree":["onreza.toml","index.html","server.js"],"files":{"onreza.toml":config,"index.html":"<h1>server content</h1>","server.js":"console.log('server')"}});
    let output = nrz()
        .args(["detect", "--stdin", "--json"])
        .write_stdin(manifest.to_string())
        .output()
        .unwrap();
    assert!(output.status.success());
    let result = stdout_json(&output);
    assert_eq!(result["suggestedCompute"], "PROCESS");
    assert!(result["metadata"]["sourceBuildContext"]["applicationRuntime"].is_null());
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("onreza.toml"), config).unwrap();
    fs::write(temp.path().join("index.html"), "<h1>server content</h1>").unwrap();
    fs::write(temp.path().join("server.js"), "console.log('server')").unwrap();
    let local = nrz()
        .current_dir(temp.path())
        .args(["detect", "--json"])
        .output()
        .unwrap();
    assert!(local.status.success());
    let result = stdout_json(&local);
    assert_eq!(result["suggestedCompute"], "PROCESS");
    assert!(result["metadata"]["sourceBuildContext"]["applicationRuntime"].is_null());
}

#[test]
fn configured_entry_without_runtime_family_rejects_command_before_inference() {
    let input = json!({"tree":["onreza.toml","index.html"],"files":{
        "onreza.toml":"[deploy]\nentry='node server.js'\n", "index.html":"<h1>hello</h1>"
    }});
    let output = nrz()
        .args(["detect", "--stdin", "--json"])
        .write_stdin(input.to_string())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let result = stdout_json(&output);
    assert_eq!(result["code"], "APPLICATION_RUNTIME_INVALID");
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("relative file path")
    );
}
