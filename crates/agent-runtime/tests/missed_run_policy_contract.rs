use agent_runtime::migrations::run_migrations;
use agent_runtime::missed_run_policy::{evaluate, MissedAction, MissedPolicy};
use agent_runtime::scheduler_persistence::{
    MissedOutcomeRecord, PersistenceError, SchedulerPersistence,
};
use agent_runtime::sqlite::SqliteStorage;

async fn setup() -> (SqliteStorage, SchedulerPersistence) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-a', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')").execute(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO scheduler_jobs (project_id, job_id, owner_id, trigger_kind, trigger_value, target_kind, target_id, target_version, timezone, enabled, lifecycle, concurrency_limit, missed_run_policy, revision, created_at_ms, updated_at_ms) VALUES ('project-a', 'job-a', 'owner-a', 'interval', '60', 'workflow', 'workflow-a', 1, 'UTC', 1, 'active', 1, 'skip', 0, 1, 1)").execute(storage.pool()).await.unwrap();
    (
        storage.clone(),
        SchedulerPersistence::new(storage.pool().clone()),
    )
}

// @spec:AC-1231
#[test]
fn replay_is_deterministic_and_bounded_for_short_and_long_downtime() {
    let policy = MissedPolicy {
        action: MissedAction::CatchUp,
        interval_ms: 100,
        lateness_window_ms: 10_000,
        catch_up_cap: 3,
        policy_version: "v1".into(),
    };
    let first = evaluate(&policy, 1_000, 1_450, true).unwrap();
    let second = evaluate(&policy, 1_000, 1_450, true).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 3);
    assert!(first
        .iter()
        .all(|item| item.action == MissedAction::CatchUp));
    let coalesced = evaluate(
        &MissedPolicy {
            action: MissedAction::Coalesce,
            ..policy
        },
        1_000,
        10_000,
        true,
    )
    .unwrap();
    assert_eq!(coalesced.len(), 1);
    assert_eq!(coalesced[0].action, MissedAction::Coalesce);
}

// @spec:AC-1232
#[test]
fn disabled_and_clock_skew_fail_closed() {
    let policy = MissedPolicy {
        action: MissedAction::CatchUp,
        interval_ms: 100,
        lateness_window_ms: 1_000,
        catch_up_cap: 3,
        policy_version: "v1".into(),
    };
    assert!(evaluate(&policy, 1_000, 900, true).unwrap().is_empty());
    assert!(evaluate(&policy, 1_000, 2_000, false).unwrap().is_empty());
    assert!(
        evaluate(
            &MissedPolicy {
                action: MissedAction::Pause,
                ..policy
            },
            1_000,
            2_000,
            true
        )
        .unwrap()[0]
            .action
            == MissedAction::Pause
    );
}

// @spec:AC-1233
#[tokio::test]
async fn outcomes_are_project_scoped_and_idempotent() {
    let (_storage, persistence) = setup().await;
    persistence
        .create_run("project-a", "run-a", "job-a", 1_000)
        .await
        .unwrap();
    let record = MissedOutcomeRecord {
        outcome_id: "outcome-a".into(),
        run_id: "run-a".into(),
        occurrence_at_ms: 1_000,
        action: "skip".into(),
        reason: "outside_window".into(),
        coalesce_key: None,
        policy_version: "v1".into(),
    };
    let outcome = persistence
        .record_missed_outcome("project-a", &record, 2_000)
        .await
        .unwrap();
    assert_eq!(outcome, "outcome-a");
    let replay = MissedOutcomeRecord {
        outcome_id: "outcome-b".into(),
        ..record.clone()
    };
    assert_eq!(
        persistence
            .record_missed_outcome("project-a", &replay, 2_001)
            .await
            .unwrap(),
        "outcome-a"
    );
    let other_project = MissedOutcomeRecord {
        outcome_id: "outcome-c".into(),
        ..record
    };
    assert!(matches!(
        persistence
            .record_missed_outcome("project-b", &other_project, 2_000)
            .await,
        Err(PersistenceError::NotFound)
    ));
}
