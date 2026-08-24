use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, ProjectId, Resource, SkillFileInput, SkillId,
    SkillManifest, SkillScope,
};
use agent_protocol::ids::TraceId;
use agent_runtime::skill_autonomous_test::{
    SkillAutonomousTestPolicy, SkillAutonomousTestReason, SkillAutonomousTestRequest,
    SkillAutonomousTestService, SkillAutonomousTestStatus,
};
use agent_runtime::skill_candidate::{
    SkillCandidateGenerationService, SkillCandidatePolicy, SkillCandidateProposal,
    SkillCandidateRequest, SkillObservationRef,
};
use agent_runtime::skill_testing::{SkillFixture, SkillTestStep};

fn candidate(
    project_id: ProjectId,
    trace_id: TraceId,
) -> agent_runtime::skill_candidate::SkillCandidate {
    let capability =
        Capability::new(Resource::Skill, Action::Create).with_scope(project_id.to_string());
    let manifest = {
        let mut m = SkillManifest::new("autonomous", "1.1.0", SkillScope::Project);
        m.id = SkillId::from_uuid(uuid::Uuid::from_u128(0x9876));
        m.trace.trace_id = trace_id;
        m.created_at = chrono::DateTime::from_timestamp(1, 0).unwrap();
        m.source.reference = format!("workspace://{project_id}/autonomous");
        m
    };
    let proposal = SkillCandidateProposal {
        document: format!(
            "---\n{}\n---\n# Candidate\nbounded",
            serde_json::to_string(&manifest).unwrap()
        ),
        files: Vec::<SkillFileInput>::new(),
    };
    SkillCandidateGenerationService::generate(SkillCandidateRequest {
        project_id,
        agent_id: "test-agent".into(),
        capability: capability.clone(),
        policy: SkillCandidatePolicy {
            allow: true,
            allowed_capabilities: CapabilitySet::new().insert(capability),
            max_observations: 4,
            max_document_bytes: 4096,
        },
        budget: BudgetLimits::default(),
        trace_id,
        base_version: "1.0.0".into(),
        observations: vec![SkillObservationRef {
            observation_id: "obs-1".into(),
            digest: "a".repeat(64),
            source: "fixture".into(),
        }],
        proposal,
    })
    .unwrap()
}

fn request(project_id: ProjectId, trace_id: TraceId) -> SkillAutonomousTestRequest {
    let capability =
        Capability::new(Resource::Skill, Action::Read).with_scope(project_id.to_string());
    SkillAutonomousTestRequest {
        project_id,
        actor_id: "test-agent".into(),
        capability,
        policy: SkillAutonomousTestPolicy {
            allow: true,
            max_rounds: 4,
            max_depth: 4,
            max_steps: 4,
        },
        budget: BudgetLimits::default(),
        trace_id,
        candidate: candidate(project_id, trace_id),
        fixture: SkillFixture::new(
            project_id,
            SkillId::from_uuid(uuid::Uuid::from_u128(0x9876)),
            "1.1.0",
            trace_id,
            vec![SkillTestStep::AssertLabel {
                label: "safe".into(),
            }],
            4,
        )
        .unwrap(),
        cancel_requested: false,
        sandbox_root: format!("project://{project_id}/skill-test"),
    }
}

#[test]
// @spec:AC-831
fn safe_candidate_runs_bounded_and_never_changes_active_version() {
    let project_id = ProjectId::new();
    let report = SkillAutonomousTestService::run(request(project_id, TraceId::new())).unwrap();
    assert_eq!(report.status, SkillAutonomousTestStatus::Passed);
    assert_eq!(report.rounds, 1);
    assert_eq!(report.depth, 1);
    assert_eq!(report.steps_executed, 1);
    assert!(!report.active_version_changed);
    assert_eq!(report.report_digest.len(), 64);
}

#[test]
// @spec:AC-832
fn cancellation_and_limits_are_terminal_without_execution() {
    let project_id = ProjectId::new();
    let trace = TraceId::new();
    let mut cancelled = request(project_id, trace);
    cancelled.cancel_requested = true;
    let report = SkillAutonomousTestService::run(cancelled).unwrap();
    assert_eq!(report.status, SkillAutonomousTestStatus::Cancelled);
    assert_eq!(report.reasons, vec![SkillAutonomousTestReason::Cancelled]);

    let mut limited = request(project_id, TraceId::new());
    limited.policy.max_steps = 0;
    let report = SkillAutonomousTestService::run(limited).unwrap();
    assert_eq!(report.status, SkillAutonomousTestStatus::TimedOut);
    assert_eq!(report.reasons, vec![SkillAutonomousTestReason::StepLimit]);
}

#[test]
// @spec:AC-833
fn identity_scope_and_sandbox_escape_fail_closed() {
    let project_id = ProjectId::new();
    let trace = TraceId::new();
    let mut wrong_root = request(project_id, trace);
    wrong_root.sandbox_root = "/tmp/skill-test".into();
    assert!(SkillAutonomousTestService::run(wrong_root).is_err());

    let mut wrong_capability = request(project_id, TraceId::new());
    wrong_capability.capability =
        Capability::new(Resource::Skill, Action::Create).with_scope(project_id.to_string());
    assert!(SkillAutonomousTestService::run(wrong_capability).is_err());
}
