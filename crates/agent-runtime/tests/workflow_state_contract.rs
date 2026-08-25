use agent_runtime::migrations::run_migrations;
use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::workflow_state_repo::{
    CreateRun, StateError, StateStore, Transition, TransitionOutcome,
};

async fn store() -> StateStore {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-1', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .execute(storage.pool())
        .await
        .unwrap();
    StateStore::new(storage.pool().clone())
}

// @spec:AC-1033
#[tokio::test]
async fn migration_is_repeatable_and_run_identity_is_bounded() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-1', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .execute(storage.pool())
        .await
        .unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let store = StateStore::new(storage.pool().clone());
    store
        .create_run(CreateRun::new("project-1", "run-1", "workflow-1", 1).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        store
            .create_run(CreateRun::new("project-1", "run-1", "workflow-1", 1).unwrap())
            .await,
        Err(StateError::Duplicate)
    ));
}

// @spec:AC-1034
#[tokio::test]
async fn transition_is_compare_and_set_and_journaled_atomically() {
    let store = store().await;
    store
        .create_run(CreateRun::new("project-1", "run-1", "workflow-1", 1).unwrap())
        .await
        .unwrap();
    let transition = Transition::new(
        "project-1",
        "run-1",
        "node-1",
        0,
        "ready",
        "running",
        "key-1",
    )
    .unwrap();
    assert_eq!(
        store.transition(transition.clone()).await.unwrap(),
        TransitionOutcome::Applied {
            sequence: 1,
            generation: 0
        }
    );
    assert!(matches!(
        store
            .transition(Transition {
                expected_state: "ready".into(),
                idempotency_key: "key-conflict".into(),
                ..transition
            })
            .await,
        Err(StateError::Conflict)
    ));
    assert!(store
        .transition(
            Transition::new(
                "project-1",
                "run-1",
                "node-1",
                0,
                "running",
                "completed",
                "key-2"
            )
            .unwrap()
        )
        .await
        .is_ok());
}

// @spec:AC-1035
#[tokio::test]
async fn replay_is_idempotent_and_sensitive_checkpoint_is_rejected() {
    let store = store().await;
    store
        .create_run(CreateRun::new("project-1", "run-1", "workflow-1", 1).unwrap())
        .await
        .unwrap();
    let mut transition = Transition::new(
        "project-1",
        "run-1",
        "node-1",
        0,
        "ready",
        "running",
        "key-1",
    )
    .unwrap();
    transition.checkpoint = Some(serde_json::json!({"credential":"secret"}));
    assert!(matches!(
        store.transition(transition).await,
        Err(StateError::InvalidCheckpoint)
    ));
    let transition = Transition::new(
        "project-1",
        "run-1",
        "node-1",
        0,
        "ready",
        "running",
        "key-1",
    )
    .unwrap();
    assert!(matches!(
        store.transition(transition.clone()).await.unwrap(),
        TransitionOutcome::Applied { .. }
    ));
    assert_eq!(
        store.transition(transition).await.unwrap(),
        TransitionOutcome::Replayed {
            sequence: 1,
            generation: 0
        }
    );
}
