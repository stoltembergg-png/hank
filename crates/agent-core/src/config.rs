use crate::agent::Personality;
use crate::error::DomainError;
use agent_protocol::{AgentId, AgentPolicyConfig, ProjectId};
use serde::{Deserialize, Serialize};

pub const AGENT_CONFIG_SCHEMA_VERSION: u32 = 1;
const MAX_INSTRUCTION_REFS: usize = 32;
const MAX_REF_LENGTH: usize = 160;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub personality: Personality,
    pub instruction_refs: Vec<String>,
    pub policy: AgentPolicyConfig,
}

impl AgentConfig {
    pub fn defaults(project_id: ProjectId, agent_id: AgentId) -> Self {
        Self {
            schema_version: AGENT_CONFIG_SCHEMA_VERSION,
            project_id,
            agent_id,
            personality: Personality::default(),
            instruction_refs: Vec::new(),
            policy: AgentPolicyConfig::default(),
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != AGENT_CONFIG_SCHEMA_VERSION {
            return Err(DomainError::Validation(
                "unsupported agent config schema".into(),
            ));
        }
        if self.instruction_refs.len() > MAX_INSTRUCTION_REFS
            || self.instruction_refs.iter().any(|reference| {
                reference.trim().is_empty()
                    || reference.len() > MAX_REF_LENGTH
                    || reference.contains('\n')
            })
        {
            return Err(DomainError::Validation(
                "agent instruction refs exceed limits".into(),
            ));
        }
        if self.personality.name.len() > 120 || self.personality.traits.len() > 32 {
            return Err(DomainError::Validation(
                "agent personality exceeds limits".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_deterministic_and_valid() {
        let project = ProjectId::new();
        let agent = AgentId::new();
        let first = AgentConfig::defaults(project, agent);
        let second = AgentConfig::defaults(project, agent);
        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            serde_json::to_value(&second).unwrap()
        );
        first.validate().unwrap();
    }

    #[test]
    fn config_roundtrips_and_rejects_unknown_or_oversized_fields() {
        let mut config = AgentConfig::defaults(ProjectId::new(), AgentId::new());
        config.instruction_refs.push("policy://safe".into());
        config.validate().unwrap();
        let encoded = serde_json::to_string(&config).unwrap();
        assert_eq!(
            serde_json::to_value(serde_json::from_str::<AgentConfig>(&encoded).unwrap()).unwrap(),
            serde_json::to_value(&config).unwrap()
        );
        assert!(serde_json::from_str::<AgentConfig>(&format!("{encoded} ")).is_ok());
        config.instruction_refs = vec!["x".repeat(MAX_REF_LENGTH + 1)];
        assert!(config.validate().is_err());
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let mut value =
            serde_json::to_value(AgentConfig::defaults(ProjectId::new(), AgentId::new())).unwrap();
        value["provider"] = serde_json::json!("openai");
        assert!(serde_json::from_value::<AgentConfig>(value).is_err());
    }
}
