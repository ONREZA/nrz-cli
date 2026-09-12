use super::*;
use axum::{Json, Router, extract::Query, routing::get};
use std::collections::HashMap;

async fn serve(app: Router) -> (ApiClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (client, server)
}
fn project(id: u128) -> nrz_api::Project200ResponseProject {
    nrz_api::Project200ResponseProject {
        id: uuid::Uuid::from_u128(id),
        name: format!("project-{id}"),
        display_name: Some(format!("Project {id}")),
        ..Default::default()
    }
}

#[tokio::test]
async fn interactive_choices_include_projects_after_the_first_page() {
    let app = Router::new().route(
        "/v1/projects/",
        get(|Query(query): Query<HashMap<String, String>>| async move {
            assert_eq!(query["limit"].parse::<f64>().unwrap(), 100.0);
            let offset = query["offset"].parse::<f64>().unwrap() as u128;
            let end = if offset == 0 {
                100
            } else {
                assert_eq!(offset, 100);
                101
            };
            Json(nrz_api::Project200Response {
                projects: (offset..end).map(project).collect(),
                total: 101,
            })
        }),
    );
    let (client, server) = serve(app).await;
    let projects = selection_projects(&client).await.unwrap();
    assert_eq!(projects.len(), 101);
    assert_eq!(projects.last().unwrap().project_name, "Project 100");
    server.abort();
}

#[tokio::test]
async fn project_selection_rejects_incomplete_pagination() {
    for repeated in [false, true] {
        let app = Router::new().route(
            "/v1/projects/",
            get(move || async move {
                Json(nrz_api::Project200Response {
                    projects: if repeated { vec![project(1)] } else { vec![] },
                    total: 100,
                })
            }),
        );
        let (client, server) = serve(app).await;
        assert!(
            selection_projects(&client)
                .await
                .unwrap_err()
                .to_string()
                .contains("no progress")
        );
        server.abort();
    }
}
