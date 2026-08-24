use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, ParsedSkill, ProjectId, Resource, Skill,
    SkillCompatibility, SkillFile, SkillFileRole, SkillId, SkillManifest, SkillParseRequest,
    SkillParser, SkillScope,
};
use agent_protocol::ids::TraceId;
use agent_runtime::skill_evaluation::{
    SkillEvaluationPolicy, SkillEvaluationReason, SkillEvaluationRequest, SkillEvaluationService,
    SkillEvaluationStatus,
};
use agent_runtime::skill_testing::{
    DeterministicSkillTestRunner, SkillFixture, SkillTestReport, SkillTestStep,
};
use agent_runtime::skill_validation::{
    SkillDependencyNode, SkillValidationPolicy, SkillValidationRequest, SkillValidationService,
};
use agent_runtime::SkillRecord;

struct EvaluationFixture {
    project_id: ProjectId,
    baseline: SkillRecord,
    candidate: ParsedSkill,
    baseline_fixture: SkillFixture,
    candidate_fixture: SkillFixture,
    baseline_report: SkillTestReport,
    candidate_report: SkillTestReport,
    validation: agent_runtime::skill_validation::SkillValidationReport,
    trace_id: TraceId,
}

fn parsed_skill(project_id: ProjectId, manifest: SkillManifest, instruction: &str) -> ParsedSkill {
    let document = format!(
        "---\n{}\n---\n# Instructions\n{}",
        serde_json::to_string(&manifest).unwrap(),
        instruction
    );
    SkillParser::default()
        .parse(SkillParseRequest {
            document,
            files: vec![agent_core::SkillFileInput {
                path: "tests/basic.json".into(),
                content: "{\"case\":\"controlled\"}".into(),
            }],
            project_id: Some(project_id),
        })
        .unwrap()
}

fn manifest(skill_id: SkillId, version: &str, trace_id: TraceId) -> SkillManifest {
    let mut manifest = SkillManifest::new("evaluator", version, SkillScope::Project);
    manifest.id = skill_id;
    manifest.trace.trace_id = trace_id;
    manifest.files.push(SkillFile {
        path: "tests/basic.json".into(),
        role: SkillFileRole::Test,
        digest: "b".repeat(64),
    });
    manifest.tests.push("tests/basic.json".into());
    manifest
}

fn fixture(
    project_id: ProjectId,
    skill_id: SkillId,
    version: &str,
    trace_id: TraceId,
    steps: Vec<SkillTestStep>,
    max_steps: u16,
) -> SkillFixture {
    SkillFixture::new(project_id, skill_id, version, trace_id, steps, max_steps).unwrap()
}

fn validation(
    project_id: ProjectId,
    candidate: &ParsedSkill,
    report: &SkillTestReport,
) -> agent_runtime::skill_validation::SkillValidationReport {
    let capability =
        Capability::new(Resource::Skill, Action::Create).with_scope(project_id.to_string());
    SkillValidationService::validate(
        candidate,
        &SkillValidationRequest {
            project_id,
            skill_id: candidate.manifest.id,
            version: candidate.manifest.version.clone(),
            actor_id: "evaluator-agent".into(),
            capability: capability.clone(),
            policy: SkillValidationPolicy {
                allowed_capabilities: CapabilitySet::new().insert(capability),
            },
            budget: BudgetLimits::default(),
            trace_id: candidate.manifest.trace.trace_id,
            dependency_graph: vec![SkillDependencyNode {
                skill_id: candidate.manifest.id,
                dependencies: Vec::new(),
            }],
        },
        Some(report),
    )
}

