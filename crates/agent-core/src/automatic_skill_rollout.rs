//! Bounded rollout eligibility; does not mutate active skills or runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloutRequest {
    pub candidate_id: String,
    pub skill_version: String,
    pub project_id: String,
    pub proposal: bool,
    pub evaluation: bool,
    pub regression: bool,
    pub score: bool,
    pub rollback: bool,
    pub all_evidence: bool,
    pub scope_allowed: bool,
}
impl RolloutRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate: &str,
        version: &str,
        project: &str,
        proposal: bool,
        evaluation: bool,
        regression: bool,
        score: bool,
        rollback: bool,
    ) -> Result<Self, RolloutError> {
        if candidate.is_empty() || version.is_empty() || project.is_empty() {
            return Err(RolloutError::InvalidIdentity);
        }
        Ok(Self {
            candidate_id: candidate.into(),
            skill_version: version.into(),
            project_id: project.into(),
            proposal,
            evaluation,
            regression,
            score,
            rollback,
            all_evidence: true,
            scope_allowed: true,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Stable,
    Failed,
    KillSwitch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloutStatus {
    CanaryReady,
    Blocked,
    Stopped,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloutScope {
    ProjectCanary,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloutError {
    InvalidIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rollout {
    status: RolloutStatus,
    scope: RolloutScope,
    global: bool,
}
impl Rollout {
    pub fn evaluate(request: RolloutRequest, health: Health) -> Result<Self, RolloutError> {
        if !request.all_evidence
            || !request.proposal
            || !request.evaluation
            || !request.regression
            || !request.score
            || !request.rollback
            || !request.scope_allowed
        {
            return Ok(Self {
                status: RolloutStatus::Blocked,
                scope: RolloutScope::ProjectCanary,
                global: false,
            });
        }
        if matches!(health, Health::Failed | Health::KillSwitch) {
            return Ok(Self {
                status: RolloutStatus::Stopped,
                scope: RolloutScope::ProjectCanary,
                global: false,
            });
        }
        Ok(Self {
            status: RolloutStatus::CanaryReady,
            scope: RolloutScope::ProjectCanary,
            global: false,
        })
    }
    pub fn status(&self) -> RolloutStatus {
        self.status
    }
    pub fn scope(&self) -> RolloutScope {
        self.scope
    }
    pub fn global_activation(&self) -> bool {
        self.global
    }
}
