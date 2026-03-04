use super::output::Phase;

#[test]
fn phase_as_str_all_variants() {
    assert_eq!(Phase::Auth.as_str(), "auth");
    assert_eq!(Phase::Build.as_str(), "build");
    assert_eq!(Phase::Db.as_str(), "db");
    assert_eq!(Phase::Deploy.as_str(), "deploy");
    assert_eq!(Phase::Detect.as_str(), "detect");
    assert_eq!(Phase::Domains.as_str(), "domains");
    assert_eq!(Phase::Env.as_str(), "env");
    assert_eq!(Phase::Init.as_str(), "init");
    assert_eq!(Phase::Install.as_str(), "install");
    assert_eq!(Phase::Link.as_str(), "link");
    assert_eq!(Phase::Projects.as_str(), "projects");
    assert_eq!(Phase::Rollback.as_str(), "rollback");
    assert_eq!(Phase::Workspace.as_str(), "workspace");
}

#[test]
fn phase_display_matches_as_str() {
    let phases = [
        Phase::Auth,
        Phase::Build,
        Phase::Db,
        Phase::Deploy,
        Phase::Detect,
        Phase::Domains,
        Phase::Env,
        Phase::Init,
        Phase::Install,
        Phase::Link,
        Phase::Projects,
        Phase::Rollback,
        Phase::Workspace,
    ];
    for p in phases {
        assert_eq!(format!("{p}"), p.as_str());
    }
}
