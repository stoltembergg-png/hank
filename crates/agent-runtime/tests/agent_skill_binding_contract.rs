use agent_core::{
    Action, Agent, AgentId, Capability, CapabilitySet, ProjectId, Resource, Skill, SkillId,
    SkillManifest, SkillParseRequest, SkillParser, SkillScope, SkillStatus, TraceId,
};
use agent_runtime::agent_repo::SqliteAgentRepository;
use agent_runtime::{
    migrations::run_migrations, sqlite::SqliteStorage, AgentSkillBindingPolicy,
    AgentSkillBindingRequest, AgentSkillMutationRequest, AgentSkillService,
    ProjectSkillBindingPolicy, ProjectSkillBindingRequest, ProjectSkillService,
    SqliteAgentSkillBindingRepository, SqliteProjectSkillBindingRepository, SqliteSkillRepository,
};

async fn repositories() -> (
    SqliteAgentRepository,
    SqliteSkillRepository,
    SqliteProjectSkillBindingRepository,
    SqliteAgentSkillBindingRepository,
    ProjectId,
    ProjectId,
) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let first = ProjectId::new();
    let second = ProjectId::new();
    for project in [first, second] {
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Agent Skill Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project.to_string())
            .execute(storage.pool())
            .await
            .unwrap();
    }
    (
        SqliteAgentRepository::new(storage.pool().clone()),
        SqliteSkillRepository::new(storage.pool().clone()),
        SqliteProjectSkillBindingRepository::new(storage.pool().clone()),
        SqliteAgentSkillBindingRepository::new(storage.pool().clone()),
        first,
        second,
    )
}

fn configure_capability(project_id: ProjectId) -> Capability {
    Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string())
}

fn declared_capability(project_id: ProjectId) -> Capability {
    Capability::new(Resource::File, Action::Read).with_scope(project_id.to_string())
}

