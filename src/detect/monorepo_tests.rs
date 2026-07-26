use super::fs::VirtualFs;
use super::monorepo::*;
use super::package_json::PackageJson;
use super::types::{MonorepoPackage, MonorepoTool, PackageManagerInfo, PackageManagerType};

// ── parse_pnpm_workspace ──────────────────────────────────────

#[test]
fn parse_pnpm_workspace_basic() {
    let content = "packages:\n  - 'apps/*'\n  - 'packages/*'\n";
    let patterns = parse_pnpm_workspace(content);
    assert_eq!(patterns, vec!["apps/*", "packages/*"]);
}

#[test]
fn parse_pnpm_workspace_double_quotes() {
    let content = "packages:\n  - \"apps/*\"\n  - \"libs/*\"\n";
    let patterns = parse_pnpm_workspace(content);
    assert_eq!(patterns, vec!["apps/*", "libs/*"]);
}

#[test]
fn parse_pnpm_workspace_no_quotes() {
    let content = "packages:\n  - apps/*\n  - packages/*\n";
    let patterns = parse_pnpm_workspace(content);
    assert_eq!(patterns, vec!["apps/*", "packages/*"]);
}

#[test]
fn parse_pnpm_workspace_with_comments() {
    let content = "# workspace config\npackages:\n  # apps\n  - 'apps/*'\n  - 'packages/*'\n";
    let patterns = parse_pnpm_workspace(content);
    assert_eq!(patterns, vec!["apps/*", "packages/*"]);
}

#[test]
fn parse_pnpm_workspace_empty() {
    let content = "packages:\n";
    let patterns = parse_pnpm_workspace(content);
    assert!(patterns.is_empty());
}

#[test]
fn parse_pnpm_workspace_other_keys() {
    let content = "packages:\n  - 'apps/*'\ncatalog:\n  react: ^18\n";
    let patterns = parse_pnpm_workspace(content);
    assert_eq!(patterns, vec!["apps/*"]);
}

// ── detect_monorepo ───────────────────────────────────────────

fn vfs(json: &str) -> VirtualFs {
    VirtualFs::from_json(json).unwrap()
}

fn pm(pm_type: PackageManagerType) -> Option<PackageManagerInfo> {
    Some(PackageManagerInfo {
        pm_type,
        version: None,
        lockfile: None,
    })
}

