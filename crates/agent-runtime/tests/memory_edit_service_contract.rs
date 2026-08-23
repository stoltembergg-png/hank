use agent_core::{Memory, MemoryProvenance, MemoryType, ProjectId, ProvenanceSource};
use agent_runtime::{
    memory_repo::SqliteMemoryRepository,
    memory_service::{MemoryEdit, MemoryMutationContext, MemoryMutationService},
    migrations::run_migrations,
    sqlite::SqliteStorage,
};

async fn repository() -> (SqliteMemoryRepository, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project = ProjectId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Editable Memory', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .bind(project.to_string())
        .execute(storage.pool())
        .await
        .unwrap();
    (SqliteMemoryRepository::new(storage.pool().clone()), project)
}

fn memory(project: ProjectId) -> Memory {
    Memory::new_candidate(
        project,
        "candidate text".into(),
        MemoryType::Semantic,
        MemoryProvenance {
            source: ProvenanceSource::UserInput,
            extractor: None,
            confidence: 0.9,
            original_context: None,
        },
    )
}

fn context(project: ProjectId) -> MemoryMutationContext {
    MemoryMutationContext {
        project_id: project,
        actor_id: "operator-1".into(),
        trace_id: "trace-edit-1".into(),
        capability: "memory.write".into(),
        policy_allowed: true,
        operation_id: "op-1".into(),
    }
}

// @spec:AC-773
#[tokio::test]
async fn valid_edit_and_approval_require_explicit_scoped_context() {
    let (repo, project) = repository().await;
    let item = memory(project);
    let id = item.id;
    repo.create(&item).await.unwrap();
    let service = MemoryMutationService::new(repo.clone());
    let updated = service
        .execute(
            context(project),
            id,
            1,
            MemoryEdit::Update {
                content: "corrected text".into(),
                summary: Some("reviewed".into()),
                importance: 0.8,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.content, "corrected text");
    assert_eq!(updated.version, 2);
    let approved = service
        .execute(
            MemoryMutationContext {
                operation_id: "op-2".into(),
                ..context(project)
            },
            id,
            2,
            MemoryEdit::Approve,
        )
        .await
        .unwrap();
    assert_eq!(approved.status, agent_core::MemoryStatus::Approved);
}

// @spec:AC-774
#[tokio::test]
async fn archive_and_restore_are_explicit_lifecycle_operations() {
    let (repo, project) = repository().await;
    let item = memory(project);
    let id = item.id;
    repo.create(&item).await.unwrap();
    let service = MemoryMutationService::new(repo.clone());
    let archived = service
        .execute(context(project), id, 1, MemoryEdit::Archive)
        .await
        .unwrap();
    assert_eq!(archived.status, agent_core::MemoryStatus::Archived);
    let restored = service
        .execute(
            MemoryMutationContext {
                operation_id: "op-2".into(),
                ..context(project)
            },
            id,
            2,
            MemoryEdit::Restore,
        )
        .await
        .unwrap();
    assert_eq!(restored.status, agent_core::MemoryStatus::Approved);
}

// @spec:AC-775
#[tokio::test]
async fn wrong_scope_policy_capability_oversized_and_stale_version_fail_without_mutation() {
    let (repo, project) = repository().await;
    let item = memory(project);
    let id = item.id;
    repo.create(&item).await.unwrap();
    let service = MemoryMutationService::new(repo.clone());
    let mut foreign = context(ProjectId::new());
    assert!(service
        .execute(foreign.clone(), id, 1, MemoryEdit::Reject)
        .await
        .is_err());
    foreign.project_id = project;
    foreign.policy_allowed = false;
    assert!(service
        .execute(foreign.clone(), id, 1, MemoryEdit::Reject)
        .await
        .is_err());
    foreign.policy_allowed = true;
    foreign.capability = "memory.read".into();
    assert!(service
        .execute(foreign.clone(), id, 1, MemoryEdit::Reject)
        .await
        .is_err());
    foreign.capability = "memory.write".into();
    assert!(service
        .execute(foreign.clone(), id, 99, MemoryEdit::Reject)
        .await
        .is_err());
    assert!(service
        .execute(
            MemoryMutationContext {
                operation_id: "op-large".into(),
                ..foreign
            },
            id,
            1,
            MemoryEdit::Update {
                content: "x".repeat(20_000),
                summary: None,
                importance: 0.5
            },
        )
        .await
        .is_err());
    assert_eq!(
        repo.get(&project, &id).await.unwrap().unwrap().status,
        agent_core::MemoryStatus::Candidate
    );
}

// @spec:AC-776
#[tokio::test]
async fn reject_is_explicit_and_duplicate_operation_is_not_replayed() {
    let (repo, project) = repository().await;
    let item = memory(project);
    let id = item.id;
    repo.create(&item).await.unwrap();
    let service = MemoryMutationService::new(repo.clone());
    let first = service
        .execute(context(project), id, 1, MemoryEdit::Reject)
        .await
        .unwrap();
    assert_eq!(first.status, agent_core::MemoryStatus::Rejected);
    let duplicate = service
        .execute(context(project), id, 1, MemoryEdit::Reject)
        .await;
    assert!(duplicate.is_err());
    assert_eq!(
        repo.get(&project, &id).await.unwrap().unwrap().status,
        agent_core::MemoryStatus::Rejected
    );
}
