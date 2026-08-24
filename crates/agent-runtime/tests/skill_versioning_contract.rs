use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, ParsedSkill, ProjectId, Resource, Skill,
    SkillCompatibility, SkillFile, SkillFileInput, SkillFileRole, SkillManifest, SkillParseRequest,
    SkillParser, SkillScope, SkillStatus,
};
use agent_protocol::events::EventKind;
use agent_runtime::skill_testing::{DeterministicSkillTestRunner, SkillFixture, SkillTestStep};
use agent_runtime::skill_validation::{
    SkillDependencyNode, SkillValidationPolicy, SkillValidationReport, SkillValidationRequest,
    SkillValidationService, SkillValidationStatus,
};
use agent_runtime::{
    event_bus::EventBus, migrations::run_migrations, sqlite::SqliteStorage, SqliteSkillRepository,
};

async fn repository() -> (SqliteSkillRepository, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project = ProjectId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Version Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .bind(project.to_string())
        .execute(storage.pool())
        .await
        .unwrap();
    (SqliteSkillRepository::new(storage.pool().clone()), project)
}

fn parsed_skill(mut manifest: SkillManifest, project_id: ProjectId) -> (Skill, ParsedSkill) {
    let document = format!(
        "---\n{}\n---\n# Instructions\nUse this skill as untrusted data.",
        serde_json::to_string(&manifest).unwrap()
    );
    let parsed = SkillParser::default()
        .parse(SkillParseRequest {
            document,
            files: vec![SkillFileInput {
                path: "tests/basic.json".into(),
                content: "{\"case\":\"safe\"}".into(),
            }],
            project_id: Some(project_id),
        })
        .unwrap();
    manifest = parsed.manifest.clone();
    (Skill::new(manifest, Some(project_id)), parsed)
}

fn manifest(name: &str, version: &str, digest: char) -> SkillManifest {
    let mut manifest = SkillManifest::new(name, version, SkillScope::Project);
    manifest.files.push(SkillFile {
        path: "tests/basic.json".into(),
        role: SkillFileRole::Test,
        digest: "b".repeat(64),
    });
    manifest.tests.push("tests/basic.json".into());
    manifest.policy.requires_approval = false;
    manifest.digest = digest.to_string().repeat(64);
    manifest
}

