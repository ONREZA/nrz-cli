use super::config::WorkspaceConfig;
use super::workspace::resolve_workspace_context_with_config;

fn two_workspace_config() -> WorkspaceConfig {
    let mut cfg = WorkspaceConfig::empty();
    cfg.add_workspace("team-a", "nrz_aaa".into(), "Team A".into());
    cfg.add_workspace("team-b", "nrz_bbb".into(), "Team B".into());
    cfg.default_workspace = Some("team-a".into());
    cfg
}

#[test]
fn explicit_token_wins() {
    let cfg = WorkspaceConfig::empty();
    let ctx =
        resolve_workspace_context_with_config(Some("nrz_explicit"), None, &cfg, None).unwrap();
    assert_eq!(ctx.token, "nrz_explicit");
    assert!(ctx.workspace_slug.is_empty());
}

#[test]
fn explicit_workspace_from_config() {
    let cfg = two_workspace_config();
    let ctx = resolve_workspace_context_with_config(None, Some("team-b"), &cfg, None).unwrap();
    assert_eq!(ctx.token, "nrz_bbb");
    assert_eq!(ctx.workspace_slug, "team-b");
}

#[test]
fn default_workspace_used() {
    let cfg = two_workspace_config();
    let ctx = resolve_workspace_context_with_config(None, None, &cfg, None).unwrap();
    assert_eq!(ctx.token, "nrz_aaa");
    assert_eq!(ctx.workspace_slug, "team-a");
}

#[test]
fn unknown_workspace_errors() {
    let cfg = two_workspace_config();
    let result = resolve_workspace_context_with_config(None, Some("nonexistent"), &cfg, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn no_config_errors() {
    let cfg = WorkspaceConfig::empty();
    let result = resolve_workspace_context_with_config(None, None, &cfg, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not logged in"));
}

#[test]
fn single_workspace_auto_selected() {
    let mut cfg = WorkspaceConfig::empty();
    cfg.add_workspace("only-one", "nrz_only".into(), "Only One".into());
    cfg.default_workspace = None;

    let ctx = resolve_workspace_context_with_config(None, None, &cfg, None).unwrap();
    assert_eq!(ctx.token, "nrz_only");
    assert_eq!(ctx.workspace_slug, "only-one");
}

#[test]
fn project_workspace_used() {
    let cfg = two_workspace_config();
    let ctx = resolve_workspace_context_with_config(None, None, &cfg, Some("team-b")).unwrap();
    assert_eq!(ctx.token, "nrz_bbb");
    assert_eq!(ctx.workspace_slug, "team-b");
}

#[test]
fn project_workspace_ignored_if_not_in_config() {
    let cfg = two_workspace_config();
    // Project references a workspace not in config — falls through to default
    let ctx = resolve_workspace_context_with_config(None, None, &cfg, Some("unknown-ws")).unwrap();
    assert_eq!(ctx.token, "nrz_aaa"); // default
    assert_eq!(ctx.workspace_slug, "team-a");
}

#[test]
fn explicit_workspace_overrides_project_workspace() {
    let cfg = two_workspace_config();
    let ctx =
        resolve_workspace_context_with_config(None, Some("team-a"), &cfg, Some("team-b")).unwrap();
    assert_eq!(ctx.workspace_slug, "team-a");
}

#[test]
fn explicit_token_overrides_workspace() {
    let cfg = two_workspace_config();
    let ctx =
        resolve_workspace_context_with_config(Some("nrz_override"), Some("team-b"), &cfg, None)
            .unwrap();
    assert_eq!(ctx.token, "nrz_override");
    assert!(ctx.workspace_slug.is_empty());
}

#[test]
fn multiple_workspaces_no_default_errors() {
    let mut cfg = two_workspace_config();
    cfg.default_workspace = None;
    let result = resolve_workspace_context_with_config(None, None, &cfg, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not logged in"));
}