#[test]
fn detect_pnpm_monorepo() {
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/", "packages/", "packages/ui/"],
        "files": {
            "pnpm-workspace.yaml": "packages:\n  - 'apps/*'\n  - 'packages/*'\n",
            "package.json": "{\"name\": \"root\"}",
            "apps/web/package.json": "{\"name\": \"@my/web\"}",
            "packages/ui/package.json": "{\"name\": \"@my/ui\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.tool, MonorepoTool::Pnpm);
    assert_eq!(info.workspaces, vec!["apps/*", "packages/*"]);
    assert_eq!(info.packages.len(), 2);
    assert_eq!(info.packages[0].path, "apps/web");
    assert_eq!(info.packages[0].name.as_deref(), Some("@my/web"));
    assert_eq!(info.packages[1].path, "packages/ui");
}

#[test]
fn detect_recursive_workspace_glob() {
    let fs = vfs(r#"{
        "tree": [
            "packages/",
            "packages/direct/",
            "packages/group/",
            "packages/group/shared/"
        ],
        "files": {
            "pnpm-workspace.yaml": "packages:\n  - 'packages/**'\n",
            "package.json": "{\"name\": \"root\"}",
            "packages/direct/package.json": "{\"name\": \"direct\"}",
            "packages/group/shared/package.json": "{\"name\": \"shared\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(
        info.packages
            .iter()
            .map(|package| package.path.as_str())
            .collect::<Vec<_>>(),
        vec!["packages/direct", "packages/group/shared"]
    );
}

#[test]
fn single_star_workspace_glob_does_not_cross_directories() {
    let fs = vfs(r#"{
        "tree": [
            "packages/",
            "packages/direct/",
            "packages/group/",
            "packages/group/nested/"
        ],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"packages/*\"]}",
            "packages/direct/package.json": "{\"name\": \"direct\"}",
            "packages/group/nested/package.json": "{\"name\": \"nested\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].path, "packages/direct");
}

#[test]
fn recursive_workspace_glob_respects_negations() {
    let fs = vfs(r#"{
        "tree": [
            "packages/",
            "packages/public/",
            "packages/public/shared/",
            "packages/internal/",
            "packages/internal/private/"
        ],
        "files": {
            "pnpm-workspace.yaml": "packages:\n  - 'packages/**'\n  - '!packages/internal/**'\n",
            "package.json": "{\"name\": \"root\"}",
            "packages/public/shared/package.json": "{\"name\": \"shared\"}",
            "packages/internal/private/package.json": "{\"name\": \"private\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].path, "packages/public/shared");
}

#[test]
fn recursive_workspace_glob_respects_unanchored_test_negation() {
    let fs = vfs(r#"{
        "tree": [
            "components/",
            "components/button/",
            "components/group/",
            "components/group/test/",
            "components/group/test/fixture/"
        ],
        "files": {
            "pnpm-workspace.yaml": "packages:\n  - 'components/**'\n  - '!**/test/**'\n",
            "package.json": "{\"name\": \"root\"}",
            "components/button/package.json": "{\"name\": \"button\"}",
            "components/group/test/fixture/package.json": "{\"name\": \"fixture\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].path, "components/button");
}

#[test]
fn workspace_glob_matching_work_is_bounded() {
    let value = "a".repeat(1024);
    let pattern = vec!['*'; 4096];
    let mut budget = WorkspaceExpansionBudget::default();

    assert!(!workspace_segment_matches(&value, &pattern, &mut budget));
}

#[test]
fn workspace_glob_matching_uses_a_shared_budget() {
    let value = "a".repeat(1000);
    let pattern = vec!['*'; 3000];
    let mut budget = WorkspaceExpansionBudget::default();

    assert!(workspace_segment_matches(&value, &pattern, &mut budget));
    assert!(!workspace_segment_matches(&value, &pattern, &mut budget));
}

#[test]
fn exhausted_workspace_expansion_budget_fails_closed() {
    let directory = "a".repeat(1000);
    let pattern = "*".repeat(3000);
    let root_package = serde_json::json!({
        "name": "root",
        "workspaces": [pattern.clone(), pattern]
    })
    .to_string();
    let mut files = serde_json::Map::new();
    files.insert("package.json".to_string(), root_package.into());
    files.insert(
        format!("{directory}/package.json"),
        r#"{"name":"workspace-package"}"#.into(),
    );
    let manifest = serde_json::json!({
        "tree": [format!("{directory}/")],
        "files": files
    });
    let fs = vfs(&manifest.to_string());
    let pkg = PackageJson::load_from_fs(&fs);

    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert!(info.packages.is_empty());
}

#[test]
fn detect_npm_workspaces_monorepo() {
    let fs = vfs(r#"{
        "tree": ["packages/", "packages/core/"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"packages/*\"]}",
            "packages/core/package.json": "{\"name\": \"core\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.tool, MonorepoTool::Npm);
    assert_eq!(info.workspaces, vec!["packages/*"]);
    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].name.as_deref(), Some("core"));
}

#[test]
fn detect_yarn_workspaces_monorepo() {
    let fs = vfs(r#"{
        "tree": ["packages/", "packages/core/", "yarn.lock"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"packages/*\"]}",
            "packages/core/package.json": "{\"name\": \"core\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let pm_info = pm(PackageManagerType::Yarn);
    let info = detect_monorepo(&fs, pkg.as_ref(), pm_info.as_ref()).unwrap();

    assert_eq!(info.tool, MonorepoTool::Yarn);
}

#[test]
fn detect_bun_workspaces_monorepo() {
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/", "bun.lock"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}",
            "apps/web/package.json": "{\"name\": \"web\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let pm_info = pm(PackageManagerType::Bun);
    let info = detect_monorepo(&fs, pkg.as_ref(), pm_info.as_ref()).unwrap();

    assert_eq!(info.tool, MonorepoTool::Bun);
}

#[test]
fn detect_turborepo_upgrades_tool() {
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/", "turbo.json"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}",
            "apps/web/package.json": "{\"name\": \"web\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.tool, MonorepoTool::Turbo);
}

#[test]
fn detect_nx_upgrades_tool() {
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/", "nx.json"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}",
            "apps/web/package.json": "{\"name\": \"web\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.tool, MonorepoTool::Nx);
}

#[test]
fn turbo_takes_priority_over_nx() {
    let fs = vfs(r#"{
        "tree": ["turbo.json", "nx.json"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.tool, MonorepoTool::Turbo);
}

#[test]
fn turbo_overrides_yarn_base_tool() {
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/", "turbo.json"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}",
            "apps/web/package.json": "{\"name\": \"web\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let pm_info = pm(PackageManagerType::Yarn);
    let info = detect_monorepo(&fs, pkg.as_ref(), pm_info.as_ref()).unwrap();

    // turbo.json overrides Yarn → Turbo
    assert_eq!(info.tool, MonorepoTool::Turbo);
}

#[test]
fn pnpm_workspace_takes_priority_over_package_json() {
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/", "packages/", "packages/core/"],
        "files": {
            "pnpm-workspace.yaml": "packages:\n  - 'apps/*'\n  - 'packages/*'\n",
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}",
            "apps/web/package.json": "{\"name\": \"web\"}",
            "packages/core/package.json": "{\"name\": \"core\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.tool, MonorepoTool::Pnpm);
    // pnpm-workspace.yaml patterns are used, not package.json
    assert_eq!(info.workspaces, vec!["apps/*", "packages/*"]);
    assert_eq!(info.packages.len(), 2);
}

#[test]
fn pnpm_workspace_ignores_pm_type() {
    // Even if PM is detected as npm, pnpm-workspace.yaml means Pnpm
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/"],
        "files": {
            "pnpm-workspace.yaml": "packages:\n  - 'apps/*'\n",
            "package.json": "{\"name\": \"root\"}",
            "apps/web/package.json": "{\"name\": \"web\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let pm_info = pm(PackageManagerType::Npm);
    let info = detect_monorepo(&fs, pkg.as_ref(), pm_info.as_ref()).unwrap();

    assert_eq!(info.tool, MonorepoTool::Pnpm);
}

#[test]
fn not_a_monorepo() {
    let fs = vfs(r#"{
        "tree": ["src/"],
        "files": {
            "package.json": "{\"name\": \"simple-app\", \"dependencies\": {\"next\": \"14.0.0\"}}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None);

    assert!(info.is_none());
}

#[test]
fn no_package_json() {
    let fs = vfs(r#"{"tree": [], "files": {}}"#);
    let info = detect_monorepo(&fs, None, None);
    assert!(info.is_none());
}

// ── resolve_app ───────────────────────────────────────────────

fn make_info(packages: Vec<MonorepoPackage>) -> super::types::MonorepoInfo {
    super::types::MonorepoInfo {
        tool: MonorepoTool::Pnpm,
        workspaces: vec!["apps/*".to_string()],
        packages,
    }
}

#[test]
fn resolve_app_by_name() {
    let info = make_info(vec![
        MonorepoPackage {
            name: Some("@my/web".to_string()),
            path: "apps/web".to_string(),
        },
        MonorepoPackage {
            name: Some("@my/api".to_string()),
            path: "apps/api".to_string(),
        },
    ]);

    assert_eq!(
        resolve_app(&info, "@my/web").unwrap(),
        Some("apps/web".to_string())
    );
    assert_eq!(
        resolve_app(&info, "@my/api").unwrap(),
        Some("apps/api".to_string())
    );
}

#[test]
fn resolve_app_by_dirname() {
    let info = make_info(vec![MonorepoPackage {
        name: Some("@scope/web-app".to_string()),
        path: "apps/web".to_string(),
    }]);

    assert_eq!(
        resolve_app(&info, "web").unwrap(),
        Some("apps/web".to_string())
    );
}

#[test]
fn resolve_app_by_path() {
    let info = make_info(vec![MonorepoPackage {
        name: None,
        path: "apps/web".to_string(),
    }]);

    assert_eq!(
        resolve_app(&info, "apps/web").unwrap(),
        Some("apps/web".to_string())
    );
}

#[test]
fn resolve_app_name_over_dirname() {
    // If name matches, it should resolve even if dirname differs
    let info = make_info(vec![MonorepoPackage {
        name: Some("web".to_string()),
        path: "apps/frontend".to_string(),
    }]);

    assert_eq!(
        resolve_app(&info, "web").unwrap(),
        Some("apps/frontend".to_string())
    );
}

#[test]
fn resolve_app_not_found() {
    let info = make_info(vec![MonorepoPackage {
        name: Some("web".to_string()),
        path: "apps/web".to_string(),
    }]);

    assert_eq!(resolve_app(&info, "missing").unwrap(), None);
}

// ── exact path pattern ───────────────────────────────────────

#[test]
fn detect_exact_path_workspace() {
    let fs = vfs(r#"{
        "tree": ["packages/", "packages/core/"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"packages/core\"]}",
            "packages/core/package.json": "{\"name\": \"@my/core\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].path, "packages/core");
    assert_eq!(info.packages[0].name.as_deref(), Some("@my/core"));
}

#[test]
fn discover_workspace_root_for_nested_pnpm_app() {
    let workspace = tempfile::tempdir().unwrap();
    let app = workspace.path().join("apps/group/web");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(
        workspace.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'apps/**'\n",
    )
    .unwrap();
    std::fs::write(workspace.path().join("package.json"), r#"{"private":true}"#).unwrap();
    std::fs::write(app.join("package.json"), r#"{"name":"web"}"#).unwrap();

    assert_eq!(discover_workspace_root(&app), workspace.path());
}

#[test]
fn discover_workspace_root_ignores_undeclared_ancestor() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("packages/web");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        workspace.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'apps/*'\n",
    )
    .unwrap();
    std::fs::write(workspace.path().join("package.json"), r#"{"private":true}"#).unwrap();
    std::fs::write(project.join("package.json"), r#"{"name":"web"}"#).unwrap();

    assert_eq!(discover_workspace_root(&project), project);
}

// ── negation patterns ─────────────────────────────────────────

#[test]
fn negation_patterns_filter_packages() {
    let fs = vfs(r#"{
        "tree": ["packages/", "packages/core/", "packages/internal/"],
        "files": {
            "pnpm-workspace.yaml": "packages:\n  - 'packages/*'\n  - '!packages/internal'\n",
            "package.json": "{\"name\": \"root\"}",
            "packages/core/package.json": "{\"name\": \"core\"}",
            "packages/internal/package.json": "{\"name\": \"internal\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    // Negation pattern filters out packages/internal
    assert_eq!(info.workspaces.len(), 2);
    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].path, "packages/core");
}

// ── dedup & edge cases ────────────────────────────────────────

#[test]
fn overlapping_patterns_deduplicated() {
    let fs = vfs(r#"{
        "tree": ["packages/", "packages/core/"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"packages/*\", \"packages/core\"]}",
            "packages/core/package.json": "{\"name\": \"core\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].name.as_deref(), Some("core"));
}

#[test]
fn workspaces_object_format_through_detect() {
    // Yarn legacy format: {"workspaces": {"packages": ["packages/*"]}}
    let fs = vfs(r#"{
        "tree": ["packages/", "packages/core/"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": {\"packages\": [\"packages/*\"]}}",
            "packages/core/package.json": "{\"name\": \"core\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let pm_info = pm(PackageManagerType::Yarn);
    let info = detect_monorepo(&fs, pkg.as_ref(), pm_info.as_ref()).unwrap();

    assert_eq!(info.tool, MonorepoTool::Yarn);
    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].name.as_deref(), Some("core"));
}

#[test]
fn directory_without_package_json_skipped() {
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/", "apps/misc/"],
        "files": {
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}",
            "apps/web/package.json": "{\"name\": \"web\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    // apps/misc has no package.json → not a workspace package
    assert_eq!(info.packages.len(), 1);
    assert_eq!(info.packages[0].path, "apps/web");
}

#[test]
fn resolve_app_with_empty_packages() {
    let info = make_info(vec![]);
    assert_eq!(resolve_app(&info, "anything").unwrap(), None);
}

#[test]
fn resolve_app_rejects_ambiguous_directory_name() {
    let info = make_info(vec![
        MonorepoPackage {
            name: Some("@site/web".to_string()),
            path: "apps/web".to_string(),
        },
        MonorepoPackage {
            name: Some("@example/web".to_string()),
            path: "examples/web".to_string(),
        },
    ]);

    assert_eq!(
        resolve_app(&info, "web").unwrap_err(),
        vec!["apps/web".to_string(), "examples/web".to_string()]
    );
}

#[test]
fn pnpm_workspace_empty_fallback_to_package_json() {
    // pnpm-workspace.yaml exists but has no patterns — fallback to package.json workspaces
    let fs = vfs(r#"{
        "tree": ["apps/", "apps/web/"],
        "files": {
            "pnpm-workspace.yaml": "catalog:\n  react: ^18\n",
            "package.json": "{\"name\": \"root\", \"workspaces\": [\"apps/*\"]}",
            "apps/web/package.json": "{\"name\": \"web\"}"
        }
    }"#);

    let pkg = PackageJson::load_from_fs(&fs);
    let info = detect_monorepo(&fs, pkg.as_ref(), None).unwrap();

    // Falls back to package.json workspaces → Npm (no PM detected)
    assert_eq!(info.tool, MonorepoTool::Npm);
    assert_eq!(info.packages.len(), 1);
}

// ── MonorepoTool display ──────────────────────────────────────

#[test]
fn monorepo_tool_display() {
    assert_eq!(format!("{}", MonorepoTool::Npm), "npm workspaces");
    assert_eq!(format!("{}", MonorepoTool::Yarn), "yarn workspaces");
    assert_eq!(format!("{}", MonorepoTool::Pnpm), "pnpm workspaces");
    assert_eq!(format!("{}", MonorepoTool::Bun), "bun workspaces");
    assert_eq!(format!("{}", MonorepoTool::Turbo), "turborepo");
    assert_eq!(format!("{}", MonorepoTool::Nx), "nx");
}
