use agent_core::task_mapping::{MappingState, TaskWorkspaceMapping, TaskWorkspaceMappingRegistry};
use agent_core::{ProjectId, RunId, TaskId, TraceId};
use agent_runtime::{
    migrations::run_migrations,
    sqlite::{SqliteStorage, SqliteStorageConfig},
    TaskWorkspaceMappingRepository,
};

fn mapping(project_id: ProjectId, task_id: TaskId, worktree_id: &str) -> TaskWorkspaceMapping {
    TaskWorkspaceMapping::new(
        project_id,
        task_id,
        "repo-1",
        worktree_id,
        "agent/task-1",
        RunId::new(),
        Some("pr-207".into()),
        TraceId::new(),
        "policy-r1",
    )
    .unwrap()
}

async fn setup() -> (SqliteStorage, TaskWorkspaceMappingRepository, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project = ProjectId::new();
    sqlx::query(
        "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Mapping project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')",
    )
    .bind(project.to_string())
    .execute(storage.pool())
    .await
    .unwrap();
    let repository = TaskWorkspaceMappingRepository::new(storage.pool().clone());
    (storage, repository, project)
}

#[tokio::test]
async fn migration_is_idempotent_and_repository_roundtrip_survives_restart() {
    // @spec:AC-1321
    let (storage, repository, project) = setup().await;
    run_migrations(storage.pool()).await.unwrap();
    let value = mapping(project, TaskId::new(), "wt-1");
    repository.create(&value, 100).await.unwrap();

    let reloaded = TaskWorkspaceMappingRepository::new(storage.pool().clone())
        .get(project, value.task_id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded, value);
    assert_eq!(reloaded.state(), MappingState::Active);
    assert_eq!(reloaded.revision(), 1);
    storage.close().await;
}

#[tokio::test]
async fn file_database_roundtrip_survives_close_and_reopen() {
    // @spec:AC-1321
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("task-mapping.sqlite");
    let project = ProjectId::new();
    let value = mapping(project, TaskId::new(), "wt-restart");

    {
        let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&database_path))
            .await
            .unwrap();
        run_migrations(storage.pool()).await.unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Restart project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')",
        )
        .bind(project.to_string())
        .execute(storage.pool())
        .await
        .unwrap();
        TaskWorkspaceMappingRepository::new(storage.pool().clone())
            .create(&value, 100)
            .await
            .unwrap();
        storage.close().await;
    }

    let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&database_path))
        .await
        .unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let reloaded = TaskWorkspaceMappingRepository::new(storage.pool().clone())
        .get(project, value.task_id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded, value);
    storage.close().await;
}

#[tokio::test]
async fn repository_enforces_project_scoped_uniqueness_and_lists_deterministically() {
    // @spec:AC-1317 @spec:AC-1318
    let (storage, repository, project) = setup().await;
    let first = mapping(project, TaskId::new(), "wt-1");
    repository.create(&first, 100).await.unwrap();
    assert!(repository.create(&first, 101).await.is_err());

    let same_worktree = mapping(project, TaskId::new(), "wt-1");
    assert!(repository.create(&same_worktree, 102).await.is_err());

    let other_project = ProjectId::new();
    sqlx::query(
        "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Other project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')",
    )
    .bind(other_project.to_string())
    .execute(storage.pool())
    .await
    .unwrap();
    let foreign = mapping(other_project, first.task_id(), "wt-1");
    repository.create(&foreign, 103).await.unwrap();
    assert_eq!(repository.list(project).await.unwrap(), vec![first.clone()]);
    assert!(repository
        .get(other_project, first.task_id())
        .await
        .unwrap()
        .is_some());
    assert_eq!(
        repository
            .get(project, foreign.task_id())
            .await
            .unwrap()
            .as_ref(),
        Some(&first)
    );
    storage.close().await;
}

#[tokio::test]
async fn repository_compare_and_set_preserves_lifecycle_and_rejects_stale_updates() {
    // @spec:AC-1319 @spec:AC-1320
    let (storage, repository, project) = setup().await;
    let value = mapping(project, TaskId::new(), "wt-1");
    repository.create(&value, 100).await.unwrap();

    let mut registry = TaskWorkspaceMappingRegistry::new(4).unwrap();
    registry.register(value.clone()).unwrap();
    let detached = registry.detach(project, value.task_id(), 1, 200).unwrap();
    repository.update(&detached, 1, 200).await.unwrap();
    assert_eq!(
        repository
            .get(project, value.task_id())
            .await
            .unwrap()
            .unwrap()
            .state(),
        MappingState::Detached
    );
    assert!(repository.update(&detached, 1, 201).await.is_err());
    storage.close().await;
}
