use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, ProjectId, Resource, SkillFileInput,
    SkillFileRole, SkillManifest, SkillScope, SkillStatus, TraceId,
};
use agent_runtime::skill_creation::{
    SkillCreationPolicy, SkillCreationRequest, SkillCreationService, SkillDiscardRequest,
    SKILL_CREATE_CAPABILITY,
};
use agent_runtime::skill_testing::{SkillFixture, SkillTestStep};
use agent_runtime::{migrations::run_migrations, sqlite::SqliteStorage, SqliteSkillRepository};

async fn repository() -> (SkillCreationService, SqliteSkillRepository, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project = ProjectId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Creation Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .bind(project.to_string())
        .execute(storage.pool())
        .await
        .unwrap();
    let skills = SqliteSkillRepository::new(storage.pool().clone());
    (SkillCreationService::new(skills.clone()), skills, project)
}

fn creation_input(project_id: ProjectId) -> (String, SkillFixture, agent_core::SkillId, TraceId) {
    let mut manifest = SkillManifest::new("creator", "1.0.0", SkillScope::Project);
    manifest.files.push(agent_core::SkillFile {
        path: "tests/basic.json".into(),
        role: SkillFileRole::Test,
        digest: "b".repeat(64),
    });
    manifest.tests.push("tests/basic.json".into());
    let trace_id = manifest.trace.trace_id;
    let skill_id = manifest.id;
    let version = manifest.version.clone();
    let document = format!(
        "---\n{}\n---\n# Instructions\nKeep this draft declarative and bounded.",
        serde_json::to_string(&manifest).unwrap()
    );
    let fixture = SkillFixture::new(
        project_id,
        skill_id,
        version,
        trace_id,
        vec![SkillTestStep::AssertLabel {
            label: "safe-creation".into(),
        }],
        4,
    )
    .unwrap();
    (document, fixture, skill_id, trace_id)
}

fn request(
    project_id: ProjectId,
    document: String,
    fixture: SkillFixture,
    trace_id: TraceId,
) -> SkillCreationRequest {
    let capability =
        Capability::new(Resource::Skill, Action::Create).with_scope(project_id.to_string());
    SkillCreationRequest {
        project_id,
        actor_id: "creator-agent".into(),
        capability: capability.clone(),
        policy: SkillCreationPolicy {
            allow: true,
            allowed_capabilities: CapabilitySet::new().insert(capability),
            max_document_bytes: 64 * 1024,
        },
        budget: BudgetLimits::default(),
        trace_id,
        document,
        files: vec![SkillFileInput {
            path: "tests/basic.json".into(),
            content: "{\"case\":\"safe\"}".into(),
        }],
        fixture,
        dependency_graph: Vec::new(),
    }
}

#[tokio::test]
// @spec:AC-803
async fn valid_creation_persists_only_a_project_draft_with_validation_evidence() {
    let (service, repository, project_id) = repository().await;
    let (document, fixture, skill_id, trace_id) = creation_input(project_id);
    let result = service
        .create(request(project_id, document, fixture, trace_id))
        .await
        .unwrap();

    assert!(result.changed);
    assert_eq!(result.record.skill.status, SkillStatus::Draft);
    assert_eq!(result.record.skill.project_id, Some(project_id));
    assert!(result.record.skill.pinned_version.is_none());
    assert_eq!(
        result.validation.status,
        agent_runtime::skill_validation::SkillValidationStatus::Passed
    );
    assert_eq!(result.validation.project_id, project_id);
    assert_eq!(result.validation.skill_id, skill_id);
    let redacted = serde_json::json!({
        "skill_id": result.record.skill.manifest.id,
        "content_hash": result.record.content_hash,
        "validation_report": result.validation.report_digest,
    });
    assert!(!redacted.to_string().contains("Keep this draft"));

    let stored = repository
        .get(SkillScope::Project, Some(&project_id), &skill_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.skill.status, SkillStatus::Draft);
}

