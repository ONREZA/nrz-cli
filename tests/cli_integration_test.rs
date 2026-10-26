//! End-to-end CLI tests. Commands run through pipes and expose JSON stdout.
#[path = "support/network_fixture.rs"]
mod network_fixture;
mod support;
use predicates::str::contains;
use serde_json::json;
use std::fs;
use support::cli::{nrz, stdout_json};

#[path = "cli_integration/build.rs"]
mod build;
#[path = "cli_integration/config.rs"]
mod config;
#[path = "cli_integration/databases.rs"]
mod databases;
#[path = "cli_integration/deployment_set.rs"]
mod deployment_set;
#[path = "cli_integration/detect.rs"]
mod detect;
#[path = "cli_integration/dev.rs"]
mod dev;
#[path = "cli_integration/domains.rs"]
mod domains;
#[path = "cli_integration/env.rs"]
mod env;
#[path = "cli_integration/fixtures.rs"]
mod fixtures;
#[path = "cli_integration/functions.rs"]
mod functions;
#[path = "cli_integration/help.rs"]
mod help;
#[path = "cli_integration/kv.rs"]
mod kv;
#[path = "cli_integration/platform.rs"]
mod platform;
#[path = "cli_integration/projects.rs"]
mod projects;
#[path = "cli_integration/rollback.rs"]
mod rollback;
#[path = "cli_integration/rules.rs"]
mod rules;

#[cfg(target_os = "linux")]
#[path = "cli_integration/terminal.rs"]
mod terminal;
