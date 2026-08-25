use agent_runtime::migrations::run_migrations;
use agent_runtime::scheduler_concurrency::{
    AdmissionError, AdmissionRequest, SchedulerConcurrency,
};
use agent_runtime::scheduler_persistence::SchedulerPersistence;
use agent_runtime::sqlite::SqliteStorage;

fn request(
    project: &str,
    key: &str,
    run: &str,
    owner: &str,
    limit: u32,
    now: u64,
    expiry: u64,
) -> AdmissionRequest {
    AdmissionRequest {
        project_id: project.into(),
        concurrency_key: key.into(),
        run_id: run.into(),
        lease_owner: owner.into(),
        limit,
        now_ms: now,
        lease_expires_at_ms: expiry,
    }
}

async fn setup() -> (SqliteStorage, SchedulerPersistence, SchedulerConcurrency) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-a', 'Project A', 'active', 'owner', '2026-01-01', '2026-01-01', '{}'), ('project-b', 'Project B', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')").execute(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO scheduler_jobs (project_id, job_id, owner_id, trigger_kind, trigger_value, target_kind, target_id, target_version, timezone, enabled, lifecycle, concurrency_limit, missed_run_policy, revision, created_at_ms, updated_at_ms) VALUES ('project-a', 'job-a', 'owner-a', 'interval', '60', 'workflow', 'workflow-a', 1, 'UTC', 1, 'active', 1, 'skip', 0, 1, 1), ('project-b', 'job-b', 'owner-b', 'interval', '60', 'workflow', 'workflow-b', 1, 'UTC', 1, 'active', 1, 'skip', 0, 1, 1)").execute(storage.pool()).await.unwrap();
    let persistence = SchedulerPersistence::new(storage.pool().clone());
    persistence
        .create_run("project-a", "run-a", "job-a", 1_000)
        .await
        .unwrap();
    persistence
        .create_run("project-a", "run-b", "job-a", 1_000)
        .await
        .unwrap();
    persistence
        .create_run("project-b", "run-a", "job-b", 1_000)
        .await
        .unwrap();
    (
        storage.clone(),
        persistence,
        SchedulerConcurrency::new(storage.pool().clone()),
    )
}

// @spec:AC-1241
#[tokio::test]
async fn two_workers_admit_only_one_with_limit_one() {
    let (_storage, _persistence, admission) = setup().await;
    admission
        .admit(&request(
            "project-a",
            "job:job-a",
            "run-a",
            "worker-a",
            1,
            1_000,
            2_000,
        ))
        .await
        .unwrap();
    assert!(matches!(
        admission
            .admit(&request(
                "project-a",
                "job:job-a",
                "run-b",
                "worker-b",
                1,
                1_001,
                2_001
            ))
            .await,
        Err(AdmissionError::CapacityReached)
    ));
}

// @spec:AC-1242
#[tokio::test]
async fn expired_lease_reuses_slot_and_wrong_owner_cannot_release() {
    let (_storage, _persistence, admission) = setup().await;
    admission
        .admit(&request(
            "project-a",
            "job:job-a",
            "run-a",
            "worker-a",
            1,
            1_000,
            2_000,
        ))
        .await
        .unwrap();
    assert!(!admission
        .release("project-a", "job:job-a", "run-a", "worker-b")
        .await
        .unwrap());
    admission
        .admit(&request(
            "project-a",
            "job:job-a",
            "run-b",
            "worker-b",
            1,
            2_000,
            3_000,
        ))
        .await
        .unwrap();
    assert!(admission
        .release("project-a", "job:job-a", "run-b", "worker-b")
        .await
        .unwrap());
    admission
        .admit(&request(
            "project-a",
            "job:job-a",
            "run-a",
            "worker-a",
            1,
            2_001,
            3_001,
        ))
        .await
        .unwrap();
}

// @spec:AC-1243
#[tokio::test]
async fn project_scope_isolated_and_limit_is_bounded() {
    let (_storage, _persistence, admission) = setup().await;
    admission
        .admit(&request(
            "project-a",
            "same-key",
            "run-a",
            "worker-a",
            1,
            1_000,
            2_000,
        ))
        .await
        .unwrap();
    admission
        .admit(&request(
            "project-b",
            "same-key",
            "run-a",
            "worker-b",
            1,
            1_000,
            2_000,
        ))
        .await
        .unwrap();
    assert!(matches!(
        admission
            .admit(&request(
                "project-a",
                "same-key",
                "run-b",
                "worker-b",
                0,
                1_000,
                2_000
            ))
            .await,
        Err(AdmissionError::InvalidLimit)
    ));
    assert!(matches!(
        admission
            .admit(&request(
                "project-a",
                "same-key",
                "run-b",
                "worker-b",
                65,
                1_000,
                2_000
            ))
            .await,
        Err(AdmissionError::InvalidLimit)
    ));
}
