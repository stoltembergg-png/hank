use agent_core::{
    Action, AgentId, BudgetLimits, Capability, CapabilitySet, ProjectId, Resource, Skill,
    SkillDependency, SkillFile, SkillFileInput, SkillFileRole, SkillId, SkillManifest,
    SkillParseRequest, SkillParser, SkillScope, SkillStatus, TraceId,
};
use agent_runtime::skill_testing::{DeterministicSkillTestRunner, SkillFixture, SkillTestStep};
use agent_runtime::skill_validation::{
    SkillDependencyNode, SkillValidationPolicy, SkillValidationReport, SkillValidationRequest,
    SkillValidationService, SkillValidationStatus,
};
use agent_runtime::{
    migrations::run_migrations, sqlite::SqliteStorage, SkillGlobalImport, SkillLoadBudget,
    SkillLoadError, SkillLoadPolicy, SkillLoadRequest, SkillLoader, SqliteSkillRepository,
};

async fn repository() -> (SqliteSkillRepository, ProjectId, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let first = ProjectId::new();
    let second = ProjectId::new();
    for project in [first, second] {
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Loader Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project.to_string())
            .execute(storage.pool())
            .await
            .unwrap();
    }
    (
        SqliteSkillRepository::new(storage.pool().clone()),
        first,
        second,
    )
}

fn capability(project_id: ProjectId) -> Capability {
    Capability::new(Resource::Skill, Action::Read).with_scope(project_id.to_string())
}

fn request(project_id: ProjectId, skill_id: SkillId, scope: SkillScope) -> SkillLoadRequest {
    SkillLoadRequest {
        project_id,
        agent_id: AgentId::new(),
        skill_id,
        version: None,
        scope,
        global_import: None,
        capability: capability(project_id),
        policy: SkillLoadPolicy {
            allow: true,
            allow_testing: false,
            allow_external_references: false,
            allowed_capabilities: CapabilitySet::new().insert(capability(project_id)),
        },
        budget: SkillLoadBudget::default(),
        trace_id: TraceId::new(),
        requested_paths: Vec::new(),
    }
}

fn project_manifest(name: &str, version: &str) -> SkillManifest {
    let mut manifest = SkillManifest::new(name, version, SkillScope::Project);
    manifest.files.push(SkillFile {
        path: "tests/basic.json".into(),
        role: SkillFileRole::Test,
        digest: "b".repeat(64),
    });
    manifest.tests.push("tests/basic.json".into());
    manifest.policy.requires_approval = false;
    manifest
}

fn global_manifest(name: &str, version: &str) -> SkillManifest {
    let mut manifest = SkillManifest::new(name, version, SkillScope::Global);
    manifest.policy.requires_approval = false;
    manifest
}

fn validation_report(
    parsed: &agent_core::ParsedSkill,
    project_id: ProjectId,
) -> SkillValidationReport {
    let fixture = SkillFixture::new(
        project_id,
        parsed.manifest.id,
        parsed.manifest.version.clone(),
        parsed.manifest.trace.trace_id,
        vec![SkillTestStep::AssertLabel {
            label: "loader".into(),
        }],
        4,
    )
    .unwrap();
    let test_report = DeterministicSkillTestRunner::run(&fixture).unwrap();
    let capability = capability(project_id);
    let request = SkillValidationRequest {
        project_id,
        skill_id: parsed.manifest.id,
        version: parsed.manifest.version.clone(),
        actor_id: "test-operator".into(),
        capability: capability.clone(),
        policy: SkillValidationPolicy {
            allowed_capabilities: CapabilitySet::new().insert(capability),
        },
        budget: BudgetLimits::default(),
        trace_id: parsed.manifest.trace.trace_id,
        dependency_graph: vec![SkillDependencyNode {
            skill_id: parsed.manifest.id,
            dependencies: Vec::new(),
        }],
    };
    let validation = SkillValidationService::validate(parsed, &request, Some(&test_report));
    assert_eq!(validation.status, SkillValidationStatus::Passed);
    validation
}

fn parsed_skill(
    manifest: SkillManifest,
    project_id: Option<ProjectId>,
    mut files: Vec<SkillFileInput>,
    links: &str,
) -> (Skill, agent_core::ParsedSkill) {
    let document = format!(
        "---\n{}\n---\n# Instructions\n{}",
        serde_json::to_string(&manifest).unwrap(),
        links
    );
    if manifest
        .tests
        .iter()
        .any(|path| !files.iter().any(|file| &file.path == path))
    {
        files.push(SkillFileInput {
            path: "tests/basic.json".into(),
            content: "{\"case\":\"safe\"}".into(),
        });
    }
    let parsed = SkillParser::default()
        .parse(SkillParseRequest {
            document,
            files,
            project_id,
        })
        .unwrap();
    let mut skill = Skill::new(parsed.manifest.clone(), project_id);
    skill.transition(SkillStatus::Testing).unwrap();
    skill.activate(parsed.manifest.version.clone()).unwrap();
    (skill, parsed)
}

