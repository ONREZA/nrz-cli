use clap::Parser;

use crate::cli::Cli;

#[test]
fn rollback_loads_project_config_from_requested_directory() {
    let checkout = tempfile::tempdir().unwrap();
    let project_id = uuid::Uuid::now_v7().to_string();
    std::fs::write(
        checkout.path().join("onreza.toml"),
        format!("[project]\nid = \"{project_id}\"\n"),
    )
    .unwrap();
    let cli = Cli::try_parse_from([
        "nrz",
        "rollback",
        "--dir",
        checkout.path().to_str().unwrap(),
        "--list",
    ])
    .unwrap();

    let config = super::load_initial_config(&cli.command).unwrap();

    assert_eq!(config.project.id.as_deref(), Some(project_id.as_str()));
}

#[test]
fn platform_resume_defers_config_loading_until_the_snapshot_root_is_known() {
    let checkout = tempfile::tempdir().unwrap();
    std::fs::write(checkout.path().join("onreza.toml"), "not valid toml = [").unwrap();
    let checkout = checkout.path().to_str().unwrap();

    let platform = Cli::try_parse_from([
        "nrz",
        "deploy",
        checkout,
        "--resume-deployment",
        "019b8952-ca22-76f0-b134-88becec7c629",
    ])
    .unwrap();
    let deferred = super::load_initial_config(&platform.command).unwrap();
    assert!(deferred.project.id.is_none());

    let local = Cli::try_parse_from(["nrz", "deploy", checkout]).unwrap();
    assert!(super::load_initial_config(&local.command).is_err());
}

#[test]
fn selected_deployment_requires_one_selection_and_accepts_independent_checkouts() {
    let project = "019b8952-ca22-76f0-b134-88becec7c629";
    let first = "019b8952-ca22-76f0-b134-88becec7c630";
    let second = "019b8952-ca22-76f0-b134-88becec7c631";
    let base = [
        "nrz",
        "deploy-set",
        "--project-id",
        project,
        "--environment",
        "production",
    ];
    assert!(Cli::try_parse_from(base).is_err());
    assert!(Cli::try_parse_from(base.into_iter().chain(["--all", "--service-id", first])).is_err());
    let cli = Cli::try_parse_from(base.into_iter().chain([
        "--service-id",
        first,
        "--service-id",
        second,
        "--local",
        "019b8952-ca22-76f0-b134-88becec7c630=./web",
        "--local",
        "019b8952-ca22-76f0-b134-88becec7c631=./api",
    ]))
    .unwrap();
    let crate::cli::Command::DeploySet(args) = cli.command else {
        panic!("expected selected deployment");
    };
    assert_eq!(
        args.service_ids,
        [
            first.parse::<uuid::Uuid>().unwrap(),
            second.parse::<uuid::Uuid>().unwrap()
        ]
    );
    assert_eq!(args.local.len(), 2);
    assert!(!args.all);
    let all = Cli::try_parse_from(base.into_iter().chain(["--all"])).unwrap();
    let crate::cli::Command::DeploySet(args) = all.command else {
        panic!("expected all Services deployment");
    };
    assert!(args.all);
    assert!(args.service_ids.is_empty());
}
