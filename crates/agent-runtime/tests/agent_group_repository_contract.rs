use agent_core::{AgentGroup, AgentId, ProjectId, TraceId};
use agent_runtime::{
    migrations::run_migrations, sqlite::SqliteStorage, SqliteAgentGroupRepository,
};

async fn repository() -> (SqliteAgentGroupRepository, ProjectId, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let first = ProjectId::new();
    let second = ProjectId::new();
    for project in [first, second] {
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Group Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project.to_string()).execute(storage.pool()).await.unwrap();
    }
    (
        SqliteAgentGroupRepository::new(storage.pool().clone()),
        first,
        second,
    )
}

fn group(project: ProjectId) -> AgentGroup {
    AgentGroup::new(
        project,
        "research-team".into(),
        AgentId::new(),
        TraceId::new(),
    )
}

#[tokio::test]
// @spec:AC-861
async fn create_get_and_project_scoped_lookup_are_transactional() {
    let (repository, project, other) = repository().await;
    let value = group(project);
    let created = repository.create(&value).await.unwrap();
    assert_eq!(created.revision, 1);
    assert_eq!(
        repository
            .get(project, value.id)
            .await
            .unwrap()
            .unwrap()
            .group
            .id,
        value.id
    );
    assert!(repository.get(other, value.id).await.unwrap().is_none());
}

#[tokio::test]
// @spec:AC-862
async fn duplicate_and_stale_updates_fail_closed() {
    let (repository, project, _) = repository().await;
    let value = group(project);
    repository.create(&value).await.unwrap();
    assert!(repository.create(&value).await.is_err());
    assert!(repository.archive(project, value.id, 99).await.is_err());
}

#[tokio::test]
// @spec:AC-863
async fn archive_is_idempotent_and_readback_preserves_policy() {
    let (repository, project, _) = repository().await;
    let value = group(project);
    repository.create(&value).await.unwrap();
    let archived = repository.archive(project, value.id, 1).await.unwrap();
    assert_eq!(archived.revision, 2);
    assert_eq!(
        archived.group.lifecycle,
        agent_core::AgentGroupLifecycle::Archived
    );
    let repeated = repository.archive(project, value.id, 2).await.unwrap();
    assert_eq!(repeated.revision, 2);
}