fn fixture_data() -> EvaluationFixture {
    let project_id = ProjectId::new();
    let skill_id = SkillId::new();
    let baseline_trace = TraceId::new();
    let candidate_trace = TraceId::new();
    let baseline_parsed = parsed_skill(
        project_id,
        manifest(skill_id, "1.0.0", baseline_trace),
        "Baseline instruction is untrusted data.",
    );
    let candidate = parsed_skill(
        project_id,
        manifest(skill_id, "1.1.0", candidate_trace),
        "Candidate instruction is also untrusted data.",
    );
    let baseline_fixture = fixture(
        project_id,
        skill_id,
        "1.0.0",
        baseline_trace,
        vec![SkillTestStep::AssertLabel {
            label: "baseline-safe".into(),
        }],
        4,
    );
    let candidate_fixture = fixture(
        project_id,
        skill_id,
        "1.1.0",
        candidate_trace,
        vec![SkillTestStep::AssertLabel {
            label: "candidate-safe".into(),
        }],
        4,
    );
    let baseline_report = DeterministicSkillTestRunner::run(&baseline_fixture).unwrap();
    let candidate_report = DeterministicSkillTestRunner::run(&candidate_fixture).unwrap();
    let baseline = SkillRecord {
        skill: Skill::new(baseline_parsed.manifest.clone(), Some(project_id)),
        parsed: baseline_parsed,
        revision: 1,
        version_id: "baseline-version".into(),
        content_hash: "a".repeat(64),
        parent_version: None,
        compatibility: SkillCompatibility::Initial,
    };
    let validation = validation(project_id, &candidate, &candidate_report);
    assert_eq!(
        validation.status,
        agent_runtime::skill_validation::SkillValidationStatus::Passed
    );
    EvaluationFixture {
        project_id,
        baseline,
        candidate,
        baseline_fixture,
        candidate_fixture,
        baseline_report,
        candidate_report,
        validation,
        trace_id: candidate_trace,
    }
}

fn request(data: &EvaluationFixture) -> SkillEvaluationRequest {
    let capability =
        Capability::new(Resource::Skill, Action::Read).with_scope(data.project_id.to_string());
    SkillEvaluationRequest {
        project_id: data.project_id,
        actor_id: "evaluator-agent".into(),
        capability,
        policy: SkillEvaluationPolicy {
            allow: true,
            allowed_capabilities: CapabilitySet::new().insert(
                Capability::new(Resource::Skill, Action::Read)
                    .with_scope(data.project_id.to_string()),
            ),
            max_tests: 2,
            max_steps: 8,
        },
        budget: BudgetLimits::default(),
        trace_id: data.trace_id,
        baseline: data.baseline.clone(),
        candidate: data.candidate.clone(),
        validation: data.validation.clone(),
        baseline_fixture: data.baseline_fixture.clone(),
        candidate_fixture: data.candidate_fixture.clone(),
        baseline_report: data.baseline_report.clone(),
        candidate_report: data.candidate_report.clone(),
    }
}

fn baseline_bytes(record: &SkillRecord) -> Vec<u8> {
    serde_json::to_vec(&(
        &record.skill,
        &record.parsed,
        record.revision,
        &record.version_id,
        &record.content_hash,
        &record.parent_version,
        &record.compatibility,
    ))
    .unwrap()
}

#[test]
// @spec:AC-810
fn safe_candidate_gets_a_deterministic_pass_report_without_raw_content() {
    let data = fixture_data();
    let report = SkillEvaluationService::evaluate(request(&data)).unwrap();

    assert_eq!(report.status, SkillEvaluationStatus::Passed);
    assert_eq!(report.baseline_score, 100);
    assert_eq!(report.candidate_score, 100);
    assert_eq!(report.score_delta, 0);
    assert_eq!(report.rollback_version.as_deref(), Some("1.0.0"));
    assert!(!report.report_digest.is_empty());
    assert_eq!(report.policy_digest.len(), 64);
    assert_eq!(report.budget_digest.len(), 64);
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("Candidate instruction"));
}

