//! End-to-end CLI tests. Commands run through pipes and expose JSON stdout.
mod support;
use predicates::str::contains;
use serde_json::json;
use std::fs;
use support::cli::{nrz, stdout_json};

#[path = "cli_integration/build.rs"]
mod build;
#[path = "cli_integration/config.rs"]
mod config;
#[path = "cli_integration/detect.rs"]
mod detect;
#[path = "cli_integration/dev.rs"]
mod dev;
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
