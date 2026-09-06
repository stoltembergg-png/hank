use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::workflow_recovery::{LeaseError, RecoveryStatus, RecoveryStore};
use sqlx::Executor;

async fn store_with_run() -> RecoveryStore {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    storage
        .pool()
        .execute(
            "CREATE TABLE projects (id TEXT PRIMARY KEY);
             CREATE TABLE workflow_runs (
               project_id TEXT NOT NULL, run_id TEXT NOT NULL, workflow_id TEXT NOT NULL,
               workflow_version INTEGER NOT NULL, state TEXT NOT NULL, generation INTEGER NOT NULL,
               sequence INTEGER NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
               lease_owner TEXT, lease_expires_at_ms INTEGER,
               PRIMARY KEY (project_id, run_id)
             );
             CREATE TABLE workflow_node_states (
               project_id TEXT NOT NULL, run_id TEXT NOT NULL, node_id TEXT NOT NULL,
               state TEXT NOT NULL, generation INTEGER NOT NULL, recovery_class TEXT NOT NULL,
               unknown_effect INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
               PRIMARY KEY (project_id, run_id, node_id)
             );
             CREATE TABLE workflow_recovery_reports (
               project_id TEXT NOT NULL, run_id TEXT NOT NULL, recovery_id TEXT NOT NULL,
               previous_generation INTEGER NOT NULL, new_generation INTEGER NOT NULL,
               recovery_class TEXT NOT NULL, requires_reconcile INTEGER NOT NULL,
               created_at_ms INTEGER NOT NULL, PRIMARY KEY (project_id, recovery_id)
             );
             INSERT INTO projects (id) VALUES ('project');
             INSERT INTO workflow_runs
               (project_id, run_id, workflow_id, workflow_version, state, generation, sequence, created_at_ms, updated_at_ms)
               VALUES ('project', 'run', 'workflow', 1, 'running', 0, 0, 1, 1);
             INSERT INTO workflow_node_states
               (project_id, run_id, node_id, state, generation, recovery_class, unknown_effect, updated_at_ms)
               VALUES ('project', 'run', 'node', 'running', 0, 'pending', 0, 1);",
        )
        .await
        .unwrap();
    RecoveryStore::new(storage.pool().clone())
}

// @spec:AC-2401
#[tokio::test]
async fn lease_fencing_rejects_competing_runner() {
    let store = store_with_run().await;
    let lease = store
        .acquire_lease("project", "run", "runner-a", 10, 100)
        .await
        .unwrap();
    assert!(store
        .fence("project", "run", "runner-a", lease.generation, 20)
        .await
        .unwrap());
    assert_eq!(
        store
            .acquire_lease("project", "run", "runner-b", 20, 100)
            .await,
        Err(LeaseError::Busy)
    );
    assert!(!store
        .fence("project", "run", "runner-a", lease.generation, 111)
        .await
        .unwrap());
}

// @spec:AC-2402
#[tokio::test]
async fn recovery_is_bounded_and_increments_generation() {
    let store = store_with_run().await;
    let old = store
        .acquire_lease("project", "run", "runner-a", 10, 1)
        .await
        .unwrap();
    let report = store
        .recover_expired("project", "runner-b", 20, 100, 1)
        .await
        .unwrap();
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].previous_generation, old.generation);
    assert_eq!(report.candidates[0].new_generation, old.generation + 1);
}

// @spec:AC-2403
#[tokio::test]
async fn recovery_marks_unknown_without_execution() {
    let store = store_with_run().await;
    store
        .acquire_lease("project", "run", "runner-a", 10, 1)
        .await
        .unwrap();
    let report = store
        .recover_expired("project", "runner-b", 20, 100, 1)
        .await
        .unwrap();
    let candidate = &report.candidates[0];
    assert_eq!(candidate.status, RecoveryStatus::Unknown);
    assert!(candidate.requires_reconcile);
    assert!(!candidate.executed);
}

// @spec:AC-2404
#[tokio::test]
async fn repeated_recovery_does_not_duplicate_active_lease() {
    let store = store_with_run().await;
    store
        .acquire_lease("project", "run", "runner-a", 10, 1)
        .await
        .unwrap();
    let first = store
        .recover_expired("project", "runner-b", 20, 100, 1)
        .await
        .unwrap();
    let second = store
        .recover_expired("project", "runner-c", 21, 100, 1)
        .await
        .unwrap();
    assert_eq!(first.candidates.len(), 1);
    assert!(second.candidates.is_empty());
}

// @spec:AC-2405
#[tokio::test]
async fn invalid_recovery_inputs_fail_without_mutation() {
    let store = store_with_run().await;
    assert_eq!(
        store.acquire_lease("project", "run", "runner", 1, 0).await,
        Err(LeaseError::Budget)
    );
    assert_eq!(
        store.recover_expired("project", "runner", 1, 10, 0).await,
        Err(LeaseError::Budget)
    );
    assert_eq!(
        store
            .acquire_lease("project", "run", "bad\nrunner", 1, 10)
            .await,
        Err(LeaseError::InvalidIdentity)
    );
}
