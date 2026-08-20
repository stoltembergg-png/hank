use crate::error::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const AUTONOMY_POLICY_SCHEMA_VERSION: u32 = 1;
pub const MAX_CONSECUTIVE_AUTONOMOUS_STEPS_LIMIT: u32 = 1000;
pub const MAX_APPROVER_ID_LEN: usize = 128;
pub const MAX_REASON_LEN: usize = 256;

/// Níveis formais de autonomia do agente (L0–L4).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    /// L0: Sugestão e apenas leitura. Qualquer execução requer aprovação.
    L0None,
    /// L1: Assistido. Apenas leituras e ferramentas seguras sem efeito colateral.
    #[default]
    L1Assisted,
    /// L2: Semi-autônomo. Executa tarefas delimitadas no escopo do projeto.
    L2SemiAutonomous,
    /// L3: Autônomo. Orquestra subagentes e workflows sob restrições do projeto.
    L3Autonomous,
    /// L4: Totalmente autônomo dentro dos limites estritos de sandbox e budget.
    L4FullyAutonomous,
}

/// Tipos de operações controladas por autonomia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyOperation {
    ReadData,
    ExecuteSafeTool,
    ExecuteStatefulTool,
    SpawnSubAgent,
    CreateWorkflow,
    ModifySkill,
    AccessExternalNetwork,
    ModifySystemConfig,
}

/// Decisão de execução de uma operação sob a política de autonomia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyDecision {
    Allow,
    RequireHumanApproval,
    Deny,
}

/// Metadados de autorização explícita para escalação de autonomia.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutonomyTransitionApproval {
    pub approver_id: String,
    pub reason: String,
    pub authorized_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Política de autonomia de agente garantindo restrições determinísticas por nível.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutonomyPolicy {
    pub schema_version: u32,
    pub level: AutonomyLevel,
    pub allow_subagents: bool,
    pub allow_workflow_creation: bool,
    pub allow_skill_modification: bool,
    pub allow_network_access: bool,
    pub max_consecutive_autonomous_steps: u32,
}

impl Default for AutonomyPolicy {
    fn default() -> Self {
        Self::defaults_for_level(AutonomyLevel::L1Assisted)
    }
}

