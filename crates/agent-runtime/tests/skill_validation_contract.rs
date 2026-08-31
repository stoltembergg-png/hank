use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, ParsedSkill, ProjectId, Resource, SkillFile,
    SkillFileInput, SkillFileRole, SkillLink, SkillLinkKind, SkillManifest, SkillParseRequest,
    SkillParser, SkillScope,
};
use agent_runtime::skill_testing::{DeterministicSkillTestRunner, SkillFixture, SkillTestStep};
use agent_runtime::skill_validation::{
    SkillDependencyNode, SkillValidationPolicy, SkillValidationReason, SkillValidationRequest,
    SkillValidationService, SkillValidationStatus,
};

fn parsed_skill(project_id: ProjectId) -> (ParsedSkill, SkillFixture) {
    let mut manifest = SkillManifest::new("reviewer", "1.0.0", SkillScope::Project);
    manifest.files.push(SkillFile {
        path: "tests/basic.json".into(),
        role: SkillFileRole::Test,
        digest: "b".repeat(64),
    });
    manifest.tests.push("tests/basic.json".into());
    let document = format!(
        "---\n{}\n---\n# Instructions\nUse only the declared review behavior.",
        serde_json::to_string(&manifest).unwrap()
    );
    let trace_id = manifest.trace.trace_id;
    let fixture = SkillFixture::new(
        project_id,
        manifest.id,
        manifest.version.clone(),
        trace_id,
        vec![SkillTestStep::AssertLabel {
            label: "manifest-valid".into(),
        }],
        4,
    )
    .unwrap();
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
    (parsed, fixture)
}

fn request(parsed: &ParsedSkill, project_id: ProjectId) -> SkillValidationRequest {
    let capability =
        Capability::new(Resource::Skill, Action::Configure).with_scope(project_id.to_string());
    SkillValidationRequest {
        project_id,
        skill_id: parsed.manifest.id,
        version: parsed.manifest.version.clone(),
        actor_id: "operator-1".into(),
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
    }
}

fn report(
    _parsed: &ParsedSkill,
    fixture: &SkillFixture,
) -> agent_runtime::skill_testing::SkillTestReport {
    DeterministicSkillTestRunner::run(fixture).unwrap()
}

#[test]
// @spec:AC-796
fn safe_skill_passes_all_validation_gates_with_redacted_report() {
    let project_id = ProjectId::new();
    let (parsed, fixture) = parsed_skill(project_id);
    let request = request(&parsed, project_id);
    let test_report = report(&parsed, &fixture);

    let validation = SkillValidationService::validate(&parsed, &request, Some(&test_report));

    assert_eq!(validation.status, SkillValidationStatus::Passed);
    assert!(validation.quarantine_reasons.is_empty());
    assert!(validation.rules.iter().all(|rule| rule.passed));
    assert_eq!(validation.project_id, project_id);
    assert_eq!(validation.skill_id, parsed.manifest.id);
    assert_eq!(validation.version, "1.0.0");
    assert_eq!(validation.trace_id, parsed.manifest.trace.trace_id);
    assert_eq!(validation.report_digest.len(), 64);
    assert!(!serde_json::to_string(&validation)
        .unwrap()
        .contains("Use only the declared review behavior"));
}

#[test]
// @spec:AC-797
fn missing_tests_and_parser_quarantine_fail_closed() {
    let project_id = ProjectId::new();
    let (mut parsed, fixture) = parsed_skill(project_id);
    let request = request(&parsed, project_id);
    let test_report = report(&parsed, &fixture);

    parsed.manifest.tests.clear();
    let missing_tests = SkillValidationService::validate(&parsed, &request, Some(&test_report));
    assert_eq!(missing_tests.status, SkillValidationStatus::Quarantined);
    assert!(missing_tests
        .quarantine_reasons
        .contains(&SkillValidationReason::TestsMissing));

    parsed.quarantined = true;
    let quarantined = SkillValidationService::validate(&parsed, &request, Some(&test_report));
    assert!(quarantined
        .quarantine_reasons
        .contains(&SkillValidationReason::ParserQuarantine));
}

#[test]
// @spec:AC-798
fn capability_mismatch_and_unsupported_capability_are_quarantined() {
    let project_id = ProjectId::new();
    let (mut parsed, fixture) = parsed_skill(project_id);
    let request = request(&parsed, project_id);
    let test_report = report(&parsed, &fixture);
    parsed
        .manifest
        .capabilities
        .push(Capability::new(Resource::Process, Action::Execute));

    let validation = SkillValidationService::validate(&parsed, &request, Some(&test_report));

    assert_eq!(validation.status, SkillValidationStatus::Quarantined);
    assert!(validation
        .quarantine_reasons
        .contains(&SkillValidationReason::CapabilityUnsupported));
    assert!(validation
        .quarantine_reasons
        .contains(&SkillValidationReason::CapabilityNotAllowed));
}

