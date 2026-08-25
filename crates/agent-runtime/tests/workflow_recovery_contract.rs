use agent_runtime::migrations::run_migrations;
use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::workflow_recovery::{LeaseError, RecoveryStatus, RecoveryStore};
use agent_runtime::workflow_state_repo::{CreateRun, StateStore, Transition};

async fn setup() -> (SqliteStorage, StateStore, RecoveryStore) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-1', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .execute(storage.pool()).await.unwrap();
    let state = StateStore::new(storage.pool().clone());
    let recovery = RecoveryStore::new(storage.pool().clone());
    (storage, state, recovery)
}

// @spec:AC-1051
#[tokio::test]
async fn lease_expiry_and_epoch_fencing_prevent_split_brain() {
    let (_storage, state, recovery) = setup().await;
    state
        .create_run(CreateRun::new("project-1", "run-1", "workflow-1", 1).unwrap())
        .await
        .unwrap();
    let first = recovery
        .acquire_lease("project-1", "run-1", "runner-a", 100, 10)
        .await
        .unwrap();
    assert!(matches!(
        recovery
            .acquire_lease("project-1", "run-1", "runner-b", 105, 10)
            .await,
        Err(LeaseError::Busy)
    ));
    assert!(recovery
        .fence("project-1", "run-1", "runner-a", first.generation, 105)
        .await
        .unwrap());
    let second = recovery
        .acquire_lease("project-1", "run-1", "runner-b", 111, 10)
        .await
        .unwrap();
    assert!(second.generation > first.generation);
    assert!(!recovery
        .fence("project-1", "run-1", "runner-a", first.generation, 112)
        .await
        .unwrap());
    assert!(recovery
        .fence("project-1", "run-1", "runner-b", second.generation, 112)
        .await
        .unwrap());
}

// @spec:AC-1052
#[tokio::test]
async fn expired_running_node_is_quarantined_without_execution() {
    let (_storage, state, recovery) = setup().await;
    state
        .create_run(CreateRun::new("project-1", "run-1", "workflow-1", 1).unwrap())
        .await
        .unwrap();
    recovery
        .acquire_lease("project-1", "run-1", "runner-a", 100, 10)
        .await
        .unwrap();
    state
        .transition(
            Transition::new(
                "project-1",
                "run-1",
                "node-1",
                0,
                "ready",
                "running",
                "dispatch-1",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let report = recovery
        .recover_expired("project-1", "runner-b", 111, 10, 8)
        .await
        .unwrap();
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].status, RecoveryStatus::Unknown);
    assert!(report.candidates[0].requires_reconcile);
    assert!(!report.candidates[0].executed);
}

// @spec:AC-1053
#[tokio::test]
async fn recovery_report_is_bounded_sorted_and_identity_scoped() {
    let (_storage, state, recovery) = setup().await;
    for run_id in ["run-b", "run-a"] {
        state
            .create_run(CreateRun::new("project-1", run_id, "workflow-1", 1).unwrap())
            .await
            .unwrap();
        recovery
            .acquire_lease("project-1", run_id, "runner-a", 100, 10)
            .await
            .unwrap();
        state
            .transition(
                Transition::new(
                    "project-1",
                    run_id,
                    "node-1",
                    0,
                    "ready",
                    "running",
                    format!("dispatch-{run_id}"),
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }
    let report = recovery
        .recover_expired("project-1", "runner-b", 111, 1, 1)
        .await
        .unwrap();
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].run_id, "run-a");
    assert!(!report.diagnostics.contains("secret"));
    assert!(matches!(
        recovery
            .recover_expired("other-project", "runner-b", 111, 1, 8)
            .await,
        Err(LeaseError::ProjectScope)
    ));
}