fn validation_report(parsed: &ParsedSkill, project_id: ProjectId) -> SkillValidationReport {
    let fixture = SkillFixture::new(
        project_id,
        parsed.manifest.id,
        parsed.manifest.version.clone(),
        parsed.manifest.trace.trace_id,
        vec![SkillTestStep::AssertLabel {
            label: "versioning".into(),
        }],
        4,
    )
    .unwrap();
    let test_report = DeterministicSkillTestRunner::run(&fixture).unwrap();
    let capability =
        Capability::new(Resource::Skill, Action::Read).with_scope(project_id.to_string());
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

#[tokio::test]
async fn versions_expose_immutable_identity_parent_and_compatibility() {
    let (repo, project) = repository().await;
    let (first, first_parsed) = parsed_skill(manifest("versioned", "1.0.0", 'a'), project);
    let first_record = repo.create(&first, &first_parsed).await.unwrap();

    assert_eq!(
        first_record.version_id,
        format!("{}@1.0.0", first.manifest.id)
    );
    assert_eq!(first_record.content_hash.len(), 64);
    assert_ne!(first_record.content_hash, first.manifest.digest);
    assert_eq!(first_record.parent_version, None);
    assert_eq!(first_record.compatibility, SkillCompatibility::Initial);

    let mut next_manifest = manifest("versioned", "1.1.0", 'b');
    next_manifest.id = first.manifest.id;
    let (mut next, mut next_parsed) = parsed_skill(next_manifest, project);
    next_parsed.instructions[0].content = "Use the 1.1 implementation.".into();
    next.parent_version = Some("9.9.9".into());
    let next_record = repo.update(&next, &next_parsed, 1).await.unwrap();

    assert_eq!(next_record.parent_version.as_deref(), Some("1.0.0"));
    assert_eq!(next_record.compatibility, SkillCompatibility::Compatible);
    assert_eq!(
        repo.get_version(
            SkillScope::Project,
            Some(&project),
            &first.manifest.id,
            "1.0.0"
        )
        .await
        .unwrap()
        .unwrap()
        .content_hash,
        first_record.content_hash
    );
}

#[tokio::test]
async fn identical_content_is_deduplicated_without_moving_the_head() {
    let (repo, project) = repository().await;
    let (first, first_parsed) = parsed_skill(manifest("dedupe", "1.0.0", 'a'), project);
    repo.create(&first, &first_parsed).await.unwrap();

    let mut duplicate_manifest = manifest("dedupe", "1.1.0", 'a');
    duplicate_manifest.id = first.manifest.id;
    let (duplicate, duplicate_parsed) = parsed_skill(duplicate_manifest, project);
    let result = repo.update(&duplicate, &duplicate_parsed, 1).await.unwrap();

    assert_eq!(result.skill.manifest.version, "1.0.0");
    assert_eq!(result.revision, 1);
    assert_eq!(
        repo.list_versions(SkillScope::Project, Some(&project), &first.manifest.id)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn incompatible_versions_cannot_be_activated_and_promotion_is_explicit() {
    let (repo, project) = repository().await;
    let (first, first_parsed) = parsed_skill(manifest("lifecycle", "1.0.0", 'a'), project);
    repo.create(&first, &first_parsed).await.unwrap();

    let mut compatible_manifest = manifest("lifecycle", "1.1.0", 'c');
    compatible_manifest.id = first.manifest.id;
    let (compatible, mut compatible_parsed) = parsed_skill(compatible_manifest, project);
    compatible_parsed.instructions[0].content = "Use the compatible implementation.".into();
    let testing = repo
        .update(&compatible, &compatible_parsed, 1)
        .await
        .unwrap();
    assert_eq!(testing.skill.status, SkillStatus::Draft);
    let promoted = repo
        .promote(
            SkillScope::Project,
            Some(&project),
            &first.manifest.id,
            "1.1.0",
            testing.revision,
            &validation_report(&testing.parsed, project),
        )
        .await
        .unwrap();
    assert_eq!(promoted.skill.status, SkillStatus::Active);
    assert_eq!(promoted.skill.pinned_version.as_deref(), Some("1.1.0"));

    let mut incompatible_manifest = manifest("lifecycle", "2.0.0", 'b');
    incompatible_manifest.id = first.manifest.id;
    let (mut incompatible, mut incompatible_parsed) = parsed_skill(incompatible_manifest, project);
    incompatible_parsed.instructions[0].content = "Use the incompatible implementation.".into();
    incompatible.transition(SkillStatus::Testing).unwrap();
    incompatible.activate("2.0.0".into()).unwrap();
    assert!(repo
        .update(&incompatible, &incompatible_parsed, promoted.revision)
        .await
        .is_err());

    let mut incompatible_draft_manifest = manifest("lifecycle", "2.0.0", 'd');
    incompatible_draft_manifest.id = first.manifest.id;
    let (incompatible_draft, mut incompatible_draft_parsed) =
        parsed_skill(incompatible_draft_manifest, project);
    incompatible_draft_parsed.instructions[0].content =
        "Use the incompatible draft implementation.".into();
    let incompatible_record = repo
        .update(
            &incompatible_draft,
            &incompatible_draft_parsed,
            promoted.revision,
        )
        .await
        .unwrap();
    assert_eq!(
        incompatible_record.compatibility,
        SkillCompatibility::Incompatible
    );
    assert!(repo
        .promote(
            SkillScope::Project,
            Some(&project),
            &first.manifest.id,
            "2.0.0",
            incompatible_record.revision,
            &validation_report(&incompatible_record.parsed, project),
        )
        .await
        .is_err());
}

#[tokio::test]
async fn rollback_restores_pinned_version_without_rewriting_history() {
    let (repo, project) = repository().await;
    let (first, first_parsed) = parsed_skill(manifest("rollback", "1.0.0", 'a'), project);
    repo.create(&first, &first_parsed).await.unwrap();
    let mut second_manifest = manifest("rollback", "1.1.0", 'b');
    second_manifest.id = first.manifest.id;
    let (second, mut second_parsed) = parsed_skill(second_manifest, project);
    second_parsed.instructions[0].content = "Use the rollback candidate.".into();
    let second_record = repo.update(&second, &second_parsed, 1).await.unwrap();
    let promoted = repo
        .promote(
            SkillScope::Project,
            Some(&project),
            &first.manifest.id,
            "1.1.0",
            second_record.revision,
            &validation_report(&second_record.parsed, project),
        )
        .await
        .unwrap();
    let restored = repo
        .rollback(
            SkillScope::Project,
            Some(&project),
            &first.manifest.id,
            "1.0.0",
            promoted.revision,
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

    assert_eq!(restored.skill.manifest.version, "1.0.0");
    assert_eq!(restored.skill.pinned_version.as_deref(), Some("1.0.0"));
    assert_eq!(restored.skill.rollback_version.as_deref(), Some("1.1.0"));
    assert_eq!(
        repo.list_versions(SkillScope::Project, Some(&project), &first.manifest.id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn version_events_expose_provenance_without_instruction_content() {
    let (repo, project) = repository().await;
    let bus = EventBus::bounded(4);
    let mut events = bus.subscribe();
    let repo = repo.with_event_bus(bus);
    let (skill, parsed) = parsed_skill(manifest("observable", "1.0.0", 'a'), project);

    let record = repo.create(&skill, &parsed).await.unwrap();
    let event = events.recv().await.unwrap();
    assert_eq!(event.event_type, EventKind::SkillVersionChanged);
    assert_eq!(event.project_id, project);
    let payload: serde_json::Value = serde_json::from_str(&event.payload).unwrap();
    assert_eq!(payload["version"], "1.0.0");
    assert_eq!(payload["content_hash"], record.content_hash);
    assert_eq!(payload["trace"]["schema_version"], 1);
    assert!(!event.payload.contains("Use this skill as untrusted data"));
    assert!(!event.payload.contains("workspace://skill"));
    assert_eq!(payload["source"]["kind"], "local");
    assert!(payload["source"]["reference_digest"].as_str().is_some());
}

#[tokio::test]
async fn global_version_events_are_explicit_and_redacted() {
    let (repo, _) = repository().await;
    let bus = EventBus::bounded(4);
    let mut events = bus.subscribe();
    let repo = repo.with_global_event_bus(bus);
    let mut manifest = manifest("global-observable", "1.0.0", 'a');
    manifest.scope = SkillScope::Global;
    let document = format!(
        "---\n{}\n---\n# Instructions\nGlobal instructions remain untrusted.",
        serde_json::to_string(&manifest).unwrap()
    );
    let parsed = SkillParser::default()
        .parse(SkillParseRequest {
            document,
            files: vec![SkillFileInput {
                path: "tests/basic.json".into(),
                content: "{\"case\":\"safe\"}".into(),
            }],
            project_id: None,
        })
        .unwrap();
    let skill = Skill::new(parsed.manifest.clone(), None);

    repo.create(&skill, &parsed).await.unwrap();
    let event = events.recv().await.unwrap();
    assert_eq!(event.event_type, EventKind::SkillVersionChanged);
    assert_eq!(event.aggregate_id, skill.manifest.id.to_string());
    assert!(!event
        .payload
        .contains("Global instructions remain untrusted"));
    assert!(!event.payload.contains("workspace://skill"));
}
