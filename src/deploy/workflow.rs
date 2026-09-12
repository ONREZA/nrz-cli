use super::*;

pub async fn run(
    args: DeployArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let resume_deployment_id = args
        .resume_deployment
        .as_deref()
        .map(str::trim)
        .map(|deployment_id| {
            Uuid::parse_str(deployment_id).map_err(|_| {
                crate::errors::CliError::new(
                    "INVALID_ARGUMENT",
                    "--resume-deployment requires a valid deployment UUID",
                )
                .phase(output::Phase::Deploy)
                .details(serde_json::json!({ "argument": "--resume-deployment" }))
                .hint("Pass a deployment UUID or omit --resume-deployment.")
                .into_anyhow()
            })
        })
        .transpose()?;
    edge_handoff::validate_resume_arguments(&args)?;
    let mut command_context = if resume_deployment_id.is_some() {
        crate::context::CommandContext::resolve_platform_root(&args.dir, config, json)?
    } else {
        crate::context::CommandContext::resolve(&args.dir, config, args.app.as_deref(), json)?
    };
    if let Some(app) = &command_context.selected_app {
        output::status(
            json,
            "~",
            format!(
                "Monorepo: deploying app \"{}\" from {}/",
                app.requested, app.path
            ),
            output::Phase::Deploy,
        );
    }

    // Verify auth early to avoid wasting time on build if token is invalid
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let edge_build_handoff =
        edge_handoff::EdgeBuildHandoffOutput::from_process_environment(resume_deployment_id)?;
    let dependency_packaging = if edge_build_handoff.is_some() {
        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization
    } else {
        crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::Embedded
    };
    let server_failure_mutations_enabled = edge_build_handoff.is_none();
    let pre_source_failure_client = server_failure_mutations_enabled.then_some(&client);

    let runner_context = if let Some(deployment_id) = resume_deployment_id {
        Some(wire::load_runner_context(&client, deployment_id).await?)
    } else {
        None
    };

    if let Some(runner) = &runner_context {
        command_context.apply_platform_runner_settings(&runner.settings)?;
    } else {
        command_context.apply_project_id_override(args.project_id.as_deref())?;
    }
    let mut early_project_id = runner_context
        .as_ref()
        .map(|runner| runner.context.project_id.clone())
        .or_else(|| command_context.effective.project_id().map(str::to_string));
    if !args.dry && early_project_id.is_none() {
        if json {
            bail!(
                "no linked project. Use --project-id, set [project] id in onreza.toml, or run `nrz link` first."
            );
        }
        output::warn(
            false,
            "No linked project. Select one:",
            output::Phase::Deploy,
        );
        let selected = link::select_project_interactive(&client).await?;
        nrz::config::save_or_update(
            &command_context.project_dir,
            &selected.project_id,
            Some(&selected.project_name),
            None,
        )?;
        crate::init::add_to_gitignore(&command_context.project_dir);
        output::success(
            false,
            format!(
                "Linked to {}",
                console::style(&selected.project_name).bold()
            ),
            output::Phase::Deploy,
        );
        early_project_id = Some(selected.project_id);
    }

    // Fetch project settings from server if project_id is known
    let server_settings = if let Some(runner) = &runner_context {
        Some(runner.settings.clone())
    } else if let Some(ref pid) = early_project_id {
        match crate::project_settings::fetch_for_effective_config(&client, pid).await? {
            crate::project_settings::ProjectSettingsFetch::Applied(info) => {
                tracing::info!(
                    ?info.build_command,
                    ?info.build_command_source,
                    ?info.install_command,
                    ?info.install_command_source,
                    ?info.output_directory,
                    ?info.output_directory_source,
                    ?info.framework_preset,
                    "fetched project settings from server"
                );
                Some(info)
            }
            crate::project_settings::ProjectSettingsFetch::TransientFailure { message } => {
                output::warn(
                    json,
                    format!(
                        "Could not fetch project settings: {message}. Using local configuration."
                    ),
                    output::Phase::Deploy,
                );
                None
            }
        }
    } else {
        None
    };

    if runner_context.is_none() {
        command_context.apply_server_settings(server_settings.as_ref());
    }

    // Explicit compute intent is safe to resolve before build because it comes
    // only from CLI/config. Framework detection stays post-build: generated
    // outputs such as root index.html are part of the detection surface.
    let explicit_compute = resolve_explicit_compute_type(
        args.compute.as_deref(),
        command_context.effective.deploy_compute(),
    )?;
    if args.dry {
        let deploy_plan = plan::build(plan::DeployPlanRequest {
            args: &args,
            command: &command_context,
            explicit_compute,
            build_logs: None,
            execution_env: &[],
            target_production: args.prod.then_some(true),
            platform_runner: edge_build_handoff.is_some(),
        })
        .await?;
        let source_bundle = deploy_plan.materialize_source_bundle(json, dependency_packaging)?;
        let explain = deploy_plan.explain(
            &command_context,
            early_project_id.as_deref(),
            &source_bundle,
        );
        emit_deploy_plan_explain(json, &explain)?;
        return Ok(());
    }

    let project_id = early_project_id
        .clone()
        .context("project must be resolved before deployment admission")?;

    let (deployment, execution_context, materialized) = if let Some(runner) = &runner_context {
        let materialized = match crate::execution_context::materialize_deployment(
            &client,
            &runner.deployment.id,
            "DEPLOY",
        )
        .await
        {
            Ok(materialized) => materialized,
            Err(error) => {
                report_pre_source_failure(
                    pre_source_failure_client,
                    &runner.deployment.id,
                    runner.deployment.attempt,
                    PreSourceFailureCode::MaterializationFailed,
                    Some(&error),
                    None,
                    json,
                )
                .await;
                return Err(error);
            }
        };
        (
            AdmissionDeployment {
                id: runner.deployment.id.clone(),
                attempt: runner.deployment.attempt,
                status: runner.deployment.status.clone(),
                url: runner.deployment.url.clone().unwrap_or_default(),
            },
            runner.context.clone(),
            materialized,
        )
    } else {
        if args.prod && args.environment.is_some() {
            bail!("--prod conflicts with --environment; select one exact environment");
        }
        let selector = args
            .environment
            .as_deref()
            .or(args.prod.then_some("production"));
        let preliminary_branch = git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"]);
        let context = crate::execution_context::resolve_for_mutation(
            &client,
            &project_id,
            &command_context.project_dir,
            selector,
            preliminary_branch.as_deref(),
        )
        .await?;
        let branch = preliminary_branch
            .or_else(|| context.source_ref.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "local".to_string());
        let commit_sha = git_cmd(&["rev-parse", "HEAD"]).unwrap_or_else(|| {
            output::warn(
                json,
                "git not available, using a synthetic source revision",
                output::Phase::Deploy,
            );
            Uuid::now_v7().simple().to_string()
        });
        output::status(json, "~", "Admitting deployment...", output::Phase::Deploy);
        let admitted = wire::admit(&client, &project_id, &context, branch, commit_sha)
            .await
            .map_err(|error| map_create_deployment_error(error, json))?;
        let materialized = match crate::execution_context::materialize_deployment(
            &client,
            &admitted.deployment.id,
            "DEPLOY",
        )
        .await
        {
            Ok(materialized) => materialized,
            Err(error) => {
                report_pre_source_failure(
                    pre_source_failure_client,
                    &admitted.deployment.id,
                    admitted.deployment.attempt,
                    PreSourceFailureCode::MaterializationFailed,
                    Some(&error),
                    None,
                    json,
                )
                .await;
                return Err(error);
            }
        };
        (admitted.deployment, admitted.context, materialized)
    };

    if deployment.status != "BUILDING" {
        let error = anyhow::anyhow!(
            "deployment {} is in {} state, expected BUILDING",
            deployment.id,
            deployment.status
        );
        report_pre_source_failure(
            pre_source_failure_client,
            &deployment.id,
            deployment.attempt,
            PreSourceFailureCode::ConfigInvalid,
            Some(&error),
            None,
            json,
        )
        .await;
        return Err(error);
    }
    if materialized.context.environment_id != execution_context.environment_id {
        let error = anyhow::anyhow!(
            "ENV_SNAPSHOT_SCOPE_MISMATCH: deployment context changed during admission"
        );
        report_pre_source_failure(
            pre_source_failure_client,
            &deployment.id,
            deployment.attempt,
            PreSourceFailureCode::ConfigInvalid,
            Some(&error),
            None,
            json,
        )
        .await;
        return Err(error);
    }
    if !args.skip_env_check
        && let Err(error) = crate::cli::env_handler::validate_materialized_env_for_deploy(
            &materialized.variables,
            json,
            &command_context.config,
        )
    {
        report_pre_source_failure(
            pre_source_failure_client,
            &deployment.id,
            deployment.attempt,
            PreSourceFailureCode::ConfigInvalid,
            Some(&error),
            None,
            json,
        )
        .await;
        return Err(error);
    }
    if let Err(error) =
        crate::execution_context::warn_local_dotenv_drift(&command_context.project_dir, json)
    {
        report_pre_source_failure(
            pre_source_failure_client,
            &deployment.id,
            deployment.attempt,
            PreSourceFailureCode::ConfigInvalid,
            Some(&error),
            None,
            json,
        )
        .await;
        return Err(error);
    }
    let mut execution_env = crate::execution_context::execution_environment(&materialized)
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(runner) = &runner_context {
        let toolchain_environment = package_manager_toolchain::environment_from_process(
            &runner.settings.package_manager,
            &command_context.project_dir,
            &command_context.root_dir,
        )
        .map_err(|error| {
            output::coded_error(
                "PLATFORM_TOOLCHAIN_UNAVAILABLE",
                format!("failed to select the platform package-manager toolchain: {error:#}"),
            )
        })?;
        execution_env = merge_command_environment(&execution_env, &toolchain_environment);
    }
    execution_env.sort_by(|left, right| left.0.cmp(&right.0));
    let mut build_log_secret_values = crate::execution_context::secret_values(&materialized);
    build_log_secret_values.push(tok.clone());
    let pre_source_failure_redactor =
        match ExactValueRedactor::from_materialized_values(&build_log_secret_values) {
            Ok(redactor) => redactor,
            Err(error) => {
                let error = error.context("failed to initialize deployment output redaction");
                report_pre_source_failure(
                    pre_source_failure_client,
                    &deployment.id,
                    deployment.attempt,
                    PreSourceFailureCode::BuildFailed,
                    Some(&error),
                    None,
                    json,
                )
                .await;
                return Err(error);
            }
        };
    let mut build_log_session = BuildLogSession::start(build_logs::StartBuildLogSession {
        client: &client,
        project_id: &project_id,
        deployment_id: &deployment.id,
        workspace_id: &execution_context.workspace_id,
        project_dir: &command_context.project_dir,
        redactor: pre_source_failure_redactor.clone(),
        config: BuildLogUploadConfig::from_args(&args, deployment.attempt),
        json,
    })
    .await;
    let target_production = Some(execution_context.environment_type == "PRODUCTION");

    let admitted_deployment_id = deployment.id.clone();
    let mut source_registered = false;
    let mut skipped_by_ignored_build_step = false;
    let deploy_result = async {
        if let Some(runner) = &runner_context {
            match ignored_build::evaluate(IgnoredBuildRequest {
                settings: &runner.settings,
                environment_type: &execution_context.environment_type,
                project_dir: &command_context.project_dir,
                execution_env: &execution_env,
                json,
                build_logs: build_log_session
                    .as_ref()
                    .and_then(BuildLogSession::emitter),
            })
            .await?
            {
                IgnoredBuildOutcome::Continue { .. } => {}
                IgnoredBuildOutcome::Skip { reason } => {
                    mark_pre_source_skipped(&client, &deployment.id, deployment.attempt, &reason)
                        .await?;
                    skipped_by_ignored_build_step = true;
                    output::success(
                        json,
                        "Deployment marked SKIPPED; install and build were not started",
                        output::Phase::Deploy,
                    );
                    if json {
                        output::json_output(&SkippedDeployOutput {
                            deployment_id: &deployment.id,
                            status: "skipped",
                        });
                    }
                    return Ok(());
                }
            }
        }

        let deploy_plan = plan::build(plan::DeployPlanRequest {
            args: &args,
            command: &command_context,
            explicit_compute,
            build_logs: build_log_session
                .as_ref()
                .and_then(BuildLogSession::emitter),
            execution_env: &execution_env,
            target_production,
            platform_runner: edge_build_handoff.is_some(),
        })
        .await?;
        if let Some(emitter) = build_log_session
            .as_ref()
            .and_then(BuildLogSession::emitter)
        {
            emitter.info(BuildLogPhase::Detect, "Build output validated");
        }
        let upload_plan = deploy_plan.materialize_source_bundle(json, dependency_packaging)?;
        let runtime_artifact_files = deploy_plan.artifact.file_breakdown.clone();
        if let Some(publisher) = &edge_build_handoff {
            let handoff = publisher.publish(&upload_plan)?;
            if json {
                output::json_output(&handoff);
            } else {
                output::success(
                    false,
                    "Edge build handoff published for trusted Agent verification",
                    output::Phase::Deploy,
                );
            }
            return Ok(());
        }
        register_deployment_source(
            &client,
            &admitted_deployment_id,
            deployment.attempt,
            deploy_plan.manifest_raw.clone(),
            conform_functions_to_wire_contract(deploy_plan.functions)?,
            json,
        )
        .await?;
        source_registered = true;

        // ── Resume mode: builder calls us with an existing deployment ID ──
        if resume_deployment_id.is_some() {
            return resume_deploy(ResumeDeployRequest {
                client: &client,
                deployment_id: &deployment.id,
                workspace_id: &execution_context.workspace_id,
                project_id: &project_id,
                upload_plan,
                json,
                warnings: deploy_plan.warnings,
                runtime_artifact_files,
            })
            .await;
        }

        // ── Normal flow continues below ─────────────────────────────────

        let deploy_warnings = deploy_plan.warnings.clone();
        let deploy_runtime_artifact_files = runtime_artifact_files;
        let deploy_health_check = deploy_plan.health_check.clone();
        let deploy_production = deploy_plan.production;
        let sync_detection = deploy_plan.artifact.build.detection.clone();

        // Sync detection results to API (best-effort, non-blocking)
        let sync_client = client.clone();
        let sync_project_id = project_id.clone();
        let _sync = tokio::spawn(async move {
            crate::detect_sync::sync_detection_to_api(
                &sync_client,
                &sync_project_id,
                &sync_detection,
            )
            .await;
        });

        // Sync compute config (health check path) for PROCESS deployments
        if let Some(ref hc) = deploy_health_check {
            let hc_client = client.clone();
            let hc_project_id = project_id.clone();
            let hc_clone = hc.clone();
            let _hc = tokio::spawn(async move {
                sync_compute_config(&hc_client, &hc_project_id, &hc_clone, json).await;
            });
        }

        output::success(
            json,
            format!(
                "SOURCE_BUNDLE_V1 archive ready ({}, sha256: {}...)",
                format_u64_bytes(upload_plan.source_size_bytes),
                &upload_plan.source_sha256[..12]
            ),
            output::Phase::Deploy,
        );
        let deployment_attempt_id = Uuid::now_v7().to_string();

        if let Some(emitter) = build_log_session
            .as_ref()
            .and_then(BuildLogSession::emitter)
        {
            emitter.info(BuildLogPhase::Upload, "Uploading deployment source bundle");
        }

        prepare_upload_and_complete(PrepareUploadAndCompleteRequest {
            client: &client,
            deployment_id: &deployment.id,
            workspace_id: &execution_context.workspace_id,
            project_id: &project_id,
            deployment_attempt_id: &deployment_attempt_id,
            json,
            plan: &upload_plan,
            runtime_artifact_files: &deploy_runtime_artifact_files,
        })
        .await?;
        if let Some(emitter) = build_log_session
            .as_ref()
            .and_then(BuildLogSession::emitter)
        {
            emitter.info(
                BuildLogPhase::Activate,
                "Deployment source uploaded; waiting for activation",
            );
        }

        let spinner = make_spinner(json, "Waiting for activation...");
        let result = wait_for_activation(
            ActivationWait {
                deployment_id: &deployment.id,
                url: &deployment.url,
                timeout: Duration::from_secs(u64::from(args.wait_timeout)),
            },
            || async { client.deployment_status(&deployment.id).await?.try_into() },
            |status| {
                if let Some(spinner) = &spinner {
                    spinner.set_message(format!("Status: {}...", output::terminal_line(status)));
                }
            },
        )
        .await;
        finish_spinner(spinner, "");
        let status = result?;
        let url = status.url.as_deref().unwrap_or(&deployment.url);
        let target = deploy_target_output(deploy_production);
        let preview_protected = deploy_preview_protected(deploy_production);
        let verification = if args.verify {
            Some(
                verify::verify_deployment(verify::DeployVerificationRequest {
                    api_client: &client,
                    deployment_id: &deployment.id,
                    project_id: &project_id,
                    url,
                    production: deploy_production == Some(true),
                    health_check: deploy_health_check.as_ref(),
                    json,
                })
                .await?,
            )
        } else {
            None
        };

        if json {
            output::json_output(&DeployOutput {
                deployment_id: deployment.id,
                url: url.to_string(),
                status: "live".into(),
                target,
                preview_protected,
                runtime_artifact_files: deploy_runtime_artifact_files.clone(),
                warnings: deploy_warnings.clone(),
                health_check: deploy_health_check.as_ref().map(|hc| hc.to_info()),
                verification,
            });
        } else {
            let url = output::terminal_line(url);
            eprintln!();
            eprintln!(
                "  {} Deployed to {}",
                console::style("✓").green().bold(),
                console::style(&url).underlined().bold(),
            );
            if let Some(verification) = &verification {
                let verified_url = output::terminal_line(&verification.url);
                eprintln!(
                    "  {} Verified {} ({})",
                    console::style("✓").green().bold(),
                    console::style(verified_url).underlined(),
                    verification.status_code
                );
            }
            eprintln!();
            if preview_protected {
                crate::preview::print_preview_access_hint(&project_id, Some(&url));
            }
        }
        Ok(())
    }
    .await;

    if let Err(error) = &deploy_result
        && !source_registered
    {
        report_pre_source_failure(
            pre_source_failure_client,
            &admitted_deployment_id,
            deployment.attempt,
            PreSourceFailureCode::BuildFailed,
            Some(error),
            Some(&pre_source_failure_redactor),
            json,
        )
        .await;
    }
    if let Some(session) = build_log_session.as_mut() {
        let success = if skipped_by_ignored_build_step {
            BuildLogSuccess::DeploymentSkipped
        } else if edge_build_handoff.is_some() {
            BuildLogSuccess::EdgeHandoffPublished
        } else {
            BuildLogSuccess::ArtifactsUploaded
        };
        let outcome = match &deploy_result {
            Ok(()) => BuildLogOutcome::Completed(success),
            Err(error) if is_activation_observation_error(error) => {
                BuildLogOutcome::ObservationStopped { success, error }
            }
            Err(error) => BuildLogOutcome::Failed(error),
        };
        session.finish(outcome).await;
    }
    deploy_result
}
