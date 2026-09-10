use super::super::{BuildLogEvent, BuildLogOrigin, EmitterState};
use super::*;
use std::sync::{Arc, Mutex, atomic::AtomicU64};
use tokio::sync::mpsc;

fn emitter() -> (BuildLogEmitter, mpsc::Receiver<BuildLogEvent>) {
    let (sender, receiver) = mpsc::channel(10);
    (
        BuildLogEmitter {
            state: Arc::new(Mutex::new(EmitterState {
                next_seq: 0,
                accepted_bytes: 0,
                sender: Some(sender),
            })),
            phase: Arc::new(Mutex::new(BuildLogPhase::Activate)),
            dropped: Arc::new(AtomicU64::new(0)),
            redactor: Arc::new(
                ExactValueRedactor::from_values(["sensitive-value".to_owned()]).unwrap(),
            ),
            include_debug: true,
            lifecycle_origin: BuildLogOrigin::Cli,
        },
        receiver,
    )
}

#[test]
fn stopped_observation_finishes_uploaded_build_with_a_redacted_warning() {
    let (emitter, mut events) = emitter();
    let error = crate::errors::CliError::new("DEPLOY_WAIT_TIMEOUT", "wait stopped sensitive-value")
        .into_anyhow();
    let request = BuildLogOutcome::ObservationStopped {
        success: BuildLogSuccess::ArtifactsUploaded,
        error: &error,
    }
    .prepare(BuildLogPhase::Activate, &emitter.redactor, Some(&emitter));
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({
            "status": "FINISHED", "message": "wait stopped [REDACTED]",
        })
    );
    let warning = events.try_recv().unwrap();
    assert_eq!(warning.level, BuildLogLevel::Warn);
    assert_eq!(warning.phase, BuildLogPhase::Activate);
    assert_eq!(warning.message, "wait stopped [REDACTED]");
    let complete = events.try_recv().unwrap();
    assert_eq!(complete.level, BuildLogLevel::Info);
    assert_eq!(complete.phase, BuildLogPhase::Complete);
    assert_eq!(complete.message, "Build artifacts uploaded");
    assert!(matches!(
        events.try_recv(),
        Err(mpsc::error::TryRecvError::Disconnected)
    ));
}

#[test]
fn failure_preserves_original_phase_and_diagnostics_before_terminal_event() {
    let (emitter, mut events) = emitter();
    let error = crate::errors::CliError::new("DEPLOY_FAILED", "failed sensitive-value")
        .details(serde_json::json!({"reason": "sensitive-value"}))
        .into_anyhow();
    let request = BuildLogOutcome::Failed(&error).prepare(
        BuildLogPhase::Activate,
        &emitter.redactor,
        Some(&emitter),
    );
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({
            "status": "FAILED", "message": "failed [REDACTED]",
            "errorCode": "DEPLOY_FAILED", "errorDetails": {"reason": "[REDACTED]"},
            "failurePhase": "ACTIVATE",
        })
    );
    assert_eq!(events.try_recv().unwrap().phase, BuildLogPhase::Error);
    assert_eq!(*emitter.phase.lock().unwrap(), BuildLogPhase::Error);
}

#[test]
fn successful_build_has_no_failure_metadata_even_without_event_shipping() {
    let redactor = ExactValueRedactor::from_values(Vec::new()).unwrap();
    for success in [
        BuildLogSuccess::ArtifactsUploaded,
        BuildLogSuccess::DeploymentSkipped,
        BuildLogSuccess::EdgeHandoffPublished,
    ] {
        let request =
            BuildLogOutcome::Completed(success).prepare(BuildLogPhase::Upload, &redactor, None);
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"status": "FINISHED"})
        );
    }
}