#[tokio::test]
// @spec:AC-804
async fn identical_creation_is_idempotent_and_does_not_create_a_second_version() {
    let (service, repository, project_id) = repository().await;
    let (document, fixture, skill_id, trace_id) = creation_input(project_id);
    let first = service
        .create(request(
            project_id,
            document.clone(),
            fixture.clone(),
            trace_id,
        ))
        .await
        .unwrap();
    let second = service
        .create(request(project_id, document, fixture, trace_id))
        .await
        .unwrap();

    assert!(first.changed);
    assert!(!second.changed);
    assert_eq!(first.record.content_hash, second.record.content_hash);
    assert_eq!(
        repository
            .list_versions(SkillScope::Project, Some(&project_id), &skill_id)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
// @spec:AC-805
async fn privileged_fixture_is_rejected_before_any_skill_head_is_persisted() {
    let (service, repository, project_id) = repository().await;
    let (document, mut fixture, skill_id, trace_id) = creation_input(project_id);
    fixture.steps = vec![SkillTestStep::ExecuteScript {
        source: "do-not-run".into(),
    }];

    let error = service
        .create(request(project_id, document, fixture, trace_id))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        agent_core::DomainError::PermissionDenied { capability, .. }
            if capability == SKILL_CREATE_CAPABILITY
    ));
    assert!(repository
        .get(SkillScope::Project, Some(&project_id), &skill_id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
// @spec:AC-806
async fn missing_scoped_capability_or_budget_fails_closed_without_persistence() {
    let (service, repository, project_id) = repository().await;
    let (document, fixture, skill_id, trace_id) = creation_input(project_id);
    let mut invalid = request(project_id, document, fixture, trace_id);
    invalid.policy.allowed_capabilities = CapabilitySet::new();
    invalid.budget.max_tokens = 0;

    assert!(service.create(invalid).await.is_err());
    assert!(repository
        .get(SkillScope::Project, Some(&project_id), &skill_id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
// @spec:AC-807
async fn discarding_initial_draft_archives_it_without_touching_active_state() {
    let (service, repository, project_id) = repository().await;
    let (document, fixture, skill_id, trace_id) = creation_input(project_id);
    let created = service
        .create(request(project_id, document, fixture, trace_id))
        .await
        .unwrap();
    let capability =
        Capability::new(Resource::Skill, Action::Delete).with_scope(project_id.to_string());
    let discarded = service
        .discard(SkillDiscardRequest {
            project_id,
            skill_id,
            version: created.record.skill.manifest.version.clone(),
            actor_id: "creator-agent".into(),
            capability: capability.clone(),
            policy: SkillCreationPolicy {
                allow: true,
                allowed_capabilities: CapabilitySet::new().insert(capability),
                max_document_bytes: 64 * 1024,
            },
            trace_id,
            expected_revision: created.record.revision,
            confirmed: true,
        })
        .await
        .unwrap();

    assert_eq!(discarded.skill.status, SkillStatus::Archived);
    assert!(discarded.skill.pinned_version.is_none());
    let again = service
        .discard(SkillDiscardRequest {
            project_id,
            skill_id,
            version: created.record.skill.manifest.version,
            actor_id: "creator-agent".into(),
            capability: Capability::new(Resource::Skill, Action::Delete)
                .with_scope(project_id.to_string()),
            policy: SkillCreationPolicy {
                allow: true,
                allowed_capabilities: CapabilitySet::new().insert(
                    Capability::new(Resource::Skill, Action::Delete)
                        .with_scope(project_id.to_string()),
                ),
                max_document_bytes: 64 * 1024,
            },
            trace_id,
            expected_revision: discarded.revision,
            confirmed: true,
        })
        .await
        .unwrap();
    assert_eq!(again.skill.status, SkillStatus::Archived);
    assert_eq!(
        repository
            .list_versions(SkillScope::Project, Some(&project_id), &skill_id)
            .await
            .unwrap()
            .len(),
        1
    );
}
