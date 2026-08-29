//! Versioned improvement candidates; evaluation and activation stay outside this domain.

use thiserror::Error;

const MAX_TEXT: usize = 256;
const MAX_OBSERVATIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Skill,
    Workflow,
    AgentConfig,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskClass {
    Low,
    Medium,
    High,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateStatus {
    Draft,
    Evaluating,
    Approved,
    Rejected,
    RolledBack,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CandidateError {
    #[error("candidate provenance is incomplete")]
    InvalidProvenance,
    #[error("candidate metadata exceeds bounds")]
    BoundsExceeded,
    #[error("candidate owner or project is unauthorized")]
    Unauthorized,
    #[error("candidate lifecycle transition is invalid")]
    InvalidTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImprovementCandidate {
    candidate_id: String,
    project_id: String,
    owner_id: String,
    source_observations: Vec<String>,
    policy_snapshot: String,
    target: TargetKind,
    proposal_digest: String,
    version: u32,
    risk: RiskClass,
    status: CandidateStatus,
    authorized: bool,
}
impl ImprovementCandidate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate_id: &str,
        project_id: &str,
        owner_id: &str,
        source_observations: Vec<&str>,
        policy_snapshot: &str,
        target: TargetKind,
        proposal_digest: &str,
        version: u32,
        risk: RiskClass,
    ) -> Result<Self, CandidateError> {
        if version == 0
            || source_observations.is_empty()
            || source_observations.len() > MAX_OBSERVATIONS
            || [
                candidate_id,
                project_id,
                owner_id,
                policy_snapshot,
                proposal_digest,
            ]
            .iter()
            .any(|value| {
                value.is_empty() || value.len() > MAX_TEXT || value.chars().any(char::is_control)
            })
            || source_observations.iter().any(|value| {
                value.is_empty() || value.len() > MAX_TEXT || value.chars().any(char::is_control)
            })
        {
            return Err(if source_observations.len() > MAX_OBSERVATIONS {
                CandidateError::BoundsExceeded
            } else {
                CandidateError::InvalidProvenance
            });
        }
        Ok(Self {
            candidate_id: candidate_id.into(),
            project_id: project_id.into(),
            owner_id: owner_id.into(),
            source_observations: source_observations.into_iter().map(str::to_owned).collect(),
            policy_snapshot: policy_snapshot.into(),
            target,
            proposal_digest: proposal_digest.into(),
            version,
            risk,
            status: CandidateStatus::Draft,
            authorized: false,
        })
    }
    pub fn status(&self) -> CandidateStatus {
        self.status
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn authorize(&mut self, project_id: &str, owner_id: &str) -> Result<(), CandidateError> {
        if project_id != self.project_id || owner_id != self.owner_id {
            return Err(CandidateError::Unauthorized);
        }
        self.authorized = true;
        Ok(())
    }
    pub fn transition(&mut self, next: CandidateStatus) -> Result<(), CandidateError> {
        let valid = matches!(
            (self.status, next),
            (CandidateStatus::Draft, CandidateStatus::Evaluating)
                | (CandidateStatus::Evaluating, CandidateStatus::Approved)
                | (CandidateStatus::Evaluating, CandidateStatus::Rejected)
                | (CandidateStatus::Approved, CandidateStatus::RolledBack)
        );
        if !valid {
            return Err(CandidateError::InvalidTransition);
        }
        self.status = next;
        Ok(())
    }
    pub fn can_activate(&self) -> bool {
        false
    }
}
