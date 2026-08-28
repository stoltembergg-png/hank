//! Fail-closed activation decision for governed Skill lifecycle transitions.

use agent_core::{
    AutonomyDecision, AutonomyOperation, AutonomyPolicy, BudgetLimits, DomainError, ProjectId,
};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SKILL_ACTIVATE_CAPABILITY: &str = "skill:activate";
pub const SKILL_ACTIVATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct SkillActivationRequest {
    pub project_id: ProjectId,
    pub actor_id: String,
    pub trace_id: TraceId,
    pub policy: AutonomyPolicy,
    pub budget: BudgetLimits,
    pub candidate_version: String,
    pub candidate_digest: String,
    pub validation_digest: String,
    pub evaluation_digest: String,
    pub autonomous_test_digest: String,
    pub human_approval: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillActivationStatus {
    Allowed,
    RequiresApproval,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillActivationReason {
    AllowedByPolicy,
    HumanApprovalRequired,
    AutonomyDenied,
    EvidenceMissing,
    IdentityInvalid,
    BudgetInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillActivationDecision {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub actor_id: String,
    pub trace_id: TraceId,
    pub candidate_version: String,
    pub candidate_digest: String,
    pub status: SkillActivationStatus,
    pub reason: SkillActivationReason,
    pub evidence_digest: String,
    pub active_pointer_changed: bool,
    pub decision_digest: String,
}

pub struct SkillActivationPolicy;

impl SkillActivationPolicy {
    pub fn decide(request: SkillActivationRequest) -> Result<SkillActivationDecision, DomainError> {
        validate(&request)?;
        let operation = AutonomyOperation::ModifySkill;
        let autonomy = request.policy.evaluate(operation);
        let (status, reason) = match autonomy {
            AutonomyDecision::Allow => (
                SkillActivationStatus::Allowed,
                SkillActivationReason::AllowedByPolicy,
            ),
            AutonomyDecision::RequireHumanApproval if request.human_approval => (
                SkillActivationStatus::Allowed,
                SkillActivationReason::AllowedByPolicy,
            ),
            AutonomyDecision::RequireHumanApproval => (
                SkillActivationStatus::RequiresApproval,
                SkillActivationReason::HumanApprovalRequired,
            ),
            AutonomyDecision::Deny => (
                SkillActivationStatus::Denied,
                SkillActivationReason::AutonomyDenied,
            ),
        };
        let evidence_digest = digest_json(&(
            &request.candidate_digest,
            &request.validation_digest,
            &request.evaluation_digest,
            &request.autonomous_test_digest,
        ));
        let decision_digest = digest_json(&(
            SKILL_ACTIVATION_SCHEMA_VERSION,
            request.project_id,
            &request.actor_id,
            request.trace_id,
            &request.candidate_version,
            &request.candidate_digest,
            status,
            reason,
            &evidence_digest,
        ));
        Ok(SkillActivationDecision {
            schema_version: SKILL_ACTIVATION_SCHEMA_VERSION,
            project_id: request.project_id,
            actor_id: request.actor_id,
            trace_id: request.trace_id,
            candidate_version: request.candidate_version,
            candidate_digest: request.candidate_digest,
            status,
            reason,
            evidence_digest,
            active_pointer_changed: false,
            decision_digest,
        })
    }
}

fn validate(request: &SkillActivationRequest) -> Result<(), DomainError> {
    if request.actor_id.trim().is_empty()
        || request.trace_id.as_uuid().is_nil()
        || request.candidate_version.trim().is_empty()
    {
        return Err(DomainError::Validation(
            "activation identity is invalid".into(),
        ));
    }
    if [
        request.candidate_digest.as_str(),
        request.validation_digest.as_str(),
        request.evaluation_digest.as_str(),
        request.autonomous_test_digest.as_str(),
    ]
    .iter()
    .any(|digest| digest.len() != 64)
    {
        return Err(deny_missing_evidence(request));
    }
    request.budget.validate()
}

fn deny_missing_evidence(_request: &SkillActivationRequest) -> DomainError {
    DomainError::PermissionDenied {
        capability: SKILL_ACTIVATE_CAPABILITY.into(),
        reason: "activation evidence is incomplete".into(),
    }
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> String {
    Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
