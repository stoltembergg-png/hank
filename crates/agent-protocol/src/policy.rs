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
#[serde(deny_unknown_fields)]
pub struct InstructionHierarchy {
    pub layers: Vec<InstructionLayer>,
    pub max_total_bytes: usize,
}

impl InstructionHierarchy {
    pub const DEFAULT_MAX_TOTAL_BYTES: usize = 32 * 1024;

    pub fn validate(&self) -> Result<(), String> {
        if self.layers.is_empty() || self.layers.len() > 8 {
            return Err("instruction hierarchy must contain 1..=8 layers".into());
        }
        if self.max_total_bytes == 0 || self.max_total_bytes > 256 * 1024 {
            return Err("instruction hierarchy size budget is invalid".into());
        }
        let mut seen = std::collections::HashSet::new();
        let mut total = 0usize;
        for layer in &self.layers {
            if !seen.insert(layer.source) {
                return Err("instruction source appears more than once".into());
            }
            if layer.name.trim().is_empty() || layer.name.len() > 80 {
                return Err("instruction layer name is invalid".into());
            }
            if layer.source == InstructionSource::Security && layer.overridable {
                return Err("security instruction layer cannot be overridden".into());
            }
            if layer.precedence == 0 {
                return Err("instruction precedence must be positive".into());
            }
            total = total.saturating_add(layer.name.len());
        }
        if total > self.max_total_bytes {
            return Err("instruction hierarchy exceeds its size budget".into());
        }
        Ok(())
    }

    pub fn ordered_layers(&self) -> Vec<InstructionLayer> {
        let mut layers = self.layers.clone();
        layers.sort_by(|left, right| {
            right
                .precedence
                .cmp(&left.precedence)
                .then_with(|| left.source.cmp(&right.source))
        });
        layers
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstructionLayer {
    pub name: String,
    pub source: InstructionSource,
    pub precedence: u32,
    pub overridable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
            max_total_bytes: InstructionHierarchy::DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

#[cfg(test)]
mod instruction_hierarchy_tests {
    use super::*;

    #[test]
    fn default_hierarchy_has_stable_order_and_validates() {
        let hierarchy = InstructionHierarchy::default();
        hierarchy.validate().unwrap();
        let sources: Vec<_> = hierarchy
            .ordered_layers()
            .into_iter()
            .map(|layer| layer.source)
            .collect();
        assert_eq!(
            sources,
            vec![
                InstructionSource::System,
                InstructionSource::Security,
                InstructionSource::Project,
                InstructionSource::Agent,
                InstructionSource::Workflow,
                InstructionSource::Skill,
                InstructionSource::Conversation,
                InstructionSource::User,
            ]
        );
    }

    #[test]
    fn duplicate_sources_and_security_override_fail_closed() {
        let mut hierarchy = InstructionHierarchy::default();
        hierarchy.layers.push(hierarchy.layers[0].clone());
        assert!(hierarchy.validate().is_err());
        let mut hierarchy = InstructionHierarchy::default();
        hierarchy.layers[1].overridable = true;
        assert!(hierarchy.validate().is_err());
    }

    #[test]
    fn unknown_fields_and_excessive_budget_fail_closed() {
        let mut value = serde_json::to_value(InstructionHierarchy::default()).unwrap();
        value["hidden_layer"] = serde_json::json!("user");
        assert!(serde_json::from_value::<InstructionHierarchy>(value).is_err());
        let hierarchy = InstructionHierarchy {
            max_total_bytes: 1,
            ..Default::default()
        };
        assert!(hierarchy.validate().is_err());
    }
}