#[test]
// @spec:AC-799
fn path_escape_and_dependency_cycle_are_blocked() {
    let project_id = ProjectId::new();
    let (mut parsed, fixture) = parsed_skill(project_id);
    parsed.links.push(SkillLink {
        source_path: "SKILL.md".into(),
        target: "../outside/secrets.txt".into(),
        kind: SkillLinkKind::Internal,
        line: 8,
    });
    let mut request = request(&parsed, project_id);
    let dependency = agent_core::SkillId::new();
    request.dependency_graph = vec![
        SkillDependencyNode {
            skill_id: parsed.manifest.id,
            dependencies: vec![dependency],
        },
        SkillDependencyNode {
            skill_id: dependency,
            dependencies: vec![parsed.manifest.id],
        },
    ];
    let test_report = report(&parsed, &fixture);

    let validation = SkillValidationService::validate(&parsed, &request, Some(&test_report));

    assert!(validation
        .quarantine_reasons
        .contains(&SkillValidationReason::PathEscape));
    assert!(validation
        .quarantine_reasons
        .contains(&SkillValidationReason::DependencyCycle));
}

#[test]
// @spec:AC-800
fn budget_overflow_and_test_identity_mismatch_are_blocked() {
    let project_id = ProjectId::new();
    let (mut parsed, fixture) = parsed_skill(project_id);
    let request = request(&parsed, project_id);
    parsed.manifest.budget.max_tokens = request.budget.max_tokens + 1;
    let mut test_report = report(&parsed, &fixture);
    test_report.version = "9.9.9".into();

    let validation = SkillValidationService::validate(&parsed, &request, Some(&test_report));

    assert!(validation
        .quarantine_reasons
        .contains(&SkillValidationReason::BudgetExceeded));
    assert!(validation
        .quarantine_reasons
        .contains(&SkillValidationReason::TestEvidenceMismatch));
}

#[test]
// @spec:AC-801
fn validation_is_deterministic_and_never_mutates_the_candidate() {
    let project_id = ProjectId::new();
    let (parsed, fixture) = parsed_skill(project_id);
    let request = request(&parsed, project_id);
    let test_report = report(&parsed, &fixture);
    let before = serde_json::to_vec(&parsed).unwrap();

    let first = SkillValidationService::validate(&parsed, &request, Some(&test_report));
    let second = SkillValidationService::validate(&parsed, &request, Some(&test_report));

    assert_eq!(first, second);
    assert_eq!(before, serde_json::to_vec(&parsed).unwrap());
    assert_eq!(first.status, SkillValidationStatus::Passed);
}

#[test]
// @spec:AC-802
fn lifecycle_evidence_rejects_tampering_or_a_different_candidate() {
    let project_id = ProjectId::new();
    let (parsed, fixture) = parsed_skill(project_id);
    let request = request(&parsed, project_id);
    let test_report = report(&parsed, &fixture);
    let validation = SkillValidationService::validate(&parsed, &request, Some(&test_report));

    assert!(SkillValidationService::report_is_approved(
        &parsed,
        project_id,
        parsed.manifest.id,
        &parsed.manifest.version,
        &validation,
        &request.policy,
        &request.budget,
    ));

    let mut tampered = validation.clone();
    tampered.content_digest = "0".repeat(64);
    assert!(!SkillValidationService::report_is_approved(
        &parsed,
        project_id,
        parsed.manifest.id,
        &parsed.manifest.version,
        &tampered,
        &request.policy,
        &request.budget,
    ));
}

#[test]
// Security test: Verify that forged policy/budget digests are rejected
fn lifecycle_evidence_rejects_forged_policy_and_budget() {
    let project_id = ProjectId::new();
    let (parsed, fixture) = parsed_skill(project_id);
    let request = request(&parsed, project_id);
    let test_report = report(&parsed, &fixture);
    let validation = SkillValidationService::validate(&parsed, &request, Some(&test_report));

    // Validation should pass with correct policy and budget
    assert!(SkillValidationService::report_is_approved(
        &parsed,
        project_id,
        parsed.manifest.id,
        &parsed.manifest.version,
        &validation,
        &request.policy,
        &request.budget,
    ));

    // Create a different policy with more permissive capabilities
    let attacker_capability = Capability::new(Resource::File, Action::Create);
    let attacker_policy = SkillValidationPolicy {
        allowed_capabilities: CapabilitySet::new().insert(attacker_capability),
    };

    // Validation should fail when verified against a different policy
    assert!(!SkillValidationService::report_is_approved(
        &parsed,
        project_id,
        parsed.manifest.id,
        &parsed.manifest.version,
        &validation,
        &attacker_policy,
        &request.budget,
    ));

    // Create a different budget with higher limits
    let attacker_budget = BudgetLimits {
        max_tokens: request.budget.max_tokens + 1000,
        ..BudgetLimits::default()
    };

    // Validation should fail when verified against a different budget
    assert!(!SkillValidationService::report_is_approved(
        &parsed,
        project_id,
        parsed.manifest.id,
        &parsed.manifest.version,
        &validation,
        &request.policy,
        &attacker_budget,
    ));
}
