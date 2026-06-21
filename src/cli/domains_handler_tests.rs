use super::domains_handler::{
    project_domain_delete_url, workspace_hostname_delete_url, workspace_zone_verify_url,
};

#[test]
fn workspace_domain_urls_use_workspace_api() {
    assert_eq!(
        workspace_hostname_delete_url("zone-1", "domain-1"),
        "/v1/workspace-domains/domains/zone-1/hostnames/domain-1"
    );
    assert_eq!(
        workspace_zone_verify_url("zone-1"),
        "/v1/workspace-domains/domains/zone-1/verify"
    );
    assert_eq!(
        project_domain_delete_url("project-1", "domain-1"),
        "/v1/domains/project-1/platform-subdomains/domain-1"
    );
}
