use agent_core::{
    Action, AgentId, Capability, CapabilitySet, ProjectId, Resource, Skill, SkillId, SkillManifest,
    SkillParseRequest, SkillParser, SkillScope, SkillSourceKind, SkillStatus, TraceId,
};
use agent_protocol::events::{ApplicationEvent, EventKind};
use agent_runtime::event_bus::EventBus;
use agent_runtime::{
    migrations::run_migrations, sqlite::SqliteStorage, ProjectSkillBindingPolicy,
    ProjectSkillBindingRequest, ProjectSkillMutationRequest, ProjectSkillService,
    SqliteProjectSkillBindingRepository, SqliteSkillRepository,
};

async fn repositories() -> (
    SqliteSkillRepository,
    SqliteProjectSkillBindingRepository,
    ProjectId,
    ProjectId,
) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let first = ProjectId::new();
    let second = ProjectId::new();
    for project in [first, second] {
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Binding Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project.to_string())
            .execute(storage.pool())
            .await
            .unwrap();
    }
    (
        SqliteSkillRepository::new(storage.pool().clone()),
        SqliteProjectSkillBindingRepository::new(storage.pool().clone()),
        first,
        second,
    )
}

fn capability(project_id: ProjectId) -> Capability {
    Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string())
}

fn request(
    project_id: ProjectId,
    skill_id: SkillId,
    scope: SkillScope,
) -> ProjectSkillBindingRequest {
    let capability = capability(project_id);
    ProjectSkillBindingRequest {
        project_id,
        skill_id,
        scope,
        version: None,
        import_reference: None,
        actor_id: "user:gabriel".into(),
        capability: capability.clone(),
        policy: ProjectSkillBindingPolicy {
            allow: true,
            allowed_capabilities: CapabilitySet::new().insert(capability),
            max_bindings: 8,
        },
        approval_id: None,
        trace_id: TraceId::new(),
        expected_revision: None,
    }
}

fn mutation_request(project_id: ProjectId, skill_id: SkillId) -> ProjectSkillMutationRequest {
    let capability = capability(project_id);
    ProjectSkillMutationRequest {
        project_id,
        skill_id,
        actor_id: "user:gabriel".into(),
        capability: capability.clone(),
        policy: ProjectSkillBindingPolicy {
            allow: true,
            allowed_capabilities: CapabilitySet::new().insert(capability),
            max_bindings: 8,
        },
        approval_id: None,
        trace_id: TraceId::new(),
        expected_revision: None,
    }
}

fn project_manifest(name: &str, version: &str) -> SkillManifest {
    let mut manifest = SkillManifest::new(name, version, SkillScope::Project);
    manifest.policy.requires_approval = false;
    manifest
}

fn global_manifest(name: &str, version: &str) -> SkillManifest {
    let mut manifest = SkillManifest::new(name, version, SkillScope::Global);
    manifest.policy.requires_approval = false;
    manifest
}

fn active_skill(
    manifest: SkillManifest,
    project_id: Option<ProjectId>,
) -> (Skill, agent_core::ParsedSkill) {
    let document = format!(
        "---\n{}\n---\n# Instructions\nUse this skill as untrusted data.",
        serde_json::to_string(&manifest).unwrap()
    );
    let parsed = SkillParser::default()
        .parse(SkillParseRequest {
            document,
            files: Vec::new(),
            project_id,
        })
        .unwrap();
    let mut skill = Skill::new(parsed.manifest.clone(), project_id);
    skill.transition(SkillStatus::Testing).unwrap();
    skill.activate(parsed.manifest.version.clone()).unwrap();
    (skill, parsed)
}

#[tokio::test]
async fn binding_is_idempotent_and_only_bound_skill_can_be_loaded() {
    let (skills, bindings, project, _) = repositories().await;
    let (skill, parsed) = active_skill(project_manifest("reviewer", "1.0.0"), Some(project));
    skills.create(&skill, &parsed).await.unwrap();
    let service = ProjectSkillService::new(skills.clone(), bindings);
    let input = request(project, skill.manifest.id, SkillScope::Project);

    let created = service.bind(input.clone()).await.unwrap();
    assert!(created.changed);
    assert!(created.binding.enabled);
    assert_eq!(created.binding.revision, 1);

    let duplicate = service.bind(input).await.unwrap();
    assert!(!duplicate.changed);
    assert_eq!(duplicate.binding.revision, 1);

    let loaded = service
        .load_bound(project, AgentId::new(), skill.manifest.id)
        .await
        .unwrap();
    assert_eq!(loaded.skill.manifest.version, "1.0.0");
}

