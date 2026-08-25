//! Pure, fail-closed Skill lifecycle curator.

use agent_core::{DomainError, ProjectId, Skill, SkillStatus};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SKILL_CURATOR_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct SkillLifecycleRequest {
    pub project_id: ProjectId,
    pub actor_id: String,
    pub trace_id: TraceId,
    pub skill: Skill,
    pub target: SkillStatus,
    pub validation_passed: bool,
    pub evaluation_passed: bool,
    pub autonomous_test_passed: bool,
    pub activation_allowed: bool,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillLifecycleDecisionStatus {
    Allowed,
    AlreadyApplied,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillLifecycleReason {
    LegalTransition,
    AlreadyAtTarget,
    InvalidTransition,
    EvidenceMissing,
    ScopeMismatch,
    RollbackMissing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillLifecycleDecision {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub trace_id: TraceId,
    pub from: SkillStatus,
    pub target: SkillStatus,
    pub status: SkillLifecycleDecisionStatus,
    pub reason: SkillLifecycleReason,
    pub version: String,
    pub state_changed: bool,
    pub decision_digest: String,
}

pub struct SkillLifecycleCurator;

impl SkillLifecycleCurator {
    pub fn decide(request: SkillLifecycleRequest) -> Result<SkillLifecycleDecision, DomainError> {
        if request.actor_id.trim().is_empty() || request.trace_id.as_uuid().is_nil() {
            return Err(DomainError::Validation(
                "lifecycle identity is invalid".into(),
            ));
        }
        if request.skill.project_id != Some(request.project_id) {
            return Err(DomainError::PermissionDenied {
                capability: "skill:lifecycle".into(),
                reason: "skill is outside project scope".into(),
            });
        }
        let from = request.skill.status;
        let (status, reason, changed) = if from == request.target {
            (
                SkillLifecycleDecisionStatus::AlreadyApplied,
                SkillLifecycleReason::AlreadyAtTarget,
                false,
            )
        } else if !from.can_transition_to(request.target) {
            (
                SkillLifecycleDecisionStatus::Denied,
                SkillLifecycleReason::InvalidTransition,
                false,
            )
        } else if (request.target == SkillStatus::Testing && !request.validation_passed)
            || (request.target == SkillStatus::Active
                && (!request.validation_passed
                    || !request.evaluation_passed
                    || !request.autonomous_test_passed
                    || !request.activation_allowed))
        {
            (
                SkillLifecycleDecisionStatus::Denied,
                SkillLifecycleReason::EvidenceMissing,
                false,
            )
        } else if request.target == SkillStatus::Active && !request.rollback_available {
            (
                SkillLifecycleDecisionStatus::Denied,
                SkillLifecycleReason::RollbackMissing,
                false,
            )
        } else {
            (
                SkillLifecycleDecisionStatus::Allowed,
                SkillLifecycleReason::LegalTransition,
                true,
            )
        };
        let decision_digest = digest_json(&(
            SKILL_CURATOR_SCHEMA_VERSION,
            request.project_id,
            request.trace_id,
            from,
            request.target,
            &request.skill.manifest.version,
            status,
            reason,
        ));
        Ok(SkillLifecycleDecision {
            schema_version: SKILL_CURATOR_SCHEMA_VERSION,
            project_id: request.project_id,
            trace_id: request.trace_id,
            from,
            target: request.target,
            status,
            reason,
            version: request.skill.manifest.version,
            state_changed: changed,
            decision_digest,
        })
    }
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}
