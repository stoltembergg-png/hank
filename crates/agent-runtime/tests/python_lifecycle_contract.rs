use agent_runtime::python_lifecycle::{
    LifecycleError, LifecycleEvent, LifecycleState, PythonLifecycle, PythonLifecycleConfig,
    WorkerIdentity,
};
use std::time::Duration;

fn config() -> PythonLifecycleConfig {
    let (command, args) = worker_command();
    PythonLifecycleConfig {
        command: command.into(),
        args,
        startup_timeout: Duration::from_secs(1),
        request_timeout: Duration::from_secs(1),
        max_restarts: 2,
        restart_backoff: Duration::ZERO,
    }
}

#[cfg(windows)]
fn worker_command() -> (String, Vec<String>) {
    (
        "cmd.exe".into(),
        vec!["/C".into(), "ping -n 61 127.0.0.1 > NUL".into()],
    )
}

#[cfg(not(windows))]
fn worker_command() -> (String, Vec<String>) {
    ("sh".into(), vec!["-c".into(), "sleep 60".into()])
}

fn identity(project_id: &str) -> WorkerIdentity {
    WorkerIdentity {
        project_id: project_id.into(),
        session_id: "session-1".into(),
        task_id: "task-1".into(),
        trace_id: "trace-1".into(),
    }
}

// @spec:AC-694 @spec:AC-697
#[tokio::test]
async fn spawn_ready_request_stop_cleans_process() {
    let mut lifecycle = PythonLifecycle::new(config(), identity("project-a")).unwrap();

    assert_eq!(lifecycle.state(), LifecycleState::Stopped);
    lifecycle.spawn().await.unwrap();
    assert!(lifecycle.readiness_deadline().is_some());
    lifecycle.mark_ready().unwrap();
    assert!(lifecycle.readiness_deadline().is_none());
    lifecycle.begin_request("op-1", 3).unwrap();
    lifecycle.complete_request("op-1").unwrap();
    lifecycle.stop().await.unwrap();

    assert_eq!(lifecycle.state(), LifecycleState::Stopped);
    assert!(lifecycle.events().iter().any(|event| matches!(
        event,
        LifecycleEvent::StateChanged {
            to: LifecycleState::Ready,
            ..
        }
    )));
}

// @spec:AC-696
#[tokio::test]
async fn duplicate_operation_key_is_rejected_and_does_not_restart() {
    let mut lifecycle = PythonLifecycle::new(config(), identity("project-a")).unwrap();
    lifecycle.spawn().await.unwrap();
    lifecycle.mark_ready().unwrap();
    lifecycle.begin_request("op-1", 3).unwrap();
    let error = lifecycle.begin_request("op-1", 3).unwrap_err();

    assert!(matches!(error, LifecycleError::DuplicateOperation(_)));
    assert_eq!(lifecycle.restart_count(), 0);
    lifecycle.stop().await.unwrap();
}

// @spec:AC-695
#[tokio::test]
async fn timeout_and_cancel_release_budget_and_cleanup() {
    let mut lifecycle = PythonLifecycle::new(config(), identity("project-a")).unwrap();
    lifecycle.spawn().await.unwrap();
    lifecycle.mark_ready().unwrap();
    lifecycle.begin_request("op-timeout", 7).unwrap();
    lifecycle.timeout_request("op-timeout").await.unwrap();
    assert_eq!(lifecycle.reserved_budget(), 0);

    lifecycle.spawn().await.unwrap();
    lifecycle.mark_ready().unwrap();
    lifecycle.begin_request("op-cancel", 5).unwrap();
    lifecycle.cancel_request("op-cancel").await.unwrap();
    assert_eq!(lifecycle.reserved_budget(), 0);
    assert_eq!(lifecycle.state(), LifecycleState::Stopped);
    lifecycle.stop().await.unwrap();
}

// @spec:AC-696
#[tokio::test]
async fn restart_is_bounded_and_identity_is_preserved() {
    let mut lifecycle = PythonLifecycle::new(config(), identity("project-a")).unwrap();
    lifecycle.spawn().await.unwrap();
    lifecycle.mark_ready().unwrap();
    lifecycle.crash().await.unwrap();
    lifecycle.restart().await.unwrap();
    lifecycle.mark_ready().unwrap();

    assert_eq!(lifecycle.restart_count(), 1);
    assert_eq!(lifecycle.identity().project_id, "project-a");

    lifecycle.crash().await.unwrap();
    lifecycle.restart().await.unwrap();
    lifecycle.mark_ready().unwrap();
    assert_eq!(lifecycle.restart_count(), 2);

    lifecycle.crash().await.unwrap();
    assert!(matches!(
        lifecycle.restart().await,
        Err(LifecycleError::RestartLimit)
    ));
}

// @spec:AC-698
#[tokio::test]
async fn worker_command_failure_is_fail_closed() {
    let mut invalid = config();
    invalid.command = "/path/that/does/not/exist".into();
    let mut lifecycle = PythonLifecycle::new(invalid, identity("project-a")).unwrap();

    let error = lifecycle.spawn().await.unwrap_err();
    assert!(matches!(error, LifecycleError::Spawn(_)));
    assert_eq!(lifecycle.state(), LifecycleState::Crashed);
}

#[test]
fn identity_cannot_be_reused_across_projects() {
    let first = PythonLifecycle::new(config(), identity("project-a")).unwrap();
    let second = PythonLifecycle::new(config(), identity("project-b")).unwrap();
    assert_ne!(first.identity().project_id, second.identity().project_id);
}
