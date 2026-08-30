//! Pure self-evaluation orchestration contract; execution remains external.

use thiserror::Error;

const MAX_TEXT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Validation,
    Tests,
    Security,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionStatus {
    Blocked,
    Rejected,
    Approved,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluatorOutcome {
    Crashed,
    Rejected,
    Approved,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvaluationError {
    #[error("evaluation snapshot is incomplete")]
    InvalidSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationRequest {
    pub candidate_id: String,
    pub project_id: String,
    pub owner_id: String,
    pub candidate_version: String,
    pub snapshot_sha: String,
    pub policy_present: bool,
    pub tests_present: bool,
    pub security_present: bool,
}
impl EvaluationRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate: &str,
        project: &str,
        owner: &str,
        version: &str,
        sha: &str,
        policy: bool,
        tests: bool,
        security: bool,
    ) -> Result<Self, EvaluationError> {
        if [candidate, project, owner, version, sha]
            .iter()
            .any(|value| {
                value.is_empty() || value.len() > MAX_TEXT || value.chars().any(char::is_control)
            })
        {
            return Err(EvaluationError::InvalidSnapshot);
        }
        Ok(Self {
            candidate_id: candidate.into(),
            project_id: project.into(),
            owner_id: owner.into(),
            candidate_version: version.into(),
            snapshot_sha: sha.into(),
            policy_present: policy,
            tests_present: tests,
            security_present: security,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    candidate_id: String,
    candidate_version: String,
    snapshot_sha: String,
    status: DecisionStatus,
    reason: String,
    stages: [Stage; 3],
}
impl DecisionRecord {
    pub fn status(&self) -> DecisionStatus {
        self.status
    }
    pub fn required_stages(&self) -> &[Stage; 3] {
        &self.stages
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn snapshot_sha(&self) -> &str {
        &self.snapshot_sha
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
    pub fn can_activate(&self) -> bool {
        false
    }
}

pub struct SelfEvaluationWorkflow;
impl SelfEvaluationWorkflow {
    pub fn start(request: EvaluationRequest) -> Result<DecisionRecord, EvaluationError> {
        let missing = [
            (!request.policy_present, "policy"),
            (!request.tests_present, "tests"),
            (!request.security_present, "security"),
        ];
        let reason = missing
            .iter()
            .find_map(|(absent, name)| absent.then_some(*name));
        Ok(record(
            request,
            reason.map_or(DecisionStatus::Blocked, |_| DecisionStatus::Blocked),
            reason.unwrap_or("evaluation stages pending"),
        ))
    }
    pub fn from_outcome(
        request: EvaluationRequest,
        outcome: EvaluatorOutcome,
    ) -> Result<DecisionRecord, EvaluationError> {
        let (status, reason) = match outcome {
            EvaluatorOutcome::Approved => (DecisionStatus::Approved, "external evaluator approved"),
            EvaluatorOutcome::Rejected => (DecisionStatus::Rejected, "external evaluator rejected"),
            EvaluatorOutcome::Crashed => (
                DecisionStatus::Blocked,
                "evaluator crashed; approval unavailable",
            ),
        };
        Ok(record(request, status, reason))
    }
}
fn record(request: EvaluationRequest, status: DecisionStatus, reason: &str) -> DecisionRecord {
    DecisionRecord {
        candidate_id: request.candidate_id,
        candidate_version: request.candidate_version,
        snapshot_sha: request.snapshot_sha,
        status,
        reason: reason.into(),
        stages: [Stage::Validation, Stage::Tests, Stage::Security],
    }
}
