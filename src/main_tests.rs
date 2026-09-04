use clap::Parser;

use crate::cli::Cli;

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
