use agent_runtime::event_bus::EventBus;
use agent_runtime::migrations::run_migrations;
use agent_runtime::scheduler::{JobStore, JobTarget, MissedRunPolicy, ScheduledJob, Trigger};
use agent_runtime::scheduler_persistence::SchedulerPersistence;
use agent_runtime::scheduler_worker::{SchedulerWorker, WorkerError};
use agent_runtime::sqlite::SqliteStorage;
use security_core::{RateLimitPolicy, RateLimiter};
use std::sync::Arc;

async fn setup() -> (SqliteStorage, SchedulerPersistence, JobStore) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-a', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')").execute(storage.pool()).await.unwrap();
    let jobs = JobStore::new(storage.pool().clone());
    jobs.create(
        ScheduledJob::new(
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
        .unwrap(),
    )
    .await
    .unwrap();
    (
        storage.clone(),
        SchedulerPersistence::new(storage.pool().clone()),
        jobs,
    )
}

// @spec:AC-1221
#[tokio::test]
async fn tick_claims_due_runs_with_bounded_dispatch_identity() {
    let (_storage, persistence, _jobs) = setup().await;
    persistence
        .create_run("project-a", "run-a", "job-a", 1_000)
        .await
        .unwrap();
    persistence
        .create_run("project-a", "run-b", "job-a", 1_001)
        .await
        .unwrap();
    let bus = EventBus::bounded(2);
    let mut receiver = bus.subscribe();
    let worker = SchedulerWorker::new(persistence, bus, "worker-a", 500, 1).unwrap();
    assert_eq!(worker.tick("project-a", 1_000).await.unwrap(), 1);
    let envelope = receiver.recv().await.unwrap();
    assert_eq!(envelope.idempotency_key, "scheduler:project-a:run-a");
    assert_eq!(worker.tick("project-a", 1_000).await.unwrap(), 0);
}

// @spec:AC-1222
#[tokio::test]
async fn shutdown_stops_claiming_and_renew_preserves_lease_owner() {
    let (_storage, persistence, _jobs) = setup().await;
    persistence
        .create_run("project-a", "run-a", "job-a", 1_000)
        .await
        .unwrap();
    let bus = EventBus::bounded(2);
    let _receiver = bus.subscribe();
    let inspection = persistence.clone();
    let worker = SchedulerWorker::new(persistence, bus, "worker-a", 500, 4).unwrap();
    let run = worker.tick("project-a", 1_000).await.unwrap();
    assert_eq!(run, 1);
    let claimed = worker
        .renew(
            &inspection.get_run("project-a", "run-a").await.unwrap(),
            1_100,
        )
        .await
        .unwrap();
    assert_eq!(claimed.lease_owner.as_deref(), Some("worker-a"));
    worker.shutdown();
    assert!(matches!(
        worker.tick("project-a", 2_000).await,
        Err(WorkerError::Stopped)
    ));
}

// @spec:AC-2578
#[tokio::test]
async fn tick_rate_limit_denies_a_second_trigger_before_claiming_a_lease() {
    let (_storage, persistence, _jobs) = setup().await;
    persistence
        .create_run("project-a", "run-a", "job-a", 1_000)
        .await
        .unwrap();
    persistence
        .create_run("project-a", "run-b", "job-a", 1_000)
        .await
        .unwrap();
    let bus = EventBus::bounded(2);
    let _receiver = bus.subscribe();
    let limiter = Arc::new(
        RateLimiter::new(RateLimitPolicy::new("scheduler-policy-1", 1, 1, 1_000, 1, 8, 4).unwrap())
            .unwrap(),
    );
    let worker = SchedulerWorker::new_with_rate_limiter(
        persistence.clone(),
        bus,
        "worker-a",
        500,
        1,
        limiter,
    )
    .unwrap();

    assert_eq!(worker.tick("project-a", 1_000).await.unwrap(), 1);
    assert_eq!(
        worker.tick("project-a", 1_001).await,
        Err(WorkerError::RateLimited {
            retry_after_ms: 1_000,
        })
    );
    assert!(persistence
        .get_run("project-a", "run-b")
        .await
        .unwrap()
        .lease_owner
        .is_none());
}
