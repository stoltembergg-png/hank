use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::workflow_recovery::{LeaseError, RecoveryStatus, RecoveryStore};
use sqlx::{Executor, Pool, Sqlite};

async fn store_with_run() -> (RecoveryStore, Pool<Sqlite>) {
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
    (
        RecoveryStore::new(storage.pool().clone()),
        storage.pool().clone(),
    )
}

// @spec:AC-1051
#[tokio::test]
async fn lease_fencing_rejects_competing_runner() {
    let (store, _) = store_with_run().await;
    let lease = store
        .acquire_lease("project", "run", "runner-a", 10, 100)
        .await
        .unwrap();
    assert!(!store
        .fence("project", "run", "runner-b", lease.generation, 20)
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

// @spec:AC-1052
#[tokio::test]
async fn recovery_is_bounded_and_increments_generation() {
    let (store, pool) = store_with_run().await;
    pool.execute("INSERT INTO workflow_runs (project_id, run_id, workflow_id, workflow_version, state, generation, sequence, created_at_ms, updated_at_ms, lease_owner, lease_expires_at_ms) VALUES ('project', 'run-2', 'workflow', 1, 'running', 0, 0, 1, 1, 'runner-z', 11)")
        .await
        .unwrap();
    pool.execute("INSERT INTO workflow_node_states (project_id, run_id, node_id, state, generation, recovery_class, unknown_effect, updated_at_ms) VALUES ('project', 'run-2', 'node', 'running', 0, 'pending', 0, 1)")
        .await
        .unwrap();
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
    let untouched: (i64, Option<String>) =
        sqlx::query_as("SELECT generation, lease_owner FROM workflow_runs WHERE run_id = 'run-2'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let reports: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_recovery_reports")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(untouched, (0, Some("runner-z".into())));
    assert_eq!(reports.0, 1);
}

// @spec:AC-1052
#[tokio::test]
async fn recovery_marks_unknown_without_execution() {
    let (store, _) = store_with_run().await;
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

// @spec:AC-1053
#[tokio::test]
async fn repeated_recovery_does_not_duplicate_active_lease() {
    let (store, _) = store_with_run().await;
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

// @spec:AC-1053
#[tokio::test]
async fn invalid_recovery_inputs_fail_without_mutation() {
    let (store, pool) = store_with_run().await;
    let before_run: (i64, Option<String>) =
        sqlx::query_as("SELECT generation, lease_owner FROM workflow_runs WHERE run_id = 'run'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let before_reports: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_recovery_reports")
        .fetch_one(&pool)
        .await
        .unwrap();
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
        Err(LeaseError::InvalidIdentity),
    );
    let after_run: (i64, Option<String>) =
        sqlx::query_as("SELECT generation, lease_owner FROM workflow_runs WHERE run_id = 'run'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let after_reports: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_recovery_reports")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after_run, before_run);
    assert_eq!(after_reports, before_reports);
}
