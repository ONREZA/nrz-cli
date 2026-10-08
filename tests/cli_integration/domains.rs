use super::{nrz, stdout_json, support};
use axum::{
    Json, Router, extract,
    http::HeaderMap,
    routing::{delete, get, post},
};
use nrz_api::{DomainResponseItem, DomainResponseItemManagedDnsZone};
use serde_json::{Value, json};
use uuid::Uuid;

const PROJECT: &str = "00000000-0000-0000-0000-000000000001";

fn binding(id: u128, name: &str, zone_id: Option<u128>) -> Value {
    serde_json::to_value(DomainResponseItem {
        id: Uuid::from_u128(id),
        domain: name.to_string(),
        dns_status: nrz_api::DomainResponseItemDnsStatus::Validated,
        tls_status: nrz_api::DomainResponseItemTlsStatus::Issued,
        dns_mode: if zone_id.is_some() {
            nrz_api::DomainResponseItemDnsMode::ManagedNs
        } else {
            nrz_api::DomainResponseItemDnsMode::PlatformSubdomain
        },
        managed_dns_zone: zone_id.map(|id| DomainResponseItemManagedDnsZone {
            id: Uuid::from_u128(id),
            zone_name: "example.com".to_string(),
            ..Default::default()
        }),
        environment: nrz_api::DomainResponseItemEnvironment {
            name: "production".to_string(),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap()
}

fn listing(response: Value) -> Router {
    Router::new().route(
        "/v1/domains/{project_id}",
        get(
            move |extract::Path(project): extract::Path<String>, headers: HeaderMap| {
                let response = response.clone();
                async move {
                    assert_eq!(project, PROJECT);
                    assert_eq!(headers["X-API-Key"], "test-token");
                    Json(response)
                }
            },
        ),
    )
}

fn domain_command(api_url: &str, args: &[&str], mode: &str, color: bool) -> std::process::Output {
    let temp = tempfile::tempdir().unwrap();
    let mut command = nrz();
    command
        .current_dir(&temp)
        .env("NRZ_API_URL", api_url)
        .env("NRZ_HUMAN", "false")
        .env("CLICOLOR_FORCE", if color { "1" } else { "0" })
        .env("CLICOLOR", if color { "1" } else { "0" })
        .args([
            mode,
            "--token",
            "test-token",
            "domains",
            "--project-id",
            PROJECT,
        ])
        .args(args);
    if color {
        command.env_remove("NO_COLOR");
    } else {
        command.env("NO_COLOR", "1");
    }
    command.output().unwrap()
}

#[test]
fn domains_list_preserves_json_and_plain_and_colored_statuses() {
    let mut response = json!({"domains": [
        binding(2, "validated.example.com", Some(20)),
        binding(3, "pending.example.com", Some(30)),
        binding(4, "failed.example.com", Some(40)),
        binding(5, "validating.example.com", Some(50)),
    ]});
    for (index, dns, tls) in [
        (1, "PENDING", "PENDING"),
        (2, "FAILED", "FAILED"),
        (3, "VALIDATING", "EXPIRED"),
    ] {
        response["domains"][index]["dnsStatus"] = json!(dns);
        response["domains"][index]["tlsStatus"] = json!(tls);
    }
    let api_url = support::api_mock::spawn(listing(response.clone()));
    for (mode, color) in [("--json", false), ("--human", false), ("--human", true)] {
        let output = domain_command(&api_url, &["list"], mode, color);
        assert!(output.status.success(), "{output:?}");
        if mode == "--json" {
            assert_eq!(stdout_json(&output), response);
            assert!(output.stderr.is_empty());
        } else {
            assert!(output.stdout.is_empty());
            let text = String::from_utf8(output.stderr).unwrap();
            if color {
                for (status, code) in [
                    ("validated", 32),
                    ("issued", 32),
                    ("pending", 33),
                    ("failed", 31),
                ] {
                    assert!(
                        text.contains(&format!("\x1b[{code}m{status}\x1b[0m")),
                        "{text:?}"
                    );
                }
                assert!(text.contains("validating   expired"), "{text:?}");
            } else {
                let mut expected = format!(
                    "\n  {:<40} {:<12} {:<10} {}\n  {}\n",
                    "Domain",
                    "DNS",
                    "TLS",
                    "Environment",
                    "-".repeat(75)
                );
                for (name, dns, tls) in [
                    ("validated.example.com", "validated", "issued"),
                    ("pending.example.com", "pending", "pending"),
                    ("failed.example.com", "failed", "failed"),
                    ("validating.example.com", "validating", "expired"),
                ] {
                    expected.push_str(&format!("  {name:<40} {dns:<12} {tls:<10} production\n"));
                }
                expected.push('\n');
                assert_eq!(text, expected);
            }
        }
    }
    let api_url = support::api_mock::spawn(listing(json!({"domains":[]})));
    let output = domain_command(&api_url, &["list"], "--human", false);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "  No custom domains found.\n"
    );
}

#[test]
fn domains_remove_routes_the_selected_managed_or_platform_binding() {
    for managed in [true, false] {
        let target = Uuid::from_u128(3).to_string();
        let response = json!({"domains": [
            binding(2, "other.example.com", Some(20)),
            binding(3, "target.example.com", managed.then_some(30)),
        ]});
        let route = if managed {
            format!(
                "/v1/workspace-domains/domains/{}/hostnames/{target}",
                Uuid::from_u128(30)
            )
        } else {
            format!("/v1/domains/{PROJECT}/platform-subdomains/{target}")
        };
        let api_url = support::api_mock::spawn(listing(response).route(
            &route,
            delete(|headers: HeaderMap| async move {
                assert_eq!(headers["X-API-Key"], "test-token");
                Json(json!({"deleted":true}))
            }),
        ));
        for mode in ["--json", "--human"] {
            let output = domain_command(&api_url, &["remove", &target], mode, false);
            assert!(output.status.success(), "{output:?}");
            if mode == "--json" {
                assert_eq!(
                    stdout_json(&output),
                    json!({"id":target,"status":"deleted"})
                );
                assert!(output.stderr.is_empty());
            } else {
                assert!(output.stdout.is_empty());
                assert_eq!(
                    String::from_utf8(output.stderr).unwrap(),
                    "  ✓ Domain removed.\n"
                );
            }
        }
    }
}

#[test]
fn domains_verify_uses_selected_zone_and_reports_delegation_or_queue() {
    let target = Uuid::from_u128(3).to_string();
    let zone = Uuid::from_u128(30);
    for delegated in [Some(true), Some(false), None] {
        let delegation = delegated
            .map(|delegated| json!({"delegated":delegated,"expected":["ns1.example.com"]}));
        let server_delegation = delegation.clone();
        let route = format!("/v1/workspace-domains/domains/{zone}/verify");
        let api_url = support::api_mock::spawn(
            listing(json!({"domains": [
                binding(2, "other.example.com", Some(20)),
                binding(3, "target.example.com", Some(30)),
            ]}))
            .route(
                &route,
                post(move |headers: HeaderMap| {
                    let delegation = server_delegation.clone();
                    async move {
                        assert_eq!(headers["X-API-Key"], "test-token");
                        Json(json!({"requeued":2,"delegation":delegation}))
                    }
                }),
            ),
        );
        for mode in ["--json", "--human"] {
            let output = domain_command(&api_url, &["verify", &target], mode, false);
            assert!(output.status.success(), "{output:?}");
            if mode == "--json" {
                assert_eq!(
                    stdout_json(&output),
                    json!({"id":target,"domain":"target.example.com","zoneId":zone,"requeued":2,"delegation":delegation})
                );
                assert!(output.stderr.is_empty());
            } else {
                assert!(output.stdout.is_empty());
                assert_eq!(
                    String::from_utf8(output.stderr).unwrap(),
                    if delegated == Some(true) {
                        "  ✓ Domain delegation verified.\n"
                    } else {
                        "  ! Domain verification queued. Check your DNS records.\n"
                    }
                );
            }
        }
    }
}

#[test]
fn domains_refuse_missing_invalid_or_unmanaged_bindings_and_unknown_wire_status() {
    let mut response = json!({"domains":[binding(2, "unmanaged.example.com", None)]});
    response["domains"][0]["dnsMode"] = json!("EXTERNAL_DNS");
    let api_url = support::api_mock::spawn(listing(response.clone()));
    let present = Uuid::from_u128(2).to_string();
    let missing = Uuid::from_u128(3).to_string();
    for (command, id, error) in [
        ("remove", missing.as_str(), "domain not found in project"),
        ("verify", missing.as_str(), "domain not found in project"),
        ("remove", present.as_str(), "reconnect it before removal"),
        (
            "verify",
            present.as_str(),
            "domain is not attached to a workspace domain",
        ),
        ("remove", "invalid/../binding", "invalid domain binding ID"),
        ("verify", "invalid/../binding", "invalid domain binding ID"),
    ] {
        let output = domain_command(&api_url, &[command, id], "--json", false);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            stdout_json(&output)["error"]
                .as_str()
                .unwrap()
                .contains(error),
            "{output:?}"
        );
    }
    response["domains"][0]["dnsStatus"] = json!("FUTURE_STATUS");
    let api_url = support::api_mock::spawn(listing(response));
    let output = domain_command(&api_url, &["list"], "--json", false);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout_json(&output)["error"]
            .as_str()
            .unwrap()
            .contains("failed to fetch domains")
    );
}

