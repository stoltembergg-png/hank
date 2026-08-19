//! Políticas de agente, modelo, orçamento e autonomia.
//!
//! Define as políticas que governam comportamento de agentes,
//! limites de execução e decisões de autorização.

use crate::capability::{Capability, CapabilitySet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Política de modelo do agente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelPolicy {
    pub provider: String,
    pub model: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub fallback: Option<Box<ModelPolicy>>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// Política de ferramentas do agente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolPolicy {
    pub allowed: CapabilitySet,
    pub denied: CapabilitySet,
    pub require_approval: CapabilitySet,
    pub default_action: ToolDefaultAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolDefaultAction {
    #[default]
    Allow,
    Ask,
    Deny,
}

/// Política de orçamento do agente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPolicy {
    pub max_tokens_per_request: Option<u64>,
    pub max_tokens_per_session: Option<u64>,
    pub max_cost_usd_per_session: Option<f64>,
    pub max_cost_usd_per_project: Option<f64>,
    pub max_parallel_invocations: Option<u32>,
    pub max_delegation_depth: Option<u32>,
    pub max_fanout: Option<u32>,
    pub max_wall_time_seconds: Option<u64>,
}

impl Default for BudgetPolicy {
    fn default() -> Self {
        Self {
            max_tokens_per_request: Some(8192),
            max_tokens_per_session: Some(100_000),
            max_cost_usd_per_session: Some(10.0),
            max_cost_usd_per_project: Some(100.0),
            max_parallel_invocations: Some(4),
            max_delegation_depth: Some(3),
            max_fanout: Some(10),
            max_wall_time_seconds: Some(300),
        }
    }
}

/// Política de autonomia do agente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutonomyPolicy {
    pub level: AutonomyLevel,
    pub can_spawn_agents: bool,
    pub can_create_workflows: bool,
    pub can_modify_skills: bool,
    pub can_access_external_apis: bool,
    pub requires_human_approval_for: CapabilitySet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    #[default]
    None,
    Assisted,
    SemiAutonomous,
    Autonomous,
    FullyAutonomous,
}

/// Decisão de política (permit/deny com razão)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: String,
    pub capability: Capability,
    pub fingerprint: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reviewer_id: Option<String>,
}

impl PolicyDecision {
    pub fn allow(capability: Capability, fingerprint: String) -> Self {
        Self {
            allowed: true,
            reason: "Allowed by policy".to_string(),
            capability,
            fingerprint,
            expires_at: None,
            reviewer_id: None,
        }
    }

    pub fn deny(capability: Capability, fingerprint: String, reason: String) -> Self {
        Self {
            allowed: false,
            reason,
            capability,
            fingerprint,
            expires_at: None,
            reviewer_id: None,
        }
    }
}

/// Configuração completa de políticas de um agente
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPolicyConfig {
    pub model: ModelPolicy,
    pub tools: ToolPolicy,
    pub budget: BudgetPolicy,
    pub autonomy: AutonomyPolicy,
    pub instruction_hierarchy: InstructionHierarchy,
}

/// Hierarquia de instruções (ordem de precedência)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionHierarchy {
    pub layers: Vec<InstructionLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionLayer {
    pub name: String,
    pub source: InstructionSource,
    pub precedence: u32,
    pub overridable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionSource {
    System,
    Security,
    Project,
    Agent,
    Workflow,
    Skill,
    Conversation,
    User,
}

impl Default for InstructionHierarchy {
    fn default() -> Self {
        Self {
            layers: vec![
                InstructionLayer {
                    name: "system".into(),
                    source: InstructionSource::System,
                    precedence: 100,
                    overridable: false,
                },
                InstructionLayer {
                    name: "security".into(),
                    source: InstructionSource::Security,
                    precedence: 90,
                    overridable: false,
                },
                InstructionLayer {
                    name: "project".into(),
                    source: InstructionSource::Project,
                    precedence: 80,
                    overridable: true,
                },
                InstructionLayer {
                    name: "agent".into(),
                    source: InstructionSource::Agent,
                    precedence: 70,
                    overridable: true,
                },
                InstructionLayer {
                    name: "workflow".into(),
                    source: InstructionSource::Workflow,
                    precedence: 60,
                    overridable: true,
                },
                InstructionLayer {
                    name: "skill".into(),
                    source: InstructionSource::Skill,
                    precedence: 50,
                    overridable: true,
                },
                InstructionLayer {
                    name: "conversation".into(),
                    source: InstructionSource::Conversation,
                    precedence: 40,
                    overridable: true,
                },
                InstructionLayer {
                    name: "user".into(),
                    source: InstructionSource::User,
                    precedence: 30,
                    overridable: true,
                },
            ],
        }
    }
}
