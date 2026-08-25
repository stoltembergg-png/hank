use agent_runtime::migrations::run_migrations;
use agent_runtime::scheduler::{
    JobError, JobStore, JobTarget, MissedRunPolicy, ScheduledJob, Trigger,
};
use agent_runtime::sqlite::SqliteStorage;

async fn setup() -> (SqliteStorage, JobStore) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-a', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .execute(storage.pool()).await.unwrap();
    (storage.clone(), JobStore::new(storage.pool().clone()))
}

fn job(job_id: &str) -> ScheduledJob {
    ScheduledJob::new(
        "project-a",
        job_id,
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
    .unwrap()
}

// @spec:AC-1121
#[tokio::test]
async fn known_triggers_and_versioned_target_roundtrip() {
    let (_storage, store) = setup().await;
    for trigger in [
        Trigger::OneShot { at_ms: 10 },
        Trigger::Interval { seconds: 60 },
        Trigger::Cron {
            expression: "0 * * * *".into(),
        },
        Trigger::Event {
            name: "deploy".into(),
        },
        Trigger::Dependency {
            job_id: "job-parent".into(),
        },
    ] {
        let mut record = job(&format!("job-{}", trigger.kind()));
        record.trigger = trigger;
        store.create(record).await.unwrap();
    }
    assert!(matches!(
        store.get("project-a", "job-interval").await.unwrap().target,
        JobTarget::Workflow { version: 1, .. }
    ));
    assert!(matches!(
        store.create(job("job-interval")).await,
        Err(JobError::Duplicate)
    ));
}

// @spec:AC-1122
#[tokio::test]
async fn bounds_and_lifecycle_fail_closed() {
    let (_storage, store) = setup().await;
    assert!(matches!(
        ScheduledJob::new(
            "project-a",
            "bad",
            "owner",
            Trigger::Interval { seconds: 1 },
            JobTarget::Workflow {
                workflow_id: "w".into(),
                version: 1
            },
            "UTC",
            1,
            MissedRunPolicy::Skip,
        ),
        Err(JobError::InvalidFrequency)
    ));
    assert!(matches!(
        ScheduledJob::new(
            "project-a",
            "bad",
            "owner",
            Trigger::Interval { seconds: 60 },
            JobTarget::Workflow {
                workflow_id: "w".into(),
                version: 1
            },
            "",
            1,
            MissedRunPolicy::Skip,
        ),
        Err(JobError::InvalidIdentity)
    ));
    let mut record = job("job-life");
    record.lifecycle = "disabled".into();
    store.create(record).await.unwrap();
    assert_eq!(
        store.get("project-a", "job-life").await.unwrap().lifecycle,
        "disabled"
    );
}

// @spec:AC-1123
#[tokio::test]
async fn migration_is_repeatable_and_stale_update_does_not_overwrite() {
    let (storage, store) = setup().await;
    run_migrations(storage.pool()).await.unwrap();
    store.create(job("job-revision")).await.unwrap();
    let current = store.get("project-a", "job-revision").await.unwrap();
    assert!(store
        .update(current.clone(), current.revision + 1)
        .await
        .is_err());
    let updated = store
        .update(current.clone(), current.revision)
        .await
        .unwrap();
    assert_eq!(updated.revision, current.revision + 1);
}
