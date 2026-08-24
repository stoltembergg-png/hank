//! Project/agent-scoped memory policy and deterministic fail-closed resolution.

use crate::{MemoryType, ProjectId};
use agent_protocol::{Action, AgentId, AutonomyLevel, Capability, Resource};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MEMORY_POLICY_SCHEMA_VERSION: u32 = 1;
const MAX_ALLOWED_TYPES: usize = 6;
const MAX_TOKENS: u32 = 1_000_000;
const MAX_COST_MICROS: u64 = 100_000_000;
const MAX_RETENTION_DAYS: u32 = 36_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryApprovalMode {
    Always,
    CandidateOnly,
    HumanForApproved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPolicyLayer {
    System,
    Security,
    Project,
    Agent,
}

impl MemoryPolicyLayer {
    fn precedence(self) -> u8 {
        match self {
            Self::System => 4,
            Self::Security => 3,
            Self::Project => 2,
            Self::Agent => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryPolicy {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub version: u64,
    pub read: bool,
    pub write: bool,
    pub learn: bool,
    pub allowed_types: Vec<MemoryType>,
    pub max_tokens: u32,
    pub max_cost_micros: u64,
    pub retention_days: u32,
    pub approval_mode: MemoryApprovalMode,
    pub autonomy_level: AutonomyLevel,
    pub allow_rollback: bool,
}

impl Default for MemoryPolicy {
    fn default() -> Self {
        Self {
            schema_version: MEMORY_POLICY_SCHEMA_VERSION,
            project_id: ProjectId::new(),
            agent_id: AgentId::new(),
            version: 1,
            read: false,
            write: false,
            learn: false,
            allowed_types: Vec::new(),
            max_tokens: 0,
            max_cost_micros: 0,
            retention_days: 0,
            approval_mode: MemoryApprovalMode::HumanForApproved,
            autonomy_level: AutonomyLevel::None,
            allow_rollback: false,
        }
    }
}

impl MemoryPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != MEMORY_POLICY_SCHEMA_VERSION {
            return Err("unsupported memory policy schema".into());
        }
        if self.version == 0 || self.max_tokens > MAX_TOKENS {
            return Err("memory policy version or token limit is invalid".into());
        }
        if self.max_cost_micros > MAX_COST_MICROS || self.retention_days > MAX_RETENTION_DAYS {
            return Err("memory policy cost or retention limit is invalid".into());
        }
        if self.allowed_types.len() > MAX_ALLOWED_TYPES {
            return Err("memory policy contains too many memory types".into());
        }
        let mut types = HashSet::new();
        if self
            .allowed_types
            .iter()
            .any(|memory_type| !types.insert(*memory_type))
        {
            return Err("memory policy contains duplicate memory types".into());
        }
        if self.write && self.max_tokens == 0 {
            return Err("memory writes require a positive token budget".into());
        }
        if self.learn && !self.write {
            return Err("memory learning requires write permission".into());
        }
        Ok(())
    }

    pub fn allows_type(&self, memory_type: MemoryType) -> bool {
        self.allowed_types.contains(&memory_type)
    }

    pub fn capability(action: Action, project_id: &ProjectId) -> Capability {
        Capability {
            resource: Resource::Memory,
            action,
            scope: Some(project_id.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPolicyEntry {
    pub layer: MemoryPolicyLayer,
    pub policy: MemoryPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPolicyAction {
    Read,
    Write,
    Learn,
}

#[derive(Debug, Clone)]
pub struct MemoryPolicyRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub action: MemoryPolicyAction,
    pub memory_type: MemoryType,
    pub requested_tokens: u32,
    pub requested_cost_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPolicyDecision {
    pub allowed: bool,
    pub reason: &'static str,
    pub policy_version: Option<u64>,
    pub layer: Option<MemoryPolicyLayer>,
}

pub struct MemoryPolicyResolver;

impl MemoryPolicyResolver {
    pub fn resolve(
        request: &MemoryPolicyRequest,
        entries: &[MemoryPolicyEntry],
    ) -> MemoryPolicyDecision {
        let mut applicable: Vec<_> = entries
            .iter()
            .filter(|entry| {
                entry.policy.project_id == request.project_id
                    && entry.policy.agent_id == request.agent_id
            })
            .collect();
        applicable.sort_by_key(|entry| std::cmp::Reverse(entry.layer.precedence()));

        let Some(highest) = applicable.first() else {
            return deny("missing memory policy", None, None);
        };
        if applicable
            .iter()
            .any(|entry| entry.policy.validate().is_err())
        {
            return deny("invalid memory policy", None, None);
        }
        if applicable.iter().any(|entry| {
            let policy = &entry.policy;
            !policy.allows_type(request.memory_type)
                || request.requested_tokens > policy.max_tokens
                || request.requested_cost_micros > policy.max_cost_micros
                || !action_allowed(policy, request.action)
        }) {
            return deny(
                "memory policy denied action or bound",
                Some(highest.policy.version),
                Some(highest.layer),
            );
        }
        MemoryPolicyDecision {
            allowed: true,
            reason: "memory policy allowed",
            policy_version: Some(highest.policy.version),
            layer: Some(highest.layer),
        }
    }
}

fn action_allowed(policy: &MemoryPolicy, action: MemoryPolicyAction) -> bool {
    match action {
        MemoryPolicyAction::Read => policy.read,
        MemoryPolicyAction::Write => policy.write,
        MemoryPolicyAction::Learn => policy.learn,
    }
}

fn deny(
    reason: &'static str,
    policy_version: Option<u64>,
    layer: Option<MemoryPolicyLayer>,
) -> MemoryPolicyDecision {
    MemoryPolicyDecision {
        allowed: false,
        reason,
        policy_version,
        layer,
    }
}
