use super::ignored_build::{
    IgnoredBuildOutcome, IgnoredBuildRequest, evaluate_for_test, remove_git_repository_environment,
};
use nrz::config::{IgnoredBuildBehavior, ProjectBuildSettings};
use std::process::Command;
use std::time::Duration;

fn settings(behavior: IgnoredBuildBehavior) -> ProjectBuildSettings {
    ProjectBuildSettings {
        ignored_build_behavior: Some(behavior),
        ..ProjectBuildSettings::default()
    }
}

async fn evaluate(
    settings: &ProjectBuildSettings,
    project_dir: &std::path::Path,
    environment_type: &str,
    execution_env: &[(String, String)],
) -> anyhow::Result<IgnoredBuildOutcome> {
    evaluate_for_test(
        IgnoredBuildRequest {
            settings,
            environment_type,
            project_dir,
            execution_env,
            json: true,
            build_logs: None,
        },
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
    .await
}

#[tokio::test]
async fn custom_allowlist_uses_materialized_branch_environment() {
    let project = tempfile::tempdir().unwrap();
    let mut settings = settings(IgnoredBuildBehavior::Custom);
    settings.ignored_build_command = Some(
        "if [ \"$ONREZA_GIT_BRANCH\" = \"main\" ] || [ \"$ONREZA_GIT_BRANCH\" = \"test\" ]; then exit 1; else exit 0; fi"
            .to_string(),
    );

    let main_env = vec![("ONREZA_GIT_BRANCH".to_string(), "main".to_string())];
    assert_eq!(
        evaluate(&settings, project.path(), "PRODUCTION", &main_env)
            .await
            .unwrap(),
        IgnoredBuildOutcome::Continue {
            reason: "Ignored Build Step returned exit code 1; continuing build".to_string()
        }
    );

    let feature_env = vec![(
        "ONREZA_GIT_BRANCH".to_string(),
        "feature/not-allowed".to_string(),
    )];
    assert_eq!(
        evaluate(&settings, project.path(), "PREVIEW", &feature_env)
            .await
            .unwrap(),
        IgnoredBuildOutcome::Skip {
            reason: "Ignored Build Step returned exit code 0".to_string()
        }
    );
}

#[tokio::test]
async fn built_in_environment_policies_do_not_run_a_shell_command() {
    let project = tempfile::tempdir().unwrap();
    let production = settings(IgnoredBuildBehavior::OnlyProduction);
    assert_eq!(
        evaluate(&production, project.path(), "PRODUCTION", &[])
            .await
            .unwrap(),
        IgnoredBuildOutcome::Continue {
            reason: "Ignored Build Step allows production deployments; continuing build"
                .to_string()
        }
    );
    assert!(matches!(
        evaluate(&production, project.path(), "PREVIEW", &[])
            .await
            .unwrap(),
        IgnoredBuildOutcome::Skip { .. }
    ));

    let preview = settings(IgnoredBuildBehavior::OnlyPreview);
    assert_eq!(
        evaluate(&preview, project.path(), "PREVIEW", &[])
            .await
            .unwrap(),
        IgnoredBuildOutcome::Continue {
            reason: "Ignored Build Step allows preview deployments; continuing build".to_string()
        }
    );
    assert!(matches!(
        evaluate(&preview, project.path(), "CUSTOM", &[])
            .await
            .unwrap(),
        IgnoredBuildOutcome::Continue { .. }
    ));
    assert!(matches!(
        evaluate(&preview, project.path(), "PRODUCTION", &[])
            .await
            .unwrap(),
        IgnoredBuildOutcome::Skip { .. }
    ));

    assert!(matches!(
        evaluate(
            &settings(IgnoredBuildBehavior::Never),
            project.path(),
            "PREVIEW",
            &[]
        )
        .await
        .unwrap(),
        IgnoredBuildOutcome::Skip { .. }
    ));
}

#[tokio::test]
async fn folder_change_policy_uses_the_project_git_tree() {
    let project = tempfile::tempdir().unwrap();
    run_git(project.path(), &["init"]);
    run_git(
        project.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    run_git(
        project.path(),
        &["config", "user.name", "Ignored Build Test"],
    );
    std::fs::create_dir_all(project.path().join("app/src")).unwrap();
    std::fs::create_dir(project.path().join("docs")).unwrap();
    std::fs::write(project.path().join("app/src/index.js"), "first").unwrap();
    std::fs::write(project.path().join("docs/readme.md"), "first").unwrap();
    run_git(project.path(), &["add", "."]);
    run_git(project.path(), &["commit", "-m", "initial"]);
    std::fs::write(project.path().join("docs/readme.md"), "second").unwrap();
    run_git(project.path(), &["add", "."]);
    run_git(project.path(), &["commit", "-m", "docs"]);

    let mut settings = settings(IgnoredBuildBehavior::ChangesInFolder);
    settings.ignored_build_folder = Some("src".to_string());
    assert!(matches!(
        evaluate(&settings, &project.path().join("app"), "PREVIEW", &[])
            .await
            .unwrap(),
        IgnoredBuildOutcome::Skip { .. }
    ));

    std::fs::write(project.path().join("app/src/index.js"), "second").unwrap();
    run_git(project.path(), &["add", "."]);
    run_git(project.path(), &["commit", "-m", "app"]);
    assert_eq!(
        evaluate(&settings, &project.path().join("app"), "PREVIEW", &[])
            .await
            .unwrap(),
        IgnoredBuildOutcome::Continue {
            reason: "Ignored Build Step found relevant changes (exit code 1); continuing build"
                .to_string()
        }
    );
}

#[tokio::test]
async fn only_changes_uses_parent_from_production_shallow_history() {
    let source = tempfile::tempdir().unwrap();
    run_git(source.path(), &["init"]);
    run_git(
        source.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    run_git(
        source.path(),
        &["config", "user.name", "Ignored Build Test"],
    );
    std::fs::write(source.path().join("index.js"), "first").unwrap();
    run_git(source.path(), &["add", "."]);
    run_git(source.path(), &["commit", "-m", "initial"]);
    std::fs::write(source.path().join("index.js"), "second").unwrap();
    run_git(source.path(), &["add", "."]);
    run_git(source.path(), &["commit", "-m", "change"]);

    let checkout_parent = tempfile::tempdir().unwrap();
    let checkout = checkout_parent.path().join("checkout");
    let source_url = format!("file://{}", source.path().display());
    let mut command = Command::new("git");
    remove_git_repository_environment(&mut command);
    let status = command
        .args(["clone", "--depth=2", &source_url])
        .arg(&checkout)
        .status()
        .unwrap();
    assert!(status.success(), "shallow fixture clone failed");

    assert_eq!(
        evaluate(
            &settings(IgnoredBuildBehavior::OnlyChanges),
            &checkout,
            "PREVIEW",
            &[]
        )
        .await
        .unwrap(),
        IgnoredBuildOutcome::Continue {
            reason: "Ignored Build Step found relevant changes (exit code 1); continuing build"
                .to_string()
        }
    );
}

#[tokio::test]
async fn invalid_or_timed_out_commands_fail_closed() {
    let project = tempfile::tempdir().unwrap();
    let missing_command = settings(IgnoredBuildBehavior::Custom);
    let error = evaluate(&missing_command, project.path(), "PREVIEW", &[])
        .await
        .unwrap_err();
    assert!(error.to_string().contains("requires a non-empty"));

    let mut timed_out = settings(IgnoredBuildBehavior::Custom);
    timed_out.ignored_build_command = Some("sleep 5".to_string());
    let error = evaluate_for_test(
        IgnoredBuildRequest {
            settings: &timed_out,
            environment_type: "PREVIEW",
            project_dir: project.path(),
            execution_env: &[],
            json: true,
            build_logs: None,
        },
        Duration::from_secs(2),
        Duration::from_millis(25),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("exceeded its 25 ms timeout"));
}

fn run_git(directory: &std::path::Path, args: &[&str]) {
    let mut command = Command::new("git");
    remove_git_repository_environment(&mut command);
    let status = command.args(args).current_dir(directory).status().unwrap();
    assert!(status.success(), "git {args:?} failed");
}