#[tokio::test]
async fn active_project_skill_loads_bounded_data_without_executing_scripts() {
    let (repo, project, _) = repository().await;
    let mut manifest = project_manifest("loader", "1.0.0");
    manifest.files.extend([
        SkillFile {
            path: "scripts/check.sh".into(),
            role: SkillFileRole::Script,
            digest: "a".repeat(64),
        },
        SkillFile {
            path: "references/guide.md".into(),
            role: SkillFileRole::Reference,
            digest: "b".repeat(64),
        },
    ]);
    let (skill, parsed) = parsed_skill(
        manifest,
        Some(project),
        vec![
            SkillFileInput {
                path: "scripts/check.sh".into(),
                content: "echo MUST_NOT_EXECUTE".into(),
            },
            SkillFileInput {
                path: "references/guide.md".into(),
                content: "Reference data".into(),
            },
        ],
        "See [guide](references/guide.md).",
    );
    repo.create(&skill, &parsed).await.unwrap();

    let loaded = SkillLoader::new(repo.clone())
        .load(request(project, skill.manifest.id, SkillScope::Project))
        .await
        .unwrap();

    assert_eq!(loaded.skill.manifest.version, "1.0.0");
    assert_eq!(loaded.instructions.len(), 1);
    assert_eq!(loaded.artifacts.len(), 3);
    assert!(loaded
        .artifacts
        .iter()
        .any(|artifact| artifact.role == SkillFileRole::Script
            && artifact.content == "echo MUST_NOT_EXECUTE"));
    assert_eq!(loaded.dependencies.len(), 0);
}

#[tokio::test]
async fn lifecycle_policy_capability_and_project_scope_fail_closed() {
    let (repo, project, other_project) = repository().await;
    let manifest = project_manifest("gated", "1.0.0");
    let (mut skill, parsed) = parsed_skill(manifest, Some(project), Vec::new(), "draft");
    skill.status = SkillStatus::Draft;
    skill.pinned_version = None;
    repo.create(&skill, &parsed).await.unwrap();

    let loader = SkillLoader::new(repo.clone());
    let mut draft_request = request(project, skill.manifest.id, SkillScope::Project);
    assert!(matches!(
        loader.load(draft_request.clone()).await,
        Err(SkillLoadError::LifecycleDenied)
    ));

    draft_request.policy.allow = false;
    assert!(matches!(
        loader.load(draft_request.clone()).await,
        Err(SkillLoadError::PolicyDenied)
    ));

    draft_request.policy.allow = true;
    draft_request.capability = Capability::new(Resource::Memory, Action::Read);
    assert!(matches!(
        loader.load(draft_request).await,
        Err(SkillLoadError::CapabilityDenied)
    ));

    let wrong_project = request(other_project, skill.manifest.id, SkillScope::Project);
    assert!(matches!(
        loader.load(wrong_project).await,
        Err(SkillLoadError::NotFound)
    ));
}

#[tokio::test]
async fn global_skill_requires_explicit_import_for_project_context() {
    let (repo, project, _) = repository().await;
    let (skill, parsed) = parsed_skill(
        global_manifest("global-loader", "1.0.0"),
        None,
        Vec::new(),
        "global data",
    );
    repo.create(&skill, &parsed).await.unwrap();
    let loader = SkillLoader::new(repo);

    let mut implicit = request(project, skill.manifest.id, SkillScope::Global);
    assert!(matches!(
        loader.load(implicit.clone()).await,
        Err(SkillLoadError::GlobalImportRequired)
    ));

    implicit.global_import = Some(SkillGlobalImport {
        reference: "project-import:global-loader".into(),
    });
    let loaded = loader.load(implicit).await.unwrap();
    assert_eq!(loaded.skill.project_id, None);
    assert_eq!(loaded.skill.manifest.scope, SkillScope::Global);
}

#[tokio::test]
async fn testing_and_external_links_require_explicit_policy_flags() {
    let (repo, project, _) = repository().await;
    let manifest = SkillManifest::new("testing-loader", "1.0.0", SkillScope::Project);
    let (mut skill, parsed) = parsed_skill(
        manifest,
        Some(project),
        Vec::new(),
        "See [external](https://example.com/reference).",
    );
    skill.status = SkillStatus::Testing;
    skill.pinned_version = None;
    repo.create(&skill, &parsed).await.unwrap();

    let loader = SkillLoader::new(repo);
    let mut load_request = request(project, skill.manifest.id, SkillScope::Project);
    assert!(matches!(
        loader.load(load_request.clone()).await,
        Err(SkillLoadError::LifecycleDenied)
    ));

    load_request.policy.allow_testing = true;
    assert!(matches!(
        loader.load(load_request.clone()).await,
        Err(SkillLoadError::PolicyDenied)
    ));

    load_request.policy.allow_external_references = true;
    let loaded = loader.load(load_request).await.unwrap();
    assert_eq!(loaded.instructions.len(), 1);
}

