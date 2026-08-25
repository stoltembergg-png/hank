use agent_runtime::migrations::run_migrations;
use agent_runtime::scheduler::{
    ClaimError, JobStore, JobTarget, MissedRunPolicy, ScheduledJob, Trigger,
};
use agent_runtime::sqlite::SqliteStorage;

async fn setup() -> (SqliteStorage, JobStore) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES ('project-a', 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .execute(storage.pool()).await.unwrap();
    (storage.clone(), JobStore::new(storage.pool().clone()))
}

fn one_shot(id: &str, due: u64) -> ScheduledJob {
    ScheduledJob::new(
        "project-a",
        id,
        "owner-a",
        Trigger::OneShot { at_ms: due },
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

// @spec:AC-1181
#[tokio::test]
async fn due_future_and_expiration_policies_are_enforced() {
    let (_storage, store) = setup().await;
    store.create(one_shot("future", 2_000)).await.unwrap();
    assert!(matches!(
        store
            .claim_one_shot("project-a", "future", "owner-a", "claim-a", 1_999)
            .await,
        Err(ClaimError::NotDue)
    ));
    assert!(store
        .claim_one_shot("project-a", "future", "owner-a", "claim-a", 2_000)
        .await
        .is_ok());
    let expired = one_shot("expired", 1_000).with_expiration(1_500).unwrap();
    store.create(expired.clone()).await.unwrap();
    assert!(matches!(
        store
            .claim_one_shot("project-a", "expired", "owner-a", "claim-b", 1_500)
            .await,
        Err(ClaimError::Expired)
    ));
}

// @spec:AC-1182
#[tokio::test]
async fn concurrent_claimers_produce_exactly_one_consumed_claim() {
    let (_storage, store) = setup().await;
    store.create(one_shot("race", 1_000)).await.unwrap();
    let first = store.clone();
    let second = store.clone();
    let (a, b) = tokio::join!(
        first.claim_one_shot("project-a", "race", "owner-a", "claim-a", 1_000),
        second.claim_one_shot("project-a", "race", "owner-a", "claim-b", 1_000),
    );
    assert_eq!(
        [a.is_ok(), b.is_ok()]
            .into_iter()
            .filter(|value| *value)
            .count(),
        1
    );
}

// @spec:AC-1183
#[tokio::test]
async fn replay_scope_and_lifecycle_fail_closed_without_mutation() {
    let (_storage, store) = setup().await;
    store.create(one_shot("replay", 1_000)).await.unwrap();
    let receipt = store
        .claim_one_shot("project-a", "replay", "owner-a", "claim-a", 1_000)
        .await
        .unwrap();
    assert_eq!(
        store
            .claim_one_shot("project-a", "replay", "owner-a", "claim-a", 2_000)
            .await
            .unwrap(),
        receipt
    );
    assert!(matches!(
        store
            .claim_one_shot("project-a", "replay", "owner-a", "claim-b", 2_000)
            .await,
        Err(ClaimError::Consumed)
    ));
    assert!(matches!(
        store
            .claim_one_shot("project-b", "replay", "owner-a", "claim-c", 2_000)
            .await,
        Err(ClaimError::NotFound)
    ));
    let mut disabled = one_shot("disabled", 1_000);
    disabled.enabled = false;
    store.create(disabled.clone()).await.unwrap();
    assert!(matches!(
        store
            .claim_one_shot("project-a", "disabled", "owner-a", "claim-d", 2_000)
            .await,
        Err(ClaimError::Disabled)
    ));
}