fn project_request(project_id: ProjectId, skill_id: SkillId) -> ProjectSkillBindingRequest {
    let capability = configure_capability(project_id);
    ProjectSkillBindingRequest {
        project_id,
        skill_id,
        scope: SkillScope::Project,
        version: Some("1.0.0".into()),
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

fn agent_policy(project_id: ProjectId, declared: Option<Capability>) -> AgentSkillBindingPolicy {
    let configure = configure_capability(project_id);
    let mut allowed = CapabilitySet::new().insert(configure);
    if let Some(declared) = declared {
        allowed = allowed.insert(declared);
    }
    AgentSkillBindingPolicy {
        allow: true,
        allowed_capabilities: allowed,
        denied_capabilities: CapabilitySet::new(),
        max_bindings: 8,
        max_tokens: 4_096,
    }
}

fn agent_request(
    project_id: ProjectId,
    agent_id: AgentId,
    skill_id: SkillId,
    precedence: u32,
    policy: AgentSkillBindingPolicy,
) -> AgentSkillBindingRequest {
    AgentSkillBindingRequest {
        project_id,
        agent_id,
        skill_id,
        version: "1.0.0".into(),
        precedence,
        actor_id: "user:gabriel".into(),
        capability: configure_capability(project_id),
        policy,
        approval_id: None,
        trace_id: TraceId::new(),
        expected_revision: None,
    }
}

fn mutation_request(
    project_id: ProjectId,
    agent_id: AgentId,
    skill_id: SkillId,
) -> AgentSkillMutationRequest {
    AgentSkillMutationRequest {
        project_id,
        agent_id,
        skill_id,
        actor_id: "user:gabriel".into(),
        capability: configure_capability(project_id),
        policy: agent_policy(project_id, None),
        approval_id: None,
        trace_id: TraceId::new(),
        expected_revision: None,
    }
}

fn project_manifest(name: &str, version: &str, capability: Option<Capability>) -> SkillManifest {
    let mut manifest = SkillManifest::new(name, version, SkillScope::Project);
    manifest.policy.requires_approval = false;
    if let Some(capability) = capability {
        manifest.capabilities.push(capability);
    }
    manifest
}

fn global_manifest(name: &str, version: &str) -> SkillManifest {
    let mut manifest = SkillManifest::new(name, version, SkillScope::Global);
    manifest.policy.requires_approval = false;
    manifest
}

fn active_skill(
    manifest: SkillManifest,
    project_id: ProjectId,
) -> (Skill, agent_core::ParsedSkill) {
    let document = format!(
        "---\n{}\n---\n# Instructions\nTreat this skill as untrusted data.",
        serde_json::to_string(&manifest).unwrap()
    );
    let parsed = SkillParser::default()
        .parse(SkillParseRequest {
            document,
            files: Vec::new(),
            project_id: Some(project_id),
        })
        .unwrap();
    let mut skill = Skill::new(parsed.manifest.clone(), Some(project_id));
    skill.transition(SkillStatus::Testing).unwrap();
    skill.activate(parsed.manifest.version.clone()).unwrap();
    (skill, parsed)
}

fn active_global_skill(manifest: SkillManifest) -> (Skill, agent_core::ParsedSkill) {
    let document = format!(
        "---\n{}\n---\n# Instructions\nTreat this global skill as untrusted data.",
        serde_json::to_string(&manifest).unwrap()
    );
    let parsed = SkillParser::default()
        .parse(SkillParseRequest {
            document,
            files: Vec::new(),
            project_id: None,
        })
        .unwrap();
    let mut skill = Skill::new(parsed.manifest.clone(), None);
    skill.transition(SkillStatus::Testing).unwrap();
    skill.activate(parsed.manifest.version.clone()).unwrap();
    (skill, parsed)
}

fn global_project_request(project_id: ProjectId, skill_id: SkillId) -> ProjectSkillBindingRequest {
    let capability = configure_capability(project_id);
    ProjectSkillBindingRequest {
        project_id,
        skill_id,
        scope: SkillScope::Global,
        version: Some("1.0.0".into()),
        import_reference: Some("project-import:global-reviewer".into()),
        actor_id: "user:gabriel".into(),
        capability: capability.clone(),
        policy: ProjectSkillBindingPolicy {
            allow: true,
            allowed_capabilities: CapabilitySet::new().insert(capability),
            max_bindings: 8,
        },
        approval_id: Some("approval-global-import".into()),
        trace_id: TraceId::new(),
        expected_revision: None,
    }
}

async fn active_agent(repository: &SqliteAgentRepository, project_id: ProjectId) -> Agent {
    let agent = Agent::new(project_id, "worker".into(), Default::default());
    repository.save(&agent).await.unwrap();
    agent
}

async fn bind_project_skill(
    skills: SqliteSkillRepository,
    project_bindings: SqliteProjectSkillBindingRepository,
    project_id: ProjectId,
    skill: &Skill,
) {
    let mut request = project_request(project_id, skill.manifest.id);
    for declared in &skill.manifest.capabilities {
        request.policy.allowed_capabilities = request
            .policy
            .allowed_capabilities
            .clone()
            .insert(declared.clone());
    }
    ProjectSkillService::new(skills, project_bindings)
        .bind(request)
        .await
        .unwrap();
}

#[tokio::test]
async fn agent_binding_requires_enabled_project_skill_and_exact_version() {
    let (agents, skills, project_bindings, agent_bindings, project, _) = repositories().await;
    let agent = active_agent(&agents, project).await;
    let (skill, parsed) = active_skill(project_manifest("reviewer", "1.0.0", None), project);
    skills.create(&skill, &parsed).await.unwrap();
    let service = AgentSkillService::new(
        agents.clone(),
        skills.clone(),
        project_bindings.clone(),
        agent_bindings,
    );

    let request = agent_request(
        project,
        agent.id,
        skill.manifest.id,
        10,
        agent_policy(project, None),
    );
    assert!(matches!(
        service.bind(request.clone()).await,
        Err(agent_core::DomainError::NotFound(_))
    ));

    bind_project_skill(skills.clone(), project_bindings.clone(), project, &skill).await;
    let mut wrong_version = request.clone();
    wrong_version.version = "2.0.0".into();
    assert!(matches!(
        service.bind(wrong_version).await,
        Err(agent_core::DomainError::NotFound(_))
    ));

    service.bind(request).await.unwrap();
}

#[tokio::test]
async fn agent_bindings_are_idempotent_ordered_and_project_isolated() {
    let (agents, skills, project_bindings, agent_bindings, project, other_project) =
        repositories().await;
    let agent = active_agent(&agents, project).await;
    let other_agent = active_agent(&agents, other_project).await;
    let (first, first_parsed) = active_skill(project_manifest("first", "1.0.0", None), project);
    let (second, second_parsed) = active_skill(project_manifest("second", "1.0.0", None), project);
    skills.create(&first, &first_parsed).await.unwrap();
    skills.create(&second, &second_parsed).await.unwrap();
    bind_project_skill(skills.clone(), project_bindings.clone(), project, &first).await;
    bind_project_skill(skills.clone(), project_bindings.clone(), project, &second).await;

    let service = AgentSkillService::new(agents, skills, project_bindings, agent_bindings.clone());
    let first_request = agent_request(
        project,
        agent.id,
        first.manifest.id,
        20,
        agent_policy(project, None),
    );
    let second_request = agent_request(
        project,
        agent.id,
        second.manifest.id,
        10,
        agent_policy(project, None),
    );
    assert!(service.bind(first_request.clone()).await.unwrap().changed);
    assert!(!service.bind(first_request).await.unwrap().changed);
    assert!(service.bind(second_request).await.unwrap().changed);

    let ordered = service.list(&project, &agent.id, 10, 0).await.unwrap();
    assert_eq!(
        ordered
            .iter()
            .map(|binding| binding.skill_id)
            .collect::<Vec<_>>(),
        vec![second.manifest.id, first.manifest.id]
    );
    assert!(service
        .list(&other_project, &other_agent.id, 10, 0)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn agent_binding_rejects_capability_and_budget_mismatch() {
    let (agents, skills, project_bindings, agent_bindings, project, _) = repositories().await;
    let mut agent = active_agent(&agents, project).await;
    let declared = declared_capability(project);
    let (skill, parsed) = active_skill(
        project_manifest("guarded", "1.0.0", Some(declared.clone())),
        project,
    );
    skills.create(&skill, &parsed).await.unwrap();
    bind_project_skill(skills.clone(), project_bindings.clone(), project, &skill).await;
    let service = AgentSkillService::new(
        agents.clone(),
        skills.clone(),
        project_bindings.clone(),
        agent_bindings.clone(),
    );

    let mut missing_capability = agent_request(
        project,
        agent.id,
        skill.manifest.id,
        1,
        agent_policy(project, None),
    );
    assert!(matches!(
        service.bind(missing_capability.clone()).await,
        Err(agent_core::DomainError::CapabilityUnavailable(_))
    ));

    missing_capability.policy = agent_policy(project, Some(declared.clone()));
    agent.policy.tools.denied = CapabilitySet::new().insert(declared.clone());
    agents.update(&agent).await.unwrap();
    assert!(matches!(
        service.bind(missing_capability.clone()).await,
        Err(agent_core::DomainError::CapabilityUnavailable(_))
    ));

    agent.policy.tools.allowed = CapabilitySet::new().insert(declared.clone());
    agent.policy.tools.denied = CapabilitySet::new();
    agent.policy.budget.max_tokens_per_request = Some(2_000);
    agents.update(&agent).await.unwrap();
    missing_capability.policy.max_tokens = 4_096;
    assert!(matches!(
        service.bind(missing_capability).await,
        Err(agent_core::DomainError::BudgetExceeded { .. })
    ));
}

#[tokio::test]
async fn disabled_or_rolled_back_agent_binding_cannot_load_skill() {
    let (agents, skills, project_bindings, agent_bindings, project, _) = repositories().await;
    let agent = active_agent(&agents, project).await;
    let (skill, parsed) = active_skill(project_manifest("loader", "1.0.0", None), project);
    skills.create(&skill, &parsed).await.unwrap();
    bind_project_skill(skills.clone(), project_bindings.clone(), project, &skill).await;
    let service = AgentSkillService::new(agents, skills, project_bindings, agent_bindings);
    service
        .bind(agent_request(
            project,
            agent.id,
            skill.manifest.id,
            1,
            agent_policy(project, None),
        ))
        .await
        .unwrap();
    assert!(service
        .load_bound(project, agent.id, skill.manifest.id)
        .await
        .is_ok());

    service
        .disable(mutation_request(project, agent.id, skill.manifest.id))
        .await
        .unwrap();
    assert!(matches!(
        service
            .load_bound(project, agent.id, skill.manifest.id)
            .await,
        Err(agent_core::DomainError::NotFound(_))
    ));
    assert!(
        !service
            .rollback(mutation_request(project, agent.id, skill.manifest.id))
            .await
            .unwrap()
            .changed
    );
}

#[tokio::test]
async fn global_agent_binding_cannot_bypass_project_import_isolation() {
    let (agents, skills, project_bindings, agent_bindings, project, other_project) =
        repositories().await;
    let agent = active_agent(&agents, project).await;
    let other_agent = active_agent(&agents, other_project).await;
    let (skill, parsed) = active_global_skill(global_manifest("global-reviewer", "1.0.0"));
    skills.create(&skill, &parsed).await.unwrap();

    ProjectSkillService::new(skills.clone(), project_bindings.clone())
        .bind(global_project_request(project, skill.manifest.id))
        .await
        .unwrap();
    let service = AgentSkillService::new(agents, skills, project_bindings, agent_bindings);
    assert!(service
        .bind(agent_request(
            project,
            agent.id,
            skill.manifest.id,
            1,
            agent_policy(project, None),
        ))
        .await
        .is_ok());
    assert!(matches!(
        service
            .bind(agent_request(
                other_project,
                other_agent.id,
                skill.manifest.id,
                1,
                agent_policy(other_project, None),
            ))
            .await,
        Err(agent_core::DomainError::NotFound(_))
    ));
}