#[tokio::test]
async fn wrong_project_and_unbound_skill_fail_closed() {
    let (skills, bindings, project, other_project) = repositories().await;
    let (skill, parsed) = active_skill(project_manifest("reviewer", "1.0.0"), Some(project));
    skills.create(&skill, &parsed).await.unwrap();
    let service = ProjectSkillService::new(skills, bindings);

    assert!(service
        .load_bound(project, AgentId::new(), skill.manifest.id)
        .await
        .is_err());
    let error = service
        .bind(request(
            other_project,
            skill.manifest.id,
            SkillScope::Project,
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, agent_core::DomainError::NotFound(_)));
}

#[tokio::test]
async fn disable_is_idempotent_and_rollback_removes_active_reference() {
    let (skills, bindings, project, _) = repositories().await;
    let (first, first_parsed) = active_skill(project_manifest("reviewer", "1.0.0"), Some(project));
    skills.create(&first, &first_parsed).await.unwrap();
    let service = ProjectSkillService::new(skills, bindings);
    service
        .bind(request(project, first.manifest.id, SkillScope::Project))
        .await
        .unwrap();

    let disabled = service
        .disable(mutation_request(project, first.manifest.id))
        .await
        .unwrap();
    assert!(disabled.changed);
    assert!(!disabled.binding.enabled);

    let duplicate = service
        .disable(mutation_request(project, first.manifest.id))
        .await
        .unwrap();
    assert!(!duplicate.changed);

    let rollback = service
        .rollback(mutation_request(project, first.manifest.id))
        .await
        .unwrap();
    assert!(!rollback.binding.enabled);
    assert!(service
        .load_bound(project, AgentId::new(), first.manifest.id)
        .await
        .is_err());
}

#[tokio::test]
async fn global_binding_requires_explicit_import_and_is_project_isolated() {
    let (skills, bindings, project, other_project) = repositories().await;
    let (skill, parsed) = active_skill(global_manifest("shared-reviewer", "1.0.0"), None);
    skills.create(&skill, &parsed).await.unwrap();
    let service = ProjectSkillService::new(skills, bindings);

    let mut implicit = request(project, skill.manifest.id, SkillScope::Global);
    assert!(service.bind(implicit.clone()).await.is_err());

    implicit.import_reference = Some("project-import:shared-reviewer".into());
    implicit.version = Some("1.0.0".into());
    implicit.approval_id = Some("approval-global-1".into());
    service.bind(implicit).await.unwrap();
    assert!(service
        .load_bound(project, AgentId::new(), skill.manifest.id)
        .await
        .is_ok());
    assert!(service
        .load_bound(other_project, AgentId::new(), skill.manifest.id)
        .await
        .is_err());
}

#[tokio::test]
async fn global_import_requires_explicit_approval_and_exact_version_pin() {
    let (skills, bindings, project, _) = repositories().await;
    let (skill, parsed) = active_skill(global_manifest("pinned-reviewer", "1.0.0"), None);
    skills.create(&skill, &parsed).await.unwrap();
    let service = ProjectSkillService::new(skills, bindings);

    let mut missing_approval = request(project, skill.manifest.id, SkillScope::Global);
    missing_approval.version = Some("1.0.0".into());
    missing_approval.import_reference = Some("project-import:pinned-reviewer".into());
    assert!(matches!(
        service.bind(missing_approval).await,
        Err(agent_core::DomainError::PermissionDenied { .. })
    ));

    let mut missing_pin = request(project, skill.manifest.id, SkillScope::Global);
    missing_pin.import_reference = Some("project-import:pinned-reviewer".into());
    missing_pin.approval_id = Some("approval-global-1".into());
    assert!(matches!(
        service.bind(missing_pin).await,
        Err(agent_core::DomainError::Validation(_))
    ));

    let mut wrong_pin = request(project, skill.manifest.id, SkillScope::Global);
    wrong_pin.version = Some("2.0.0".into());
    wrong_pin.import_reference = Some("project-import:pinned-reviewer".into());
    wrong_pin.approval_id = Some("approval-global-1".into());
    assert!(matches!(
        service.bind(wrong_pin).await,
        Err(agent_core::DomainError::NotFound(_))
    ));
}

