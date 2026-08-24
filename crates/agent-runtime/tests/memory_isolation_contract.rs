use agent_core::{Memory, MemoryProvenance, MemoryStatus, MemoryType, ProjectId, ProvenanceSource};
use agent_runtime::{
    context::{
        memory_selector::{MemoryContextCandidate, MemorySelectionRequest, MemorySelector},
        ContextSourceKind,
    },
    memory_repo::SqliteMemoryRepository,
    memory_service::{MemoryEdit, MemoryMutationContext, MemoryMutationService},
    migrations::run_migrations,
    sqlite::SqliteStorage,
};
use provider_core::CancellationToken;

async fn fixture() -> (SqliteMemoryRepository, ProjectId, ProjectId, Memory, Memory) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project_a = ProjectId::new();
    let project_b = ProjectId::new();
    for project in [project_a, project_b] {
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Isolation fixture', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project.to_string())
            .execute(storage.pool())
            .await
            .unwrap();
    }
    let memory_a = memory(project_a, "project-a-secret");
    let memory_b = memory(project_b, "project-b-secret");
    let repository = SqliteMemoryRepository::new(storage.pool().clone());
    repository.create(&memory_a).await.unwrap();
    repository.create(&memory_b).await.unwrap();
    (repository, project_a, project_b, memory_a, memory_b)
}

fn memory(project: ProjectId, content: &str) -> Memory {
    Memory::new_candidate(
        project,
        content.into(),
        MemoryType::Semantic,
        MemoryProvenance {
            source: ProvenanceSource::UserInput,
            extractor: None,
            confidence: 0.9,
            original_context: None,
        },
    )
}

fn mutation_context(project: ProjectId, operation_id: &str) -> MemoryMutationContext {
    MemoryMutationContext {
        project_id: project,
        actor_id: "operator-isolation".into(),
        trace_id: "trace-isolation".into(),
        capability: "memory.write".into(),
        policy_allowed: true,
        operation_id: operation_id.into(),
    }
}

// @spec:AC-778
#[tokio::test]
async fn repository_never_reads_or_lists_foreign_project_content() {
    let (repository, project_a, project_b, memory_a, memory_b) = fixture().await;
    assert_eq!(
        repository
            .get(&project_a, &memory_a.id)
            .await
            .unwrap()
            .unwrap()
            .content,
        "project-a-secret"
    );
    assert!(repository
        .get(&project_a, &memory_b.id)
        .await
        .unwrap()
        .is_none());
    let listed = repository.list_active(&project_a, 100, 0).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].project_id, project_a);
    assert_ne!(listed[0].content, "project-b-secret");
    assert!(repository
        .get(&project_b, &memory_b.id)
        .await
        .unwrap()
        .is_some());
}

// @spec:AC-778
#[tokio::test]
async fn mutation_rejects_foreign_scope_before_effect() {
    let (repository, project_a, project_b, memory_a, memory_b) = fixture().await;
    let service = MemoryMutationService::new(repository.clone());
    let result = service
        .execute(
            mutation_context(project_a, "foreign-attempt"),
            memory_b.id,
            1,
            MemoryEdit::Reject,
        )
        .await;
    assert!(result.is_err());
    assert_eq!(
        repository
            .get(&project_b, &memory_b.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        MemoryStatus::Candidate
    );
    assert_eq!(
        repository
            .get(&project_a, &memory_a.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        MemoryStatus::Candidate
    );
}

// @spec:AC-778
#[test]
fn selector_omits_foreign_project_content_before_context_creation() {
    let project_a = ProjectId::new();
    let project_b = ProjectId::new();
    let result = MemorySelector::select(MemorySelectionRequest {
        project_id: project_a,
        agent_id: agent_core::AgentId::new(),
        candidates: vec![
            MemoryContextCandidate {
                memory_id: "a".into(),
                project_id: project_a,
                agent_id: None,
                status: MemoryStatus::Approved,
                content: "project-a-safe".into(),
                estimated_tokens: 2,
                confidence: 0.9,
                importance: 0.9,
                recency_rank: 1,
                provenance: ProvenanceSource::UserInput,
                duplicate_key: None,
                policy_allowed: true,
                capability_allowed: true,
            },
            MemoryContextCandidate {
                memory_id: "b".into(),
                project_id: project_b,
                agent_id: None,
                status: MemoryStatus::Approved,
                content: "project-b-secret".into(),
                estimated_tokens: 2,
                confidence: 1.0,
                importance: 1.0,
                recency_rank: 1,
                provenance: ProvenanceSource::UserInput,
                duplicate_key: None,
                policy_allowed: true,
                capability_allowed: true,
            },
        ],
        max_tokens: 10,
        trace_id: "trace-isolation".into(),
        cancellation: CancellationToken::new(),
    })
    .unwrap();
    assert_eq!(result.selected.len(), 1);
    assert_eq!(result.selected[0].context.kind, ContextSourceKind::Memory);
    assert_eq!(result.selected[0].context.content, "project-a-safe");
    assert_eq!(result.omitted.len(), 1);
}