#[tokio::test]
async fn cache_key_changes_after_update_and_rollback() {
    let (repo, project, _) = repository().await;
    let first_manifest = project_manifest("cached", "1.0.0");
    let (first, first_parsed) = parsed_skill(first_manifest, Some(project), Vec::new(), "v1");
    repo.create(&first, &first_parsed).await.unwrap();
    let loader = SkillLoader::new(repo.clone());
    let first_loaded = loader
        .load(request(project, first.manifest.id, SkillScope::Project))
        .await
        .unwrap();

    let mut second_manifest = project_manifest("cached", "1.1.0");
    second_manifest.id = first.manifest.id;
    second_manifest.digest = "2".repeat(64);
    let (mut second, second_parsed) =
        parsed_skill(second_manifest, Some(project), Vec::new(), "v2");
    second.status = SkillStatus::Testing;
    second.pinned_version = None;
    let updated = repo.update(&second, &second_parsed, 1).await.unwrap();
    repo.promote(
        SkillScope::Project,
        Some(&project),
        &first.manifest.id,
        "1.1.0",
        updated.revision,
        &validation_report(&updated.parsed, project),
    )
    .await
    .unwrap();
    let second_loaded = loader
        .load(request(project, first.manifest.id, SkillScope::Project))
        .await
        .unwrap();
    assert_eq!(first_loaded.skill.manifest.version, "1.0.0");
    assert_eq!(second_loaded.skill.manifest.version, "1.1.0");
    assert_ne!(first_loaded.cache_key, second_loaded.cache_key);

    repo.rollback(
        SkillScope::Project,
        Some(&project),
        &first.manifest.id,
        "1.0.0",
        3,
        &validation_report(
            &repo
                .get_version(
                    SkillScope::Project,
                    Some(&project),
                    &first.manifest.id,
                    "1.0.0",
                )
                .await
                .unwrap()
                .unwrap()
                .parsed,
            project,
        ),
    )
    .await
    .unwrap();
    let rolled_back = loader
        .load(request(project, first.manifest.id, SkillScope::Project))
        .await
        .unwrap();
    assert_eq!(rolled_back.skill.manifest.version, "1.0.0");
    assert_ne!(second_loaded.cache_key, rolled_back.cache_key);

    let mut pinned_request = request(project, first.manifest.id, SkillScope::Project);
    pinned_request.version = Some("1.1.0".into());
    let pinned = loader.load(pinned_request).await.unwrap();
    assert_eq!(pinned.skill.manifest.version, "1.1.0");
}

#[tokio::test]
async fn dependency_cycles_and_depth_limits_are_rejected() {
    let (repo, project, _) = repository().await;
    let mut first_manifest = project_manifest("cycle-a", "1.0.0");
    let mut second_manifest = project_manifest("cycle-b", "1.0.0");
    first_manifest.dependencies.push(SkillDependency {
        skill_id: second_manifest.id,
        version_req: "*".into(),
        optional: false,
    });
    second_manifest.dependencies.push(SkillDependency {
        skill_id: first_manifest.id,
        version_req: "*".into(),
        optional: false,
    });
    let (first, first_parsed) = parsed_skill(first_manifest, Some(project), Vec::new(), "a");
    let (second, second_parsed) = parsed_skill(second_manifest, Some(project), Vec::new(), "b");
    repo.create(&first, &first_parsed).await.unwrap();
    repo.create(&second, &second_parsed).await.unwrap();

    let loader = SkillLoader::new(repo);
    assert!(matches!(
        loader
            .load(request(project, first.manifest.id, SkillScope::Project))
            .await,
        Err(SkillLoadError::DependencyCycle)
    ));

    let mut limited = request(project, first.manifest.id, SkillScope::Project);
    limited.budget.max_dependency_depth = 0;
    assert!(matches!(
        loader.load(limited).await,
        Err(SkillLoadError::InvalidRequest)
    ));
}

#[tokio::test]
async fn path_and_budget_limits_are_enforced_before_returning_content() {
    let (repo, project, _) = repository().await;
    let mut manifest = project_manifest("bounded-loader", "1.0.0");
    manifest.files.push(SkillFile {
        path: "references/large.md".into(),
        role: SkillFileRole::Reference,
        digest: "c".repeat(64),
    });
    let (skill, parsed) = parsed_skill(
        manifest,
        Some(project),
        vec![SkillFileInput {
            path: "references/large.md".into(),
            content: "bounded reference content".into(),
        }],
        "bounded",
    );
    repo.create(&skill, &parsed).await.unwrap();
    let loader = SkillLoader::new(repo);

    let mut path_request = request(project, skill.manifest.id, SkillScope::Project);
    path_request.requested_paths = vec!["../escape.md".into()];
    assert!(matches!(
        loader.load(path_request).await,
        Err(SkillLoadError::InvalidReference)
    ));

    let mut budget_request = request(project, skill.manifest.id, SkillScope::Project);
    budget_request.budget.max_bytes = 1;
    assert!(matches!(
        loader.load(budget_request).await,
        Err(SkillLoadError::BudgetExceeded)
    ));
}
