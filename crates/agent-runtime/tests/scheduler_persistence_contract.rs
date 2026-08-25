use agent_runtime::migrations::run_migrations;
use agent_runtime::scheduler::{JobStore, JobTarget, MissedRunPolicy, ScheduledJob, Trigger};
use agent_runtime::scheduler_persistence::{PersistenceError, SchedulerPersistence};
use agent_runtime::sqlite::SqliteStorage;

async fn setup() -> (SqliteStorage, SchedulerPersistence) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-a', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')").execute(storage.pool()).await.unwrap();
    let jobs = JobStore::new(storage.pool().clone());
    let job = ScheduledJob::new(
        "project-a",
        "job-a",
        "owner-a",
        Trigger::Interval { seconds: 60 },
        JobTarget::Workflow {
            workflow_id: "workflow-a".into(),
            version: 1,
        },
        "UTC",
        1,
        MissedRunPolicy::Skip,
    )
    .unwrap();
    jobs.create(job).await.unwrap();
    (
        storage.clone(),
        SchedulerPersistence::new(storage.pool().clone()),
    )
}

// @spec:AC-1201
#[tokio::test]
async fn migration_is_repeatable_and_run_is_project_scoped() {
    let (_storage, persistence) = setup().await;
    persistence
        .create_run("project-a", "run-future", "job-a", 2_000)
        .await
        .unwrap();
    assert!(matches!(
        persistence
            .claim("project-a", "run-future", "worker-a", 1_000, 500)
            .await,
        Err(PersistenceError::NotClaimed)
    ));
    persistence
        .create_run("project-a", "run-a", "job-a", 1_000)
        .await
        .unwrap();
    assert_eq!(
        persistence
            .claim("project-a", "run-a", "worker-a", 1_000, 500)
            .await
            .unwrap()
            .status,
        "claimed"
    );
    assert!(matches!(
        persistence
            .claim("project-b", "run-a", "worker-b", 2_000, 500)
            .await,
        Err(PersistenceError::NotFound)
    ));
}

// @spec:AC-1202
#[tokio::test]
async fn lease_expiry_allows_one_recovery_claim() {
    let (_storage, persistence) = setup().await;
    persistence
        .create_run("project-a", "run-lease", "job-a", 1_000)
        .await
        .unwrap();
    persistence
        .claim("project-a", "run-lease", "worker-a", 1_000, 100)
        .await
        .unwrap();
    assert!(matches!(
        persistence
            .claim("project-a", "run-lease", "worker-b", 1_099, 100)
            .await,
        Err(PersistenceError::NotClaimed)
    ));
    let recovered = persistence
        .claim("project-a", "run-lease", "worker-b", 1_100, 100)
        .await
        .unwrap();
    assert_eq!(recovered.lease_owner.as_deref(), Some("worker-b"));
}

// @spec:AC-1203
#[tokio::test]
async fn only_current_owner_completes_and_completion_is_terminal() {
    let (_storage, persistence) = setup().await;
    persistence
        .create_run("project-a", "run-complete", "job-a", 1_000)
        .await
        .unwrap();
    persistence
        .claim("project-a", "run-complete", "worker-a", 1_000, 500)
        .await
        .unwrap();
    assert!(matches!(
        persistence
            .complete("project-a", "run-complete", "worker-b", "ok", 1_100)
            .await,
        Err(PersistenceError::NotClaimed)
    ));
    let completed = persistence
        .complete("project-a", "run-complete", "worker-a", "ok", 1_100)
        .await
        .unwrap();
    assert_eq!(completed.status, "completed");
    assert!(matches!(
        persistence
            .claim("project-a", "run-complete", "worker-c", 2_000, 100)
            .await,
        Err(PersistenceError::Terminal)
    ));
}
