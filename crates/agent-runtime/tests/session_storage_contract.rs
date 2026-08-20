use agent_core::ids::{AgentId, ProjectId};
use agent_core::session::{Session, SessionStatus};
use agent_runtime::migrations::run_migrations;
use agent_runtime::session_repo::{SessionStorageError, SqliteSessionRepository};
use agent_runtime::sqlite::SqliteStorage;
use sqlx::Row;

async fn setup() -> (SqliteStorage, SqliteSessionRepository, ProjectId, AgentId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project_id = ProjectId::new();
    let agent_id = AgentId::new();
    sqlx::query(
        "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Project', 'active', 'owner', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '{}')",
    )
    .bind(project_id.to_string())
    .execute(storage.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agents (id, project_id, name, status, personality, policy, created_at, updated_at) VALUES (?, ?, 'Agent', 'active', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .bind(agent_id.to_string())
    .bind(project_id.to_string())
    .execute(storage.pool())
    .await
    .unwrap();
    let repository = SqliteSessionRepository::new(storage.pool().clone());
    (storage, repository, project_id, agent_id)
}

#[tokio::test]
async fn clean_migration_adds_versioned_session_columns_and_is_idempotent() {
    let (storage, _, _, _) = setup().await;
    run_migrations(storage.pool()).await.unwrap();
    let columns = sqlx::query("PRAGMA table_info(sessions)")
        .fetch_all(storage.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    for expected in [
        "schema_version",
        "correlation_id",
        "participants",
        "metadata",
        "budget_ref",
        "trace_id",
        "failure_reason",
    ] {
        assert!(columns.iter().any(|column| column == expected));
    }
}

#[tokio::test]
async fn repository_create_get_update_close_and_recovery_roundtrip() {
    let (storage, repository, project_id, agent_id) = setup().await;
    let mut session = Session::new(project_id, agent_id, "corr_1").unwrap();
    session.activate().unwrap();
    repository.create(&session).await.unwrap();
    let fetched = repository
        .get_by_id(&project_id, &session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, SessionStatus::Active);
    assert_eq!(fetched.correlation_id, "corr_1");

    let expected = fetched.updated_at;
    let mut updated = fetched;
    updated
        .add_metadata("mode", serde_json::json!("chat"))
        .unwrap();
    repository.update(&updated, expected).await.unwrap();
    let closed = repository.close(&project_id, &session.id).await.unwrap();
    assert_eq!(closed.status, SessionStatus::Closed);
    assert!(closed.closed_at.is_some());
    let recovered = repository
        .get_by_id(&project_id, &session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.status, SessionStatus::Closed);
    assert_eq!(
        recovered.metadata.get("mode"),
        Some(&serde_json::json!("chat"))
    );
    storage.close().await;
}

#[tokio::test]
async fn stale_concurrent_update_rolls_back_and_does_not_overwrite() {
    let (storage, repository, project_id, agent_id) = setup().await;
    let mut session = Session::new(project_id, agent_id, "corr_1").unwrap();
    session.activate().unwrap();
    repository.create(&session).await.unwrap();
    let first = repository
        .get_by_id(&project_id, &session.id)
        .await
        .unwrap()
        .unwrap();
    let second = first.clone();
    let expected = first.updated_at;
    let mut changed = first;
    changed
        .add_metadata("owner", serde_json::json!("first"))
        .unwrap();
    repository.update(&changed, expected).await.unwrap();

    let mut stale = second;
    stale
        .add_metadata("owner", serde_json::json!("stale"))
        .unwrap();
    assert!(matches!(
        repository.update(&stale, expected).await,
        Err(SessionStorageError::Conflict)
    ));
    let persisted = repository
        .get_by_id(&project_id, &session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted.metadata.get("owner"),
        Some(&serde_json::json!("first"))
    );
    storage.close().await;
}

#[tokio::test]
async fn project_scope_and_bounded_listing_are_enforced() {
    let (storage, repository, project_id, agent_id) = setup().await;
    let mut session = Session::new(project_id, agent_id, "corr_1").unwrap();
    session.activate().unwrap();
    repository.create(&session).await.unwrap();
    let foreign = ProjectId::new();
    assert!(matches!(
        repository.get_by_id(&foreign, &session.id).await,
        Err(SessionStorageError::ScopeMismatch)
    ));
    assert_eq!(
        repository.list(&project_id, 0, 1000).await.unwrap().len(),
        1
    );
    assert!(repository.list(&project_id, 0, 0).await.is_err());
    storage.close().await;
}

#[tokio::test]
async fn duplicate_create_and_missing_close_are_typed() {
    let (storage, repository, project_id, agent_id) = setup().await;
    let mut session = Session::new(project_id, agent_id, "corr_1").unwrap();
    session.activate().unwrap();
    repository.create(&session).await.unwrap();
    assert!(matches!(
        repository.create(&session).await,
        Err(SessionStorageError::Conflict)
    ));
    assert!(matches!(
        repository
            .close(
                &project_id,
                &Session::new(project_id, agent_id, "corr_2").unwrap().id
            )
            .await,
        Err(SessionStorageError::NotFound)
    ));
    storage.close().await;
}