#[test]
// @spec:AC-811
fn regression_is_non_active_and_baseline_remains_byte_for_byte_unchanged() {
    let data = fixture_data();
    let before = baseline_bytes(&data.baseline);
    let mut request = request(&data);
    request.candidate_report.status = "failed".into();

    let report = SkillEvaluationService::evaluate(request).unwrap();

    assert_eq!(report.status, SkillEvaluationStatus::Failed);
    assert_eq!(report.candidate_score, 0);
    assert_eq!(report.score_delta, -100);
    assert_eq!(baseline_bytes(&data.baseline), before);
}

#[test]
// @spec:AC-812
fn injection_or_tampered_validation_is_quarantined_without_self_approval() {
    let data = fixture_data();
    let mut request = request(&data);
    request.candidate.quarantined = true;
    request.validation.content_digest = "f".repeat(64);

    let report = SkillEvaluationService::evaluate(request).unwrap();

    assert_eq!(report.status, SkillEvaluationStatus::Quarantined);
    assert!(report
        .reasons
        .contains(&SkillEvaluationReason::ValidationRejected));
}

#[test]
// @spec:AC-813
fn budget_and_timeout_limits_are_non_active_and_bounded() {
    let data = fixture_data();
    let mut budget_request = request(&data);
    budget_request.policy.max_tests = 1;
    let budget_report = SkillEvaluationService::evaluate(budget_request).unwrap();
    assert_eq!(budget_report.status, SkillEvaluationStatus::Quarantined);
    assert!(budget_report
        .reasons
        .contains(&SkillEvaluationReason::BudgetExceeded));

    let mut timeout_request = request(&data);
    timeout_request.policy.max_steps = 0;
    let timeout_report = SkillEvaluationService::evaluate(timeout_request).unwrap();
    assert_eq!(timeout_report.status, SkillEvaluationStatus::TimedOut);
}

#[test]
// @spec:AC-814
fn flaky_or_inconclusive_evidence_cannot_pass() {
    let data = fixture_data();
    let mut request = request(&data);
    request.candidate_report.status = "inconclusive".into();

    let report = SkillEvaluationService::evaluate(request).unwrap();

    assert_eq!(report.status, SkillEvaluationStatus::Inconclusive);
    assert_ne!(report.status, SkillEvaluationStatus::Passed);
}

#[test]
// @spec:AC-815
fn identical_evaluation_is_idempotent_and_deduped_by_report_digest() {
    let data = fixture_data();
    let first = SkillEvaluationService::evaluate(request(&data)).unwrap();
    let second = SkillEvaluationService::evaluate(request(&data)).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.report_digest, second.report_digest);

    let mut policy_changed = request(&data);
    policy_changed.policy.max_steps = 7;
    let policy_report = SkillEvaluationService::evaluate(policy_changed).unwrap();
    assert_ne!(first.policy_digest, policy_report.policy_digest);
    assert_ne!(first.report_digest, policy_report.report_digest);

    let mut budget_changed = request(&data);
    budget_changed.budget.max_tokens += 1;
    let budget_report = SkillEvaluationService::evaluate(budget_changed).unwrap();
    assert_ne!(first.budget_digest, budget_report.budget_digest);
    assert_ne!(first.report_digest, budget_report.report_digest);
}

#[test]
// @spec:AC-816
fn capability_drift_and_wrong_scope_fail_closed() {
    let data = fixture_data();
    let mut drifted = request(&data);
    drifted.candidate.manifest.capabilities.push(
        Capability::new(Resource::File, Action::Read).with_scope(data.project_id.to_string()),
    );
    let report = SkillEvaluationService::evaluate(drifted).unwrap();
    assert_eq!(report.status, SkillEvaluationStatus::Quarantined);
    assert!(report
        .reasons
        .contains(&SkillEvaluationReason::CapabilityDrift));

    let mut wrong_scope = request(&data);
    wrong_scope.project_id = ProjectId::new();
    assert!(SkillEvaluationService::evaluate(wrong_scope).is_err());
}
