use agent_protocol::{Action, Capability, Resource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const TOOL_PERMISSION_SCHEMA_VERSION: u32 = 1;
const MAX_RULES: usize = 128;
const MAX_SCOPE_LENGTH: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionEffect {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    Project,
    Agent,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolPermissionRule {
    pub capability: Capability,
    pub effect: PermissionEffect,
    pub scope: PermissionScope,
    pub scope_id: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolPermissionPolicy {
    pub schema_version: u32,
    pub default_effect: PermissionEffect,
    pub rules: Vec<ToolPermissionRule>,
}

impl Default for ToolPermissionPolicy {
    fn default() -> Self {
        Self {
            schema_version: TOOL_PERMISSION_SCHEMA_VERSION,
            default_effect: PermissionEffect::Deny,
            rules: Vec::new(),
        }
    }
}

impl ToolPermissionPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != TOOL_PERMISSION_SCHEMA_VERSION {
            return Err("unsupported tool permission schema".into());
        }
        if self.default_effect != PermissionEffect::Deny {
            return Err("tool permission default must deny".into());
        }
        let mut keys = HashSet::new();
        if self.rules.len() > MAX_RULES {
            return Err("too many tool permission rules".into());
        }
        for rule in &self.rules {
            if rule.scope_id.trim().is_empty()
                || rule.scope_id.len() > MAX_SCOPE_LENGTH
                || rule.scope_id.contains('*')
                || rule.scope_id.contains('\n')
            {
                return Err("invalid permission scope".into());
            }
            if is_privileged_wildcard(&rule.capability) {
                return Err("privileged wildcard capability is forbidden".into());
            }
            let key = (rule.capability.clone(), rule.scope, rule.scope_id.clone());
            if !keys.insert(key) {
                return Err("conflicting duplicate permission rule".into());
            }
        }
        Ok(())
    }

    pub fn effect_for(
        &self,
        capability: &Capability,
        scope: PermissionScope,
        scope_id: &str,
    ) -> PermissionEffect {
        self.rules
            .iter()
            .find(|rule| {
                &rule.capability == capability
                    && rule.scope == scope
                    && rule.scope_id == scope_id
                    && rule.expires_at.is_none_or(|expiry| expiry > Utc::now())
            })
            .map(|rule| rule.effect)
            .unwrap_or(self.default_effect)
    }
}

fn is_privileged_wildcard(capability: &Capability) -> bool {
    matches!(
        capability.resource,
        Resource::File | Resource::Process | Resource::Network
    ) && (capability.scope.is_none() || capability.scope.as_deref() == Some("*"))
        && matches!(
            capability.action,
            Action::Execute | Action::Invoke | Action::Delete
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(effect: PermissionEffect) -> ToolPermissionRule {
        ToolPermissionRule {
            capability: Capability::new(Resource::Tool, Action::Invoke),
            effect,
            scope: PermissionScope::Project,
            scope_id: "proj-1".into(),
            expires_at: None,
        }
    }

    #[test]
    fn default_policy_denies_and_explicit_rules_are_deterministic() {
        let policy = ToolPermissionPolicy::default();
        policy.validate().unwrap();
        let capability = Capability::new(Resource::Tool, Action::Invoke);
        assert_eq!(
            policy.effect_for(&capability, PermissionScope::Project, "proj-1"),
            PermissionEffect::Deny
        );
        let policy = ToolPermissionPolicy {
            rules: vec![rule(PermissionEffect::Ask)],
            ..Default::default()
        };
        policy.validate().unwrap();
        assert_eq!(
            policy.effect_for(&capability, PermissionScope::Project, "proj-1"),
            PermissionEffect::Ask
        );
    }

    #[test]
    fn malformed_conflicting_and_privileged_rules_fail_closed() {
        let mut policy = ToolPermissionPolicy {
            rules: vec![rule(PermissionEffect::Allow), rule(PermissionEffect::Deny)],
            ..Default::default()
        };
        assert!(policy.validate().is_err());
        policy.rules = vec![ToolPermissionRule {
            capability: Capability::new(Resource::Process, Action::Execute),
            ..rule(PermissionEffect::Allow)
        }];
        assert!(policy.validate().is_err());
        let mut value = serde_json::to_value(ToolPermissionPolicy::default()).unwrap();
        value["provider_key"] = serde_json::json!("secret");
        assert!(serde_json::from_value::<ToolPermissionPolicy>(value).is_err());
    }
}
