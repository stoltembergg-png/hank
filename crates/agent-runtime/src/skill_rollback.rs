//! Explicit, idempotent rollback decision for governed Skill versions.

use agent_core::{BudgetLimits, DomainError, ProjectId};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SKILL_ROLLBACK_CAPABILITY: &str = "skill:rollback";
pub const SKILL_ROLLBACK_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct SkillRollbackRequest {
    pub project_id: ProjectId,
    pub actor_id: String,
    pub trace_id: TraceId,
    pub budget: BudgetLimits,
    pub active_version: String,
    pub target_version: String,
    pub target_digest: String,
    pub known_good: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillRollbackStatus {
    Applied,
    AlreadyApplied,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillRollbackReason {
    RestoredKnownGood,
    AlreadyAtTarget,
    UnknownTarget,
    InvalidIdentity,
    BudgetInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRollbackDecision {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub actor_id: String,
    pub trace_id: TraceId,
    pub from_version: String,
    pub target_version: String,
    pub status: SkillRollbackStatus,
    pub reason: SkillRollbackReason,
    pub active_pointer_changed: bool,
    pub cache_invalidation_required: bool,
    pub decision_digest: String,
}

pub struct SkillRollbackService;

impl SkillRollbackService {
    pub fn decide(request: SkillRollbackRequest) -> Result<SkillRollbackDecision, DomainError> {
        validate(&request)?;
        let (status, reason, changed) = if request.active_version == request.target_version {
            (
                SkillRollbackStatus::AlreadyApplied,
                SkillRollbackReason::AlreadyAtTarget,
                false,
            )
        } else if request.known_good {
            (
                SkillRollbackStatus::Applied,
                SkillRollbackReason::RestoredKnownGood,
                true,
            )
        } else {
            (
                SkillRollbackStatus::Denied,
                SkillRollbackReason::UnknownTarget,
                false,
            )
        };
        let decision_digest = digest_json(&(
            SKILL_ROLLBACK_SCHEMA_VERSION,
            request.project_id,
            &request.actor_id,
            request.trace_id,
            &request.active_version,
            &request.target_version,
            &request.target_digest,
            status,
            reason,
        ));
        Ok(SkillRollbackDecision {
            schema_version: SKILL_ROLLBACK_SCHEMA_VERSION,
            project_id: request.project_id,
            actor_id: request.actor_id,
            trace_id: request.trace_id,
            from_version: request.active_version,
            target_version: request.target_version,
            status,
            reason,
            active_pointer_changed: changed,
            cache_invalidation_required: changed,
            decision_digest,
        })
    }
}

fn validate(request: &SkillRollbackRequest) -> Result<(), DomainError> {
    if request.actor_id.trim().is_empty()
        || request.trace_id.as_uuid().is_nil()
        || request.active_version.trim().is_empty()
        || request.target_version.trim().is_empty()
    {
        return Err(DomainError::Validation(
            "rollback identity is invalid".into(),
        ));
    }
    if request.target_digest.len() != 64 {
        return Err(DomainError::PermissionDenied {
            capability: SKILL_ROLLBACK_CAPABILITY.into(),
            reason: "rollback target digest is invalid".into(),
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