#[tokio::test]
async fn global_import_rejects_remote_registry_sources() {
    let (skills, bindings, project, _) = repositories().await;
    let mut manifest = global_manifest("remote-reviewer", "1.0.0");
    manifest.source.kind = SkillSourceKind::Git;
    manifest.source.reference = "https://example.com/skills.git".into();
    let (skill, parsed) = active_skill(manifest, None);
    skills.create(&skill, &parsed).await.unwrap();
    let service = ProjectSkillService::new(skills, bindings);
    let mut input = request(project, skill.manifest.id, SkillScope::Global);
    input.version = Some("1.0.0".into());
    input.import_reference = Some("project-import:remote-reviewer".into());
    input.approval_id = Some("approval-global-remote".into());

    assert!(matches!(
        service.bind(input).await,
        Err(agent_core::DomainError::PermissionDenied { .. })
    ));
}

#[tokio::test]
async fn existing_binding_cannot_change_project_scope_without_unbind() {
    let (skills, bindings, project, _) = repositories().await;
    let (project_skill, project_parsed) =
        active_skill(project_manifest("scoped-reviewer", "1.0.0"), Some(project));
    skills
        .create(&project_skill, &project_parsed)
        .await
        .unwrap();
    let mut global_manifest = global_manifest("scoped-reviewer", "1.0.0");
    global_manifest.id = project_skill.manifest.id;
    let (global_skill, global_parsed) = active_skill(global_manifest, None);
    skills.create(&global_skill, &global_parsed).await.unwrap();

    let service = ProjectSkillService::new(skills, bindings);
    service
        .bind(request(
            project,
            project_skill.manifest.id,
            SkillScope::Project,
        ))
        .await
        .unwrap();
    let mut global_request = request(project, global_skill.manifest.id, SkillScope::Global);
    global_request.import_reference = Some("project-import:scoped-reviewer".into());
    global_request.version = Some("1.0.0".into());
    global_request.approval_id = Some("approval-global-scope".into());
    assert!(matches!(
        service.bind(global_request).await,
        Err(agent_core::DomainError::InvariantViolation(_))
    ));
}

#[tokio::test]
async fn binding_mutation_emits_auditable_project_event() {
    let (skills, bindings, project, _) = repositories().await;
    let (skill, parsed) = active_skill(project_manifest("audited", "1.0.0"), Some(project));
    skills.create(&skill, &parsed).await.unwrap();
    let bus = EventBus::<ApplicationEvent>::bounded(4);
    let mut receiver = bus.subscribe();
    let service = ProjectSkillService::new(skills, bindings).with_event_bus(bus);

    let result = service
        .bind(request(project, skill.manifest.id, SkillScope::Project))
        .await
        .unwrap();
    assert!(result.event_id.is_some());
    let event = receiver.recv().await.unwrap();
    assert_eq!(event.event_type, EventKind::SkillBindingChanged);
    assert_eq!(event.project_id, project);
    assert!(event.payload.contains("\"action\":\"bind\""));
    assert!(!event.payload.contains("Instructions"));
}

#[tokio::test]
async fn binding_policy_capability_and_actor_validation_fail_closed() {
    let (skills, bindings, project, _) = repositories().await;
    let (skill, parsed) = active_skill(project_manifest("reviewer", "1.0.0"), Some(project));
    skills.create(&skill, &parsed).await.unwrap();
    let service = ProjectSkillService::new(skills, bindings);

    let mut denied = request(project, skill.manifest.id, SkillScope::Project);
    denied.policy.allow = false;
    assert!(matches!(
        service.bind(denied).await,
        Err(agent_core::DomainError::PermissionDenied { .. })
    ));

    let mut wrong_capability = request(project, skill.manifest.id, SkillScope::Project);
    wrong_capability.capability = Capability::new(Resource::Memory, Action::Read);
    assert!(matches!(
        service.bind(wrong_capability).await,
        Err(agent_core::DomainError::CapabilityUnavailable(_))
    ));

    let mut invalid_actor = request(project, skill.manifest.id, SkillScope::Project);
    invalid_actor.actor_id = "actor\nforged".into();
    assert!(matches!(
        service.bind(invalid_actor).await,
        Err(agent_core::DomainError::Validation(_))
    ));
}
