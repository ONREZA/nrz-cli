use anyhow::Context;
use nrz_api::*;
use uuid::Uuid;

use super::ApiClient;

impl ApiClient {
    pub async fn project_domains(&self, project_id: &str) -> anyhow::Result<DomainResponse> {
        let response = self
            .platform()?
            .get_v1domains_by_id(GetV1domainsByIdRequest {
                path: GetV1domainsByIdRequestPath {
                    id: project_id.parse().context("invalid project ID")?,
                },
            })
            .await?;
        api_success!(
            response,
            GetV1domainsByIdRequest,
            GetV1domainsByIdResponse::Ok
        )
    }

    pub async fn domain_zones(&self) -> anyhow::Result<DomainResponse2> {
        let response = self
            .platform()?
            .get_v1workspace_domains_domains(GetV1workspaceDomainsDomainsRequest {})
            .await?;
        api_success!(
            response,
            GetV1workspaceDomainsDomainsRequest,
            GetV1workspaceDomainsDomainsResponse::Ok
        )
    }

    pub async fn attach_hostname(
        &self,
        zone_id: Uuid,
        body: HostnameRequestBody,
    ) -> anyhow::Result<HostnameResponse> {
        let response = self
            .platform()?
            .post_v1workspace_domains_domains_by_zone_id_hostnames(
                PostV1workspaceDomainsDomainsByZoneIdHostnamesRequest {
                    path: PostV1workspaceDomainsDomainsByZoneIdHostnamesRequestPath { zone_id },
                    body,
                },
            )
            .await?;
        api_success!(
            response,
            PostV1workspaceDomainsDomainsByZoneIdHostnamesRequest,
            PostV1workspaceDomainsDomainsByZoneIdHostnamesResponse::Ok
        )
    }

    pub async fn detach_hostname(
        &self,
        zone_id: Uuid,
        binding_id: Uuid,
    ) -> anyhow::Result<DeletedResponse2> {
        let response = self
            .platform()?
            .delete_v1workspace_domains_domains_by_zone_id_hostnames_by_binding_id(
                DeleteV1workspaceDomainsDomainsByZoneIdHostnamesByBindingIdRequest {
                    path: DeleteV1workspaceDomainsDomainsByZoneIdHostnamesByBindingIdRequestPath {
                        zone_id,
                        binding_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1workspaceDomainsDomainsByZoneIdHostnamesByBindingIdRequest,
            DeleteV1workspaceDomainsDomainsByZoneIdHostnamesByBindingIdResponse::Ok
        )
    }

    pub async fn delete_platform_subdomain(
        &self,
        project_id: &str,
        domain_id: Uuid,
    ) -> anyhow::Result<DeletedResponse> {
        let response = self
            .platform()?
            .delete_v1domains_by_id_platform_subdomains_by_domain_id(
                DeleteV1domainsByIdPlatformSubdomainsByDomainIdRequest {
                    path: DeleteV1domainsByIdPlatformSubdomainsByDomainIdRequestPath {
                        id: project_id.parse().context("invalid project ID")?,
                        domain_id,
                    },
                },
            )
            .await?;
        api_success!(
            response,
            DeleteV1domainsByIdPlatformSubdomainsByDomainIdRequest,
            DeleteV1domainsByIdPlatformSubdomainsByDomainIdResponse::Ok
        )
    }

    pub async fn verify_domain_zone(&self, zone_id: Uuid) -> anyhow::Result<Verify200Response> {
        let response = self
            .platform()?
            .post_v1workspace_domains_domains_by_zone_id_verify(
                PostV1workspaceDomainsDomainsByZoneIdVerifyRequest {
                    path: PostV1workspaceDomainsDomainsByZoneIdVerifyRequestPath { zone_id },
                },
            )
            .await?;
        api_success!(
            response,
            PostV1workspaceDomainsDomainsByZoneIdVerifyRequest,
            PostV1workspaceDomainsDomainsByZoneIdVerifyResponse::Ok
        )
    }
}