#[test]
fn domains_add_resolves_environment_and_attaches_zone_relative_hostname() {
    let zone = Uuid::from_u128(20);
    let environment = Uuid::from_u128(30);
    let attachment = Uuid::from_u128(40);
    let app = Router::new()
        .route(&format!("/v1/projects/{PROJECT}/execution-context/resolve"), post(move |Json(body): Json<Value>| async move {
            assert_eq!(body, json!({"environment":"preview","sourceRef":null,"selectionSource":"EXPLICIT"}));
            Json(json!({"protocolVersion":"execution-context-v2","context":{
                "workspaceId":Uuid::from_u128(10),"workspaceSlug":"team","projectId":PROJECT,"projectName":"app",
                "environmentId":environment,"environmentName":"preview","environmentType":"PREVIEW",
                "sourceRef":null,"selectionSource":"EXPLICIT"
            }}))
        }))
        .route("/v1/workspace-domains/domains", get(move || async move {
            Json(nrz_api::DomainResponse2 {
                domains: vec![nrz_api::DomainResponse2Item {
                    id:zone, zone_name:"example.com".to_string(), ..Default::default()
                }], ..Default::default()
            })
        }))
        .route(&format!("/v1/workspace-domains/domains/{zone}/hostnames"), post(move |headers: HeaderMap, Json(body): Json<Value>| async move {
            assert_eq!(headers["X-API-Key"], "test-token");
            assert_eq!(body,json!({"name":"www","projectId":PROJECT,"environmentId":environment,"redirectFromWww":false}));
            Json(json!({"hostname":{"id":attachment,"domain":"www.example.com","dnsMode":"EXTERNAL_DNS"}}))
        }));
    let api_url = support::api_mock::spawn(app);
    for mode in ["--json", "--human"] {
        let output = domain_command(
            &api_url,
            &["add", "WWW.EXAMPLE.COM.", "--environment", "preview"],
            mode,
            false,
        );
        assert!(output.status.success(), "{output:?}");
        if mode == "--json" {
            assert_eq!(
                stdout_json(&output),
                json!({"id":attachment,"domain":"www.example.com","dnsMode":"EXTERNAL_DNS"})
            );
            assert!(output.stderr.is_empty());
        } else {
            assert!(output.stdout.is_empty());
            assert_eq!(
                String::from_utf8(output.stderr).unwrap(),
                "  ✓ Added domain www.example.com\n"
            );
        }
    }
}
