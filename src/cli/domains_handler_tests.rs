use axum::{
    Json, Router,
    routing::{get, post},
};
use nrz_api::{DomainResponse2Item, HostnameRequestBody};
use serde_json::json;
use uuid::Uuid;

use super::domains_handler::hostname_zone;

fn zone(id: u128, name: &str) -> DomainResponse2Item {
    DomainResponse2Item {
        id: Uuid::from_u128(id),
        zone_name: name.to_string(),
        ..Default::default()
    }
}

#[test]
fn hostname_selection_uses_dns_label_boundaries_and_the_most_specific_zone() {
    let zones = [
        zone(1, "example.com"),
        zone(2, "app.example.com"),
        zone(3, "xn--bcher-kva.example"),
    ];
    for (name, id, label) in [
        ("example.com", 1, "@"),
        ("www.example.com", 1, "www"),
        ("APP.EXAMPLE.COM.", 2, "@"),
        ("api.app.example.com", 2, "api"),
        ("bücher.example", 3, "@"),
    ] {
        assert_eq!(
            hostname_zone(name, &zones).unwrap(),
            (Uuid::from_u128(id), label.to_string())
        );
    }
    for invalid in ["notexample.com", "example.com.attacker.test", "127.0.0.1"] {
        assert!(
            hostname_zone(invalid, &zones).is_err(),
            "accepted {invalid}"
        );
    }
}

#[tokio::test]
async fn generated_domain_operations_use_project_listing_and_zone_relative_attachment() {
    let project = Uuid::from_u128(1);
    let zone = Uuid::from_u128(2);
    let environment = Uuid::from_u128(3);
    let app = Router::new()
        .route(&format!("/v1/domains/{project}"), get(|| async { Json(json!({"domains": []})) }))
        .route(&format!("/v1/workspace-domains/domains/{zone}/hostnames"), post(move |Json(body): Json<serde_json::Value>| async move {
            assert_eq!(body, json!({"name":"www", "projectId":project, "environmentId": environment, "redirectFromWww":false}));
            Json(json!({"hostname": {"id": Uuid::from_u128(4), "domain":"www.example.com", "dnsMode":"EXTERNAL_DNS"}}))
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = crate::api::ApiClient::with_http_client(
        format!("http://{address}"),
        reqwest::Client::new(),
    )
    .unwrap();
    assert!(
        client
            .project_domains(&project.to_string())
            .await
            .unwrap()
            .domains
            .is_empty()
    );
    let attached = client
        .attach_hostname(
            zone,
            HostnameRequestBody {
                name: "www".to_string(),
                project_id: project,
                environment_id: environment,
                redirect_from_www: Some(false),
                replace_record_ids: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(attached.hostname.domain, "www.example.com");
    server.abort();
}
