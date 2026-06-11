use super::environment::{ProjectEnvironment, resolve_environment_from_list};

fn env(id: &str, env_type: &str, name: &str) -> ProjectEnvironment {
    ProjectEnvironment {
        id: id.to_string(),
        env_type: env_type.to_string(),
        name: name.to_string(),
    }
}

#[test]
fn resolve_environment_defaults_to_production() {
    let environments = vec![
        env("env-prev", "PREVIEW", "Preview"),
        env("env-prod", "PRODUCTION", "Production"),
    ];

    let resolved = resolve_environment_from_list(&environments, None).unwrap();

    assert_eq!(resolved.id, "env-prod");
}

#[test]
fn resolve_environment_accepts_type_name_or_id() {
    let environments = vec![
        env("env-prod", "PRODUCTION", "Production"),
        env("env-stage", "CUSTOM", "stage"),
    ];

    assert_eq!(
        resolve_environment_from_list(&environments, Some("prod"))
            .unwrap()
            .id,
        "env-prod"
    );
    assert_eq!(
        resolve_environment_from_list(&environments, Some("stage"))
            .unwrap()
            .id,
        "env-stage"
    );
    assert_eq!(
        resolve_environment_from_list(&environments, Some("env-stage"))
            .unwrap()
            .id,
        "env-stage"
    );
}
