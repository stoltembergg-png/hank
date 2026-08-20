use agent_core::ids::{AgentId, ProjectId, SessionId};
use agent_core::session::{Message, MessageProvenance, MessageRole, MessageStatus, Session};
use agent_runtime::message_repo::{MessageStorageError, SqliteMessageRepository};
use agent_runtime::migrations::run_migrations;
use agent_runtime::session_repo::SqliteSessionRepository;
use agent_runtime::sqlite::SqliteStorage;
use sqlx::Row;

async fn setup() -> (
    SqliteStorage,
    SqliteMessageRepository,
    ProjectId,
    AgentId,
    Session,
) {
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
    let session = {
        let repository = SqliteSessionRepository::new(storage.pool().clone());
        let mut session = Session::new(project_id, agent_id, "corr_1").unwrap();
        session.activate().unwrap();
        repository.create(&session).await.unwrap();
        session
    };
    let pool = storage.pool().clone();
    (
        storage,
        SqliteMessageRepository::new(pool),
        project_id,
        agent_id,
        session,
    )
}

fn new_message(session_id: SessionId, sequence: u64) -> Message {
    Message::new(
        session_id,
        MessageRole::User,
        MessageProvenance::User,
        sequence,
        1,
        format!("message-{sequence}"),
    )
    .unwrap()
}

#[tokio::test]
async fn migration_adds_message_ordering_columns_and_is_idempotent() {
    let (storage, _, _, _, _) = setup().await;
    run_migrations(storage.pool()).await.unwrap();
    let columns = sqlx::query("PRAGMA table_info(messages)")
        .fetch_all(storage.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<_>>();
    for expected in [
        "schema_version",
        "provenance",
        "status",
        "correlation_id",
        "sequence",
        "generation",
        "parts",
    ] {
        assert!(columns.iter().any(|column| column == expected));
    }
}

#[tokio::test]
async fn append_get_list_and_partial_stream_recovery_roundtrip() {
    let (storage, repository, project_id, _, session) = setup().await;
    let first = new_message(session.id, 0);
    repository
        .append(&project_id, &session.id, &first)
        .await
        .unwrap();
    let mut partial = new_message(session.id, 1);
    partial.start_stream().unwrap();
    repository
        .append(&project_id, &session.id, &partial)
        .await
        .unwrap();
    let fetched = repository
        .get_by_id(&project_id, &session.id, &first.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.content, "message-0");
    assert_eq!(fetched.provenance, MessageProvenance::User);
    assert_eq!(fetched.status, MessageStatus::Draft);
    let listed = repository
        .list(&project_id, &session.id, 0, 1000)
        .await
        .unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[1].status, MessageStatus::Streaming);
    storage.close().await;
}

#[tokio::test]
async fn duplicate_stale_and_out_of_order_append_fail_without_data_loss() {
    let (storage, repository, project_id, _, session) = setup().await;
    let first = new_message(session.id, 0);
    repository
        .append(&project_id, &session.id, &first)
        .await
        .unwrap();
    assert!(matches!(
        repository.append(&project_id, &session.id, &first).await,
        Err(MessageStorageError::Conflict)
    ));
    let duplicate = new_message(session.id, 0);
    assert!(matches!(
        repository
            .append(&project_id, &session.id, &duplicate)
            .await,
        Err(MessageStorageError::DuplicateSequence)
    ));
    let out_of_order = new_message(session.id, 2);
    assert!(matches!(
        repository
            .append(&project_id, &session.id, &out_of_order)
            .await,
        Err(MessageStorageError::OutOfOrder { .. })
    ));
    assert_eq!(
        repository
            .list(&project_id, &session.id, 0, 10)
            .await
            .unwrap()
            .len(),
        1
    );
    storage.close().await;
}

#[tokio::test]
async fn terminal_update_is_idempotent_and_stale_update_rolls_back() {
    let (storage, repository, project_id, _, session) = setup().await;
    let mut message = new_message(session.id, 0);
    repository
        .append(&project_id, &session.id, &message)
        .await
        .unwrap();
    message.start_stream().unwrap();
    repository
        .update(&project_id, &message, MessageStatus::Draft)
        .await
        .unwrap();
    message.complete().unwrap();
    repository
        .update(&project_id, &message, MessageStatus::Streaming)
        .await
        .unwrap();
    repository
        .update(&project_id, &message, MessageStatus::Complete)
        .await
        .unwrap();
    assert!(matches!(
        repository
            .update(&project_id, &message, MessageStatus::Draft)
            .await,
        Err(MessageStorageError::Conflict)
    ));
    let persisted = repository
        .get_by_id(&project_id, &session.id, &message.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(persisted.status, MessageStatus::Complete);
    storage.close().await;
}

#[tokio::test]
async fn cross_session_access_and_pagination_are_bounded() {
    let (storage, repository, project_id, agent_id, session) = setup().await;
    let other = {
        let session_repo = SqliteSessionRepository::new(storage.pool().clone());
        let mut other = Session::new(project_id, agent_id, "corr_2").unwrap();
        other.activate().unwrap();
        session_repo.create(&other).await.unwrap();
        other
    };
    let message = new_message(session.id, 0);
    repository
        .append(&project_id, &session.id, &message)
        .await
        .unwrap();
    assert!(matches!(
        repository
            .get_by_id(&project_id, &other.id, &message.id)
            .await,
        Err(MessageStorageError::NotFound)
    ));
    assert!(matches!(
        repository
            .append(
                &project_id,
                &session.id,
                &Message::new(
                    other.id,
                    MessageRole::User,
                    MessageProvenance::User,
                    0,
                    1,
                    "foreign"
                )
                .unwrap()
            )
            .await,
        Err(MessageStorageError::ScopeMismatch)
    ));
    assert_eq!(
        repository
            .list(&project_id, &session.id, 0, 1000)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(repository
        .list(&project_id, &session.id, 0, 0)
        .await
        .is_err());
    storage.close().await;
}
