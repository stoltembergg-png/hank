use agent_core::{Memory, MemoryProvenance, MemoryType, ProjectId, ProvenanceSource};
use agent_runtime::{migrations::run_migrations, sqlite::SqliteStorage, SqliteMemoryRepository};

async fn repository() -> (SqliteMemoryRepository, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project = ProjectId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Memory Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .bind(project.to_string())
        .execute(storage.pool())
        .await
        .unwrap();
    (SqliteMemoryRepository::new(storage.pool().clone()), project)
}

fn memory(project: ProjectId) -> Memory {
    Memory::new_candidate(
        project,
        "untrusted fact".into(),
        MemoryType::Semantic,
        MemoryProvenance {
            source: ProvenanceSource::UserInput,
            extractor: None,
            confidence: 0.9,
            original_context: None,
        },
    )
}

// @spec:AC-735
#[tokio::test]
async fn memory_crud_is_project_scoped_and_archived_is_not_active() {
    let (repo, project) = repository().await;
    let item = memory(project);
    repo.create(&item).await.unwrap();
    assert_eq!(
        repo.get(&project, &item.id).await.unwrap().unwrap().id,
        item.id
    );
    assert_eq!(repo.list_active(&project, 10, 0).await.unwrap().len(), 1);
    assert!(repo
        .get(&ProjectId::new(), &item.id)
        .await
        .unwrap()
        .is_none());
    repo.archive(&project, &item.id, 1).await.unwrap();
    assert!(repo.list_active(&project, 10, 0).await.unwrap().is_empty());
}

// @spec:AC-736
#[tokio::test]
async fn duplicate_memory_and_version_conflict_fail_without_mutation() {
    let (repo, project) = repository().await;
    let item = memory(project);
    repo.create(&item).await.unwrap();
    assert!(repo.create(&item).await.is_err());
    assert!(repo.archive(&project, &item.id, 99).await.is_err());
    assert_eq!(
        repo.get(&project, &item.id).await.unwrap().unwrap().version,
        1
    );
}
