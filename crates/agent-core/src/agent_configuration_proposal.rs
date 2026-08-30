//! Proposal-only agent configuration diff; active configuration is immutable here.

use thiserror::Error;

const MAX_TEXT: usize = 512;
const MAX_CHANGES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    Model,
    Personality,
    UserInstruction,
    SystemInstruction,
    SecurityInstruction,
    Tool,
    Memory,
    Autonomy,
    Budget,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigChange {
    pub field: ConfigField,
    pub from: String,
    pub to: String,
}
impl ConfigChange {
    pub fn new(field: ConfigField, from: &str, to: &str) -> Self {
        Self {
            field,
            from: from.into(),
            to: to.into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecedenceClass {
    Agent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    Draft,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigProposalError {
    #[error("configuration proposal identity is invalid")]
    InvalidIdentity,
    #[error("system or security policy is immutable")]
    ImmutablePolicy,
    #[error("configuration policy approval is required")]
    PolicyRequired,
    #[error("configuration proposal is oversized")]
    TooLarge,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProposalRequest {
    pub agent_id: String,
    pub active_version: String,
    pub candidate_id: String,
    pub policy_id: String,
    pub changes: Vec<ConfigChange>,
    pub capability_delta: bool,
    pub autonomy_delta: bool,
    pub budget_delta: bool,
    pub approved: bool,
}
impl ConfigProposalRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent: &str,
        version: &str,
        candidate: &str,
        policy: &str,
        changes: Vec<ConfigChange>,
        capability: bool,
        autonomy: bool,
        budget: bool,
        approved: bool,
    ) -> Result<Self, ConfigProposalError> {
        if [agent, version, candidate, policy]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_TEXT)
        {
            return Err(ConfigProposalError::InvalidIdentity);
        }
        Ok(Self {
            agent_id: agent.into(),
            active_version: version.into(),
            candidate_id: candidate.into(),
            policy_id: policy.into(),
            changes,
            capability_delta: capability,
            autonomy_delta: autonomy,
            budget_delta: budget,
            approved,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfigurationProposal {
    active_version: String,
    precedence: PrecedenceClass,
    status: ProposalStatus,
    fingerprint: String,
}
impl AgentConfigurationProposal {
    pub fn create(request: ConfigProposalRequest) -> Result<Self, ConfigProposalError> {
        if request.changes.is_empty() || request.changes.len() > MAX_CHANGES {
            return Err(ConfigProposalError::TooLarge);
        }
        if request.changes.iter().any(|change| {
            matches!(
                change.field,
                ConfigField::SystemInstruction | ConfigField::SecurityInstruction
            )
        }) {
            return Err(ConfigProposalError::ImmutablePolicy);
        }
        if (request.capability_delta || request.autonomy_delta || request.budget_delta)
            && !request.approved
        {
            return Err(ConfigProposalError::PolicyRequired);
        }
        if request.changes.iter().any(|change| {
            change.from.len() > MAX_TEXT
                || change.to.len() > MAX_TEXT
                || change.from.contains("token=")
                || change.to.contains("token=")
        }) {
            return Err(ConfigProposalError::InvalidIdentity);
        }
        let material = format!(
            "{}|{}|{}|{}|{:?}",
            request.agent_id,
            request.active_version,
            request.candidate_id,
            request.policy_id,
            request.changes
        );
        Ok(Self {
            active_version: request.active_version,
            precedence: PrecedenceClass::Agent,
            status: ProposalStatus::Draft,
            fingerprint: digest(&material),
        })
    }
    pub fn active_version(&self) -> &str {
        &self.active_version
    }
    pub fn precedence(&self) -> PrecedenceClass {
        self.precedence
    }
    pub fn status(&self) -> ProposalStatus {
        self.status
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn can_activate(&self) -> bool {
        false
    }
}
fn digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