impl AutonomyPolicy {
    pub fn defaults_for_level(level: AutonomyLevel) -> Self {
        match level {
            AutonomyLevel::L0None => Self {
                schema_version: AUTONOMY_POLICY_SCHEMA_VERSION,
                level,
                allow_subagents: false,
                allow_workflow_creation: false,
                allow_skill_modification: false,
                allow_network_access: false,
                max_consecutive_autonomous_steps: 1,
            },
            AutonomyLevel::L1Assisted => Self {
                schema_version: AUTONOMY_POLICY_SCHEMA_VERSION,
                level,
                allow_subagents: false,
                allow_workflow_creation: false,
                allow_skill_modification: false,
                allow_network_access: false,
                max_consecutive_autonomous_steps: 10,
            },
            AutonomyLevel::L2SemiAutonomous => Self {
                schema_version: AUTONOMY_POLICY_SCHEMA_VERSION,
                level,
                allow_subagents: true,
                allow_workflow_creation: true,
                allow_skill_modification: false,
                allow_network_access: false,
                max_consecutive_autonomous_steps: 50,
            },
            AutonomyLevel::L3Autonomous => Self {
                schema_version: AUTONOMY_POLICY_SCHEMA_VERSION,
                level,
                allow_subagents: true,
                allow_workflow_creation: true,
                allow_skill_modification: true,
                allow_network_access: true,
                max_consecutive_autonomous_steps: 200,
            },
            AutonomyLevel::L4FullyAutonomous => Self {
                schema_version: AUTONOMY_POLICY_SCHEMA_VERSION,
                level,
                allow_subagents: true,
                allow_workflow_creation: true,
                allow_skill_modification: true,
                allow_network_access: true,
                max_consecutive_autonomous_steps: 1000,
            },
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != AUTONOMY_POLICY_SCHEMA_VERSION {
            return Err(DomainError::Validation(
                "unsupported autonomy policy schema version".into(),
            ));
        }

        if self.max_consecutive_autonomous_steps == 0
            || self.max_consecutive_autonomous_steps > MAX_CONSECUTIVE_AUTONOMOUS_STEPS_LIMIT
        {
            return Err(DomainError::Validation(
                "max consecutive autonomous steps out of bounds (1..=1000)".into(),
            ));
        }

        // Invariante de integridade: níveis inferiores não podem habilitar capacidades proibidas
        match self.level {
            AutonomyLevel::L0None => {
                if self.allow_subagents
                    || self.allow_workflow_creation
                    || self.allow_skill_modification
                    || self.allow_network_access
                    || self.max_consecutive_autonomous_steps > 1
                {
                    return Err(DomainError::Validation(
                        "L0 policy cannot enable autonomous execution flags or multi-step execution".into(),
                    ));
                }
            }
            AutonomyLevel::L1Assisted => {
                if self.allow_subagents
                    || self.allow_workflow_creation
                    || self.allow_skill_modification
                    || self.allow_network_access
                {
                    return Err(DomainError::Validation(
                        "L1 policy cannot enable advanced autonomous flags".into(),
                    ));
                }
            }
            AutonomyLevel::L2SemiAutonomous => {
                if self.allow_skill_modification || self.allow_network_access {
                    return Err(DomainError::Validation(
                        "L2 policy cannot enable skill modification or network access without escalation".into(),
                    ));
                }
            }
            AutonomyLevel::L3Autonomous | AutonomyLevel::L4FullyAutonomous => {}
        }

        Ok(())
    }

    /// Avalia a decisão de uma operação para o nível atual.
    pub fn evaluate(&self, op: AutonomyOperation) -> AutonomyDecision {
        match op {
            AutonomyOperation::ReadData => AutonomyDecision::Allow,
            AutonomyOperation::ExecuteSafeTool => match self.level {
                AutonomyLevel::L0None => AutonomyDecision::RequireHumanApproval,
                _ => AutonomyDecision::Allow,
            },
            AutonomyOperation::ExecuteStatefulTool => match self.level {
                AutonomyLevel::L0None | AutonomyLevel::L1Assisted => {
                    AutonomyDecision::RequireHumanApproval
                }
                _ => AutonomyDecision::Allow,
            },
            AutonomyOperation::SpawnSubAgent => {
                if self.allow_subagents {
                    AutonomyDecision::Allow
                } else if self.level >= AutonomyLevel::L1Assisted {
                    AutonomyDecision::RequireHumanApproval
                } else {
                    AutonomyDecision::Deny
                }
            }
            AutonomyOperation::CreateWorkflow => {
                if self.allow_workflow_creation {
                    AutonomyDecision::Allow
                } else if self.level >= AutonomyLevel::L1Assisted {
                    AutonomyDecision::RequireHumanApproval
                } else {
                    AutonomyDecision::Deny
                }
            }
            AutonomyOperation::ModifySkill => {
                if self.allow_skill_modification {
                    AutonomyDecision::Allow
                } else if self.level >= AutonomyLevel::L2SemiAutonomous {
                    AutonomyDecision::RequireHumanApproval
                } else {
                    AutonomyDecision::Deny
                }
            }
            AutonomyOperation::AccessExternalNetwork => {
                if self.allow_network_access {
                    AutonomyDecision::Allow
                } else if self.level >= AutonomyLevel::L2SemiAutonomous {
                    AutonomyDecision::RequireHumanApproval
                } else {
                    AutonomyDecision::Deny
                }
            }
            // Modificações no sistema / autodesenvolvimento nunca são totalmente automáticas
            AutonomyOperation::ModifySystemConfig => AutonomyDecision::RequireHumanApproval,
        }
    }

    /// Valida transição entre níveis de autonomia.
    ///
    /// Regra: Redução de nível (downgrade) é sempre permitida (reversibilidade).
    /// Elevação de nível (escalação) requer aprovação humana explícita válida e não expirada.
    pub fn validate_transition(
        &self,
        target_level: AutonomyLevel,
        approval: Option<&AutonomyTransitionApproval>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        // Mesma política: no-op válido
        if target_level == self.level {
            return Ok(());
        }

        // Downgrade / reversibilidade é sempre livre e segura
        if target_level < self.level {
            return Ok(());
        }

        // Escalação exige aprovação formal
        let approval = approval.ok_or_else(|| DomainError::PermissionDenied {
            capability: "autonomy:escalation".into(),
            reason: "autonomy escalation requires explicit human approval".into(),
        })?;

        if approval.approver_id.trim().is_empty()
            || approval.approver_id.len() > MAX_APPROVER_ID_LEN
        {
            return Err(DomainError::Validation(
                "approver_id is empty or exceeds limit".into(),
            ));
        }

        if approval.reason.trim().is_empty() || approval.reason.len() > MAX_REASON_LEN {
            return Err(DomainError::Validation(
                "approval reason is empty or exceeds limit".into(),
            ));
        }

        if let Some(expiry) = approval.expires_at {
            if expiry <= now {
                return Err(DomainError::PermissionDenied {
                    capability: "autonomy:escalation".into(),
                    reason: "autonomy escalation approval has expired".into(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_for_all_levels_are_valid() {
        let levels = [
            AutonomyLevel::L0None,
            AutonomyLevel::L1Assisted,
            AutonomyLevel::L2SemiAutonomous,
            AutonomyLevel::L3Autonomous,
            AutonomyLevel::L4FullyAutonomous,
        ];
        for level in levels {
            let policy = AutonomyPolicy::defaults_for_level(level);
            assert!(policy.validate().is_ok());
        }
    }

    #[test]
    fn autonomy_serde_roundtrip() {
        let policy = AutonomyPolicy::defaults_for_level(AutonomyLevel::L2SemiAutonomous);
        let json = serde_json::to_string(&policy).expect("serialize");
        let decoded: AutonomyPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.level, AutonomyLevel::L2SemiAutonomous);
        assert!(decoded.allow_subagents);
        assert!(!decoded.allow_network_access);
        assert!(decoded.validate().is_ok());
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let mut value = serde_json::to_value(AutonomyPolicy::defaults_for_level(
            AutonomyLevel::L1Assisted,
        ))
        .unwrap();
        value["self_escalate"] = serde_json::json!(true);
        assert!(serde_json::from_value::<AutonomyPolicy>(value).is_err());
    }

    #[test]
    fn invalid_level_flag_combination_fails_validation() {
        let mut policy = AutonomyPolicy::defaults_for_level(AutonomyLevel::L0None);
        policy.allow_subagents = true;
        assert!(policy.validate().is_err());

        let mut policy = AutonomyPolicy::defaults_for_level(AutonomyLevel::L1Assisted);
        policy.allow_network_access = true;
        assert!(policy.validate().is_err());

        let mut policy = AutonomyPolicy::defaults_for_level(AutonomyLevel::L2SemiAutonomous);
        policy.allow_skill_modification = true;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn evaluation_matrix_satisfies_level_contracts() {
        let l0 = AutonomyPolicy::defaults_for_level(AutonomyLevel::L0None);
        assert_eq!(
            l0.evaluate(AutonomyOperation::ReadData),
            AutonomyDecision::Allow
        );
        assert_eq!(
            l0.evaluate(AutonomyOperation::ExecuteSafeTool),
            AutonomyDecision::RequireHumanApproval
        );
        assert_eq!(
            l0.evaluate(AutonomyOperation::SpawnSubAgent),
            AutonomyDecision::Deny
        );

        let l1 = AutonomyPolicy::defaults_for_level(AutonomyLevel::L1Assisted);
        assert_eq!(
            l1.evaluate(AutonomyOperation::ReadData),
            AutonomyDecision::Allow
        );
        assert_eq!(
            l1.evaluate(AutonomyOperation::ExecuteSafeTool),
            AutonomyDecision::Allow
        );
        assert_eq!(
            l1.evaluate(AutonomyOperation::ExecuteStatefulTool),
            AutonomyDecision::RequireHumanApproval
        );

        let l2 = AutonomyPolicy::defaults_for_level(AutonomyLevel::L2SemiAutonomous);
        assert_eq!(
            l2.evaluate(AutonomyOperation::ExecuteStatefulTool),
            AutonomyDecision::Allow
        );
        assert_eq!(
            l2.evaluate(AutonomyOperation::SpawnSubAgent),
            AutonomyDecision::Allow
        );
        assert_eq!(
            l2.evaluate(AutonomyOperation::ModifySkill),
            AutonomyDecision::RequireHumanApproval
        );

        let l4 = AutonomyPolicy::defaults_for_level(AutonomyLevel::L4FullyAutonomous);
        assert_eq!(
            l4.evaluate(AutonomyOperation::ModifySystemConfig),
            AutonomyDecision::RequireHumanApproval
        );
    }

    #[test]
    fn transition_downgrade_is_always_permitted() {
        let now = Utc::now();
        let l3 = AutonomyPolicy::defaults_for_level(AutonomyLevel::L3Autonomous);
        assert!(l3
            .validate_transition(AutonomyLevel::L1Assisted, None, now)
            .is_ok());
        assert!(l3
            .validate_transition(AutonomyLevel::L0None, None, now)
            .is_ok());
    }

    #[test]
    fn transition_escalation_requires_approval_and_rejects_expired() {
        let now = Utc::now();
        let l1 = AutonomyPolicy::defaults_for_level(AutonomyLevel::L1Assisted);

        // Sem aprovação: erro
        assert!(l1
            .validate_transition(AutonomyLevel::L2SemiAutonomous, None, now)
            .is_err());

        // Com aprovação válida
        let valid_approval = AutonomyTransitionApproval {
            approver_id: "sec-admin-01".into(),
            reason: "Batch data extraction task".into(),
            authorized_at: now,
            expires_at: Some(now + chrono::Duration::hours(2)),
        };
        assert!(l1
            .validate_transition(AutonomyLevel::L2SemiAutonomous, Some(&valid_approval), now)
            .is_ok());

        // Com aprovação expirada
        let expired_approval = AutonomyTransitionApproval {
            approver_id: "sec-admin-01".into(),
            reason: "Expired task".into(),
            authorized_at: now - chrono::Duration::hours(2),
            expires_at: Some(now - chrono::Duration::minutes(5)),
        };
        assert!(l1
            .validate_transition(
                AutonomyLevel::L2SemiAutonomous,
                Some(&expired_approval),
                now
            )
            .is_err());
    }
}
