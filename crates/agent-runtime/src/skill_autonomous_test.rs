//! Bounded autonomous testing for Skill candidates.
//!
//! This first slice is a data-only orchestration boundary: it reuses the
//! deterministic fixture runner and never executes a process, tool, network,
//! filesystem operation, or activation transition.

use crate::skill_candidate::{SkillCandidate, SkillCandidateStatus};
use crate::skill_testing::{DeterministicSkillTestRunner, SkillFixture};
use agent_core::{Action, BudgetLimits, Capability, DomainError, ProjectId, Resource};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SKILL_AUTONOMOUS_TEST_CAPABILITY: &str = "skill:test";
pub const SKILL_AUTONOMOUS_TEST_SCHEMA_VERSION: u32 = 1;
const MAX_ROUNDS: u16 = 16;
const MAX_DEPTH: u16 = 16;
const MAX_STEPS: u16 = 64;

#[derive(Debug, Clone)]
pub struct SkillAutonomousTestPolicy {
    pub allow: bool,
    pub max_rounds: u16,
    pub max_depth: u16,
    pub max_steps: u16,
}

#[derive(Debug, Clone)]
pub struct SkillAutonomousTestRequest {
    pub project_id: ProjectId,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: SkillAutonomousTestPolicy,
    pub budget: BudgetLimits,
    pub trace_id: TraceId,
    pub candidate: SkillCandidate,
    pub fixture: SkillFixture,
    pub cancel_requested: bool,
    pub sandbox_root: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAutonomousTestStatus {
    Passed,
    TimedOut,
    Cancelled,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAutonomousTestReason {
    InvalidIdentity,
    CandidateNotDraft,
    ScopeMismatch,
    CapabilityDenied,
    BudgetExceeded,
    RoundLimit,
    DepthLimit,
    StepLimit,
    Cancelled,
    FixtureRejected,
    SandboxEscape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAutonomousTestReport {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub candidate_id: String,
    pub trace_id: TraceId,
    pub status: SkillAutonomousTestStatus,
    pub rounds: u16,
    pub depth: u16,
    pub steps_executed: u16,
    pub candidate_digest: String,
    pub fixture_digest: String,
    pub reasons: Vec<SkillAutonomousTestReason>,
    pub active_version_changed: bool,
    pub report_digest: String,
}

pub struct SkillAutonomousTestService;

impl SkillAutonomousTestService {
    pub fn run(
        request: SkillAutonomousTestRequest,
    ) -> Result<SkillAutonomousTestReport, DomainError> {
        validate(&request)?;
        let candidate_digest = request.candidate.candidate_digest.clone();
        let fixture_digest = digest_json(&request.fixture);
        let (status, rounds, depth, steps, reasons) = if request.cancel_requested {
            (
                SkillAutonomousTestStatus::Cancelled,
                0,
                0,
                0,
                vec![SkillAutonomousTestReason::Cancelled],
            )
        } else if request.policy.max_rounds == 0 || request.policy.max_rounds > MAX_ROUNDS {
            (
                SkillAutonomousTestStatus::TimedOut,
                0,
                0,
                0,
                vec![SkillAutonomousTestReason::RoundLimit],
            )
        } else if request.policy.max_depth == 0 || request.policy.max_depth > MAX_DEPTH {
            (
                SkillAutonomousTestStatus::TimedOut,
                0,
                0,
                0,
                vec![SkillAutonomousTestReason::DepthLimit],
            )
        } else if request.fixture.steps.len() as u16 > request.policy.max_steps
            || request.policy.max_steps > MAX_STEPS
        {
            (
                SkillAutonomousTestStatus::TimedOut,
                0,
                0,
                0,
                vec![SkillAutonomousTestReason::StepLimit],
            )
        } else {
            match DeterministicSkillTestRunner::run(&request.fixture) {
                Ok(report) => (
                    SkillAutonomousTestStatus::Passed,
                    1,
                    1,
                    report.steps_executed,
                    Vec::new(),
                ),
                Err(_) => (
                    SkillAutonomousTestStatus::Quarantined,
                    0,
                    0,
                    0,
                    vec![SkillAutonomousTestReason::FixtureRejected],
                ),
            }
        };
        let fingerprint = (
            &candidate_digest,
            &fixture_digest,
            request.project_id,
            request.trace_id,
            status,
            rounds,
            depth,
            steps,
            &reasons,
        );
        let report_digest = digest_json(&fingerprint);
        Ok(SkillAutonomousTestReport {
            schema_version: SKILL_AUTONOMOUS_TEST_SCHEMA_VERSION,
            project_id: request.project_id,
            candidate_id: request.candidate.candidate_id,
            trace_id: request.trace_id,
            status,
            rounds,
            depth,
            steps_executed: steps,
            candidate_digest,
            fixture_digest,
            reasons,
            active_version_changed: false,
            report_digest,
        })
    }
}

fn validate(request: &SkillAutonomousTestRequest) -> Result<(), DomainError> {
    if request.actor_id.trim().is_empty() || request.actor_id.len() > 128 {
        return Err(DomainError::Validation(
            "autonomous test actor is invalid".into(),
        ));
    }
    let expected =
        Capability::new(Resource::Skill, Action::Read).with_scope(request.project_id.to_string());
    if !request.policy.allow || request.capability != expected {
        return Err(DomainError::PermissionDenied {
            capability: SKILL_AUTONOMOUS_TEST_CAPABILITY.into(),
            reason: "autonomous test capability is not authorized".into(),
        });
    }
    if request.trace_id.as_uuid().is_nil()
        || request.candidate.project_id != request.project_id
        || request.candidate.trace_id != request.trace_id
    {
        return Err(DomainError::Validation(
            "autonomous test identity is inconsistent".into(),
        ));
    }
    if request.candidate.status != SkillCandidateStatus::Draft {
        return Err(DomainError::InvalidStateTransition {
            from: "candidate".into(),
            to: "testing".into(),
        });
    }
    if !request.sandbox_root.starts_with("project://") || request.sandbox_root.len() > 128 {
        return Err(DomainError::PermissionDenied {
            capability: SKILL_AUTONOMOUS_TEST_CAPABILITY.into(),
            reason: "sandbox root is outside project scope".into(),
        });
    }
    request.budget.validate()
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}
