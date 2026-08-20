//! Políticas de orçamento, reserva de recursos e tracking determinístico de custos.

use crate::error::DomainError;
use crate::ids::{AgentId, ProjectId, SessionId, TaskId, WorkflowId};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const BUDGET_POLICY_SCHEMA_VERSION: u32 = 1;
pub const MAX_TOKENS_LIMIT: u64 = 1_000_000_000;
pub const MAX_COST_MICRO_USD_LIMIT: u64 = 100_000_000_000; // $100,000.00 in microdollars
pub const MAX_PARALLEL_INVOCATIONS_LIMIT: u32 = 64;
pub const MAX_WALL_TIME_SECONDS_LIMIT: u64 = 604_800; // 7 days

/// Período de renovação periódica do orçamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetPeriod {
    Never,
    Daily,
    Weekly,
    Monthly,
}

impl Default for ResetPeriod {
    fn default() -> Self {
        Self::Never
    }
}

/// Escopo de granularidade do orçamento.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetScope {
    Project(ProjectId),
    Agent(AgentId),
    Session(SessionId),
    Workflow(WorkflowId),
    Task(TaskId),
}

/// Limites numéricos estritos de um orçamento.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetLimits {
    pub max_tokens: u64,
    pub max_cost_micro_usd: u64,
    pub max_parallel_invocations: u32,
    pub max_wall_time_seconds: u64,
    pub reset_period: ResetPeriod,
}

impl Default for BudgetLimits {
    fn default() -> Self {
        Self {
            max_tokens: 1_000_000,
            max_cost_micro_usd: 10_000_000, // $10.00
            max_parallel_invocations: 4,
            max_wall_time_seconds: 300,
            reset_period: ResetPeriod::Never,
        }
    }
}

impl BudgetLimits {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.max_tokens == 0 || self.max_tokens > MAX_TOKENS_LIMIT {
            return Err(DomainError::Validation(
                "max_tokens out of valid range (1..=1000000000)".into(),
            ));
        }
        if self.max_cost_micro_usd == 0 || self.max_cost_micro_usd > MAX_COST_MICRO_USD_LIMIT {
            return Err(DomainError::Validation(
                "max_cost_micro_usd out of valid range (1..=100000000000)".into(),
            ));
        }
        if self.max_parallel_invocations == 0
            || self.max_parallel_invocations > MAX_PARALLEL_INVOCATIONS_LIMIT
        {
            return Err(DomainError::Validation(
                "max_parallel_invocations out of valid range (1..=64)".into(),
            ));
        }
        if self.max_wall_time_seconds == 0
            || self.max_wall_time_seconds > MAX_WALL_TIME_SECONDS_LIMIT
        {
            return Err(DomainError::Validation(
                "max_wall_time_seconds out of valid range (1..=604800)".into(),
            ));
        }
        Ok(())
    }
}

/// Identificador de reserva de budget em voo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReservationId(pub uuid::Uuid);

impl ReservationId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}


impl Default for ReservationId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveReservation {
    pub reserved_tokens: u64,
    pub reserved_cost_micro_usd: u64,
    pub created_at: DateTime<Utc>,
}

/// Conta de orçamento ativa gerenciando consumo, reserva e resets periódicos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetAccount {
    pub schema_version: u32,
    pub scope: BudgetScope,
    pub limits: BudgetLimits,
    pub used_tokens: u64,
    pub reserved_tokens: u64,
    pub used_cost_micro_usd: u64,
    pub reserved_cost_micro_usd: u64,
    pub active_invocations: u32,
    pub reservations: HashMap<ReservationId, ActiveReservation>,
    pub last_reset_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BudgetAccount {
    pub fn new(scope: BudgetScope, limits: BudgetLimits, now: DateTime<Utc>) -> Self {
        Self {
            schema_version: BUDGET_POLICY_SCHEMA_VERSION,
            scope,
            limits,
            used_tokens: 0,
            reserved_tokens: 0,
            used_cost_micro_usd: 0,
            reserved_cost_micro_usd: 0,
            active_invocations: 0,
            reservations: HashMap::new(),
            last_reset_at: now,
            updated_at: now,
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != BUDGET_POLICY_SCHEMA_VERSION {
            return Err(DomainError::Validation(
                "unsupported budget policy schema version".into(),
            ));
        }
        self.limits.validate()?;
        Ok(())
    }

    /// Executa reset periódico se o tempo decorrido atingir a política.
    pub fn reset_if_needed(&mut self, now: DateTime<Utc>) -> bool {
        let should_reset = match self.limits.reset_period {
            ResetPeriod::Never => false,
            ResetPeriod::Daily => {
                now.date_naive() > self.last_reset_at.date_naive()
            }
            ResetPeriod::Weekly => {
                let days = (now.date_naive() - self.last_reset_at.date_naive()).num_days();
                days >= 7 || (now.iso_week() != self.last_reset_at.iso_week())
            }
            ResetPeriod::Monthly => {
                now.year() > self.last_reset_at.year() || now.month() > self.last_reset_at.month()
            }
        };

        if should_reset {
            self.used_tokens = 0;
            self.used_cost_micro_usd = 0;
            self.last_reset_at = now;
            self.updated_at = now;
            true
        } else {
            false
        }
    }

    /// Verifica se há saldo disponível sem alocar.
    pub fn check_availability(
        &self,
        tokens: u64,
        cost_micro_usd: u64,
    ) -> Result<(), DomainError> {
        let total_tokens = self
            .used_tokens
            .checked_add(self.reserved_tokens)
            .and_then(|sum| sum.checked_add(tokens))
            .ok_or_else(|| DomainError::BudgetExceeded {
                budget_type: "tokens_overflow".into(),
                limit: self.limits.max_tokens.to_string(),
                used: "overflow".into(),
            })?;

        if total_tokens > self.limits.max_tokens {
            return Err(DomainError::BudgetExceeded {
                budget_type: "tokens".into(),
                limit: self.limits.max_tokens.to_string(),
                used: total_tokens.to_string(),
            });
        }

        let total_cost = self
            .used_cost_micro_usd
            .checked_add(self.reserved_cost_micro_usd)
            .and_then(|sum| sum.checked_add(cost_micro_usd))
            .ok_or_else(|| DomainError::BudgetExceeded {
                budget_type: "cost_overflow".into(),
                limit: self.limits.max_cost_micro_usd.to_string(),
                used: "overflow".into(),
            })?;

        if total_cost > self.limits.max_cost_micro_usd {
            return Err(DomainError::BudgetExceeded {
                budget_type: "cost_micro_usd".into(),
                limit: self.limits.max_cost_micro_usd.to_string(),
                used: total_cost.to_string(),
            });
        }

        Ok(())
    }

    /// Reserva budget antes de iniciar execução assíncrona.
    pub fn reserve(
        &mut self,
        tokens: u64,
        cost_micro_usd: u64,
        now: DateTime<Utc>,
    ) -> Result<ReservationId, DomainError> {
        self.reset_if_needed(now);
        self.check_availability(tokens, cost_micro_usd)?;

        if self.active_invocations >= self.limits.max_parallel_invocations {
            return Err(DomainError::BudgetExceeded {
                budget_type: "parallel_invocations".into(),
                limit: self.limits.max_parallel_invocations.to_string(),
                used: (self.active_invocations + 1).to_string(),
            });
        }

        let reservation_id = ReservationId::new();
        self.reserved_tokens = self.reserved_tokens.saturating_add(tokens);
        self.reserved_cost_micro_usd = self.reserved_cost_micro_usd.saturating_add(cost_micro_usd);
        self.active_invocations = self.active_invocations.saturating_add(1);
        self.reservations.insert(
            reservation_id,
            ActiveReservation {
                reserved_tokens: tokens,
                reserved_cost_micro_usd: cost_micro_usd,
                created_at: now,
            },
        );
        self.updated_at = now;

        Ok(reservation_id)
    }

    /// Confirma o consumo real liberando a reserva correspondente.
    pub fn commit(
        &mut self,
        reservation_id: ReservationId,
        actual_tokens: u64,
        actual_cost_micro_usd: u64,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let reservation = self
            .reservations
            .remove(&reservation_id)
            .ok_or_else(|| DomainError::NotFound("active reservation not found".into()))?;

        self.reserved_tokens = self
            .reserved_tokens
            .saturating_sub(reservation.reserved_tokens);
        self.reserved_cost_micro_usd = self
            .reserved_cost_micro_usd
            .saturating_sub(reservation.reserved_cost_micro_usd);
        self.active_invocations = self.active_invocations.saturating_sub(1);

        self.used_tokens = self.used_tokens.saturating_add(actual_tokens);
        self.used_cost_micro_usd = self
            .used_cost_micro_usd
            .saturating_add(actual_cost_micro_usd);
        self.updated_at = now;

        Ok(())
    }

    /// Devolve a reserva integralmente em caso de cancelamento ou falha.
    pub fn refund(
        &mut self,
        reservation_id: ReservationId,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let reservation = self
            .reservations
            .remove(&reservation_id)
            .ok_or_else(|| DomainError::NotFound("active reservation not found".into()))?;

        self.reserved_tokens = self
            .reserved_tokens
            .saturating_sub(reservation.reserved_tokens);
        self.reserved_cost_micro_usd = self
            .reserved_cost_micro_usd
            .saturating_sub(reservation.reserved_cost_micro_usd);
        self.active_invocations = self.active_invocations.saturating_sub(1);
        self.updated_at = now;

        Ok(())
    }

    /// Consumo direto sem etapa prévia de reserva.
    pub fn direct_consume(
        &mut self,
        tokens: u64,
        cost_micro_usd: u64,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.reset_if_needed(now);
        self.check_availability(tokens, cost_micro_usd)?;

        self.used_tokens = self.used_tokens.saturating_add(tokens);
        self.used_cost_micro_usd = self.used_cost_micro_usd.saturating_add(cost_micro_usd);
        self.updated_at = now;

        Ok(())
    }
}

/// Rastreamento hierárquico de orçamento através de múltiplos escopos (Project, Agent, Session).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct BudgetHierarchyTracker {
    pub accounts: HashMap<BudgetScope, BudgetAccount>,
}

impl BudgetHierarchyTracker {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn get_or_create(
        &mut self,
        scope: BudgetScope,
        limits: BudgetLimits,
        now: DateTime<Utc>,
    ) -> &mut BudgetAccount {
        self.accounts
            .entry(scope.clone())
            .or_insert_with(|| BudgetAccount::new(scope, limits, now))
    }

    /// Valida disponibilidade conjunta em toda a hierarquia (Project + Agent + Session).
    pub fn check_hierarchy(
        &self,
        scopes: &[BudgetScope],
        tokens: u64,
        cost_micro_usd: u64,
    ) -> Result<(), DomainError> {
        for scope in scopes {
            if let Some(account) = self.accounts.get(scope) {
                account.check_availability(tokens, cost_micro_usd)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_are_valid() {
        let limits = BudgetLimits::default();
        assert!(limits.validate().is_ok());
    }

    #[test]
    fn budget_account_roundtrip() {
        let now = Utc::now();
        let account = BudgetAccount::new(
            BudgetScope::Project(ProjectId::new()),
            BudgetLimits::default(),
            now,
        );
        let json = serde_json::to_string(&account).expect("serialize");
        let decoded: BudgetAccount = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.schema_version, BUDGET_POLICY_SCHEMA_VERSION);
        assert_eq!(decoded.used_tokens, 0);
        assert!(decoded.validate().is_ok());
    }

    #[test]
    fn reservation_commit_and_refund_lifecycle() {
        let now = Utc::now();
        let mut account = BudgetAccount::new(
            BudgetScope::Agent(AgentId::new()),
            BudgetLimits {
                max_tokens: 10_000,
                max_cost_micro_usd: 1_000_000, // $1.00
                max_parallel_invocations: 2,
                ..Default::default()
            },
            now,
        );

        // Reserva 1
        let r1 = account.reserve(4_000, 400_000, now).expect("reserve r1");
        assert_eq!(account.reserved_tokens, 4_000);
        assert_eq!(account.active_invocations, 1);

        // Reserva 2
        let r2 = account.reserve(5_000, 500_000, now).expect("reserve r2");
        assert_eq!(account.reserved_tokens, 9_000);
        assert_eq!(account.active_invocations, 2);

        // Terceira reserva excede invocações paralelas
        assert!(account.reserve(500, 50_000, now).is_err());

        // Commit da reserva 1 com consumo real menor (3000 tokens)
        account.commit(r1, 3_000, 300_000, now).expect("commit r1");
        assert_eq!(account.used_tokens, 3_000);
        assert_eq!(account.reserved_tokens, 5_000);
        assert_eq!(account.active_invocations, 1);

        // Refund da reserva 2 (cancelamento)
        account.refund(r2, now).expect("refund r2");
        assert_eq!(account.used_tokens, 3_000);
        assert_eq!(account.reserved_tokens, 0);
        assert_eq!(account.active_invocations, 0);
    }

    #[test]
    fn exceeded_tokens_or_cost_fails_closed() {
        let now = Utc::now();
        let mut account = BudgetAccount::new(
            BudgetScope::Project(ProjectId::new()),
            BudgetLimits {
                max_tokens: 1_000,
                max_cost_micro_usd: 100_000,
                ..Default::default()
            },
            now,
        );

        assert!(account.direct_consume(1_001, 10_000, now).is_err());
        assert!(account.direct_consume(500, 100_001, now).is_err());
        assert!(account.direct_consume(600, 50_000, now).is_ok());
        // Segundo consumo estoura limite acumulado
        assert!(account.direct_consume(500, 10_000, now).is_err());
    }

    #[test]
    fn periodic_reset_clears_used_accumulators() {
        let past = Utc::now() - chrono::Duration::days(2);
        let now = Utc::now();
        let mut account = BudgetAccount::new(
            BudgetScope::Project(ProjectId::new()),
            BudgetLimits {
                max_tokens: 1_000,
                max_cost_micro_usd: 100_000,
                reset_period: ResetPeriod::Daily,
                ..Default::default()
            },
            past,
        );

        account.direct_consume(900, 90_000, past).expect("consume past");
        assert_eq!(account.used_tokens, 900);

        // No momento atual (2 dias depois), consome novamente e dispara reset
        account.direct_consume(500, 50_000, now).expect("consume after reset");
        assert_eq!(account.used_tokens, 500);
    }

    #[test]
    fn hierarchy_tracker_evaluates_multi_scope_limits() {
        let now = Utc::now();
        let mut tracker = BudgetHierarchyTracker::new();

        let project_id = ProjectId::new();
        let agent_id = AgentId::new();
        let project_scope = BudgetScope::Project(project_id);
        let agent_scope = BudgetScope::Agent(agent_id);

        tracker.get_or_create(
            project_scope.clone(),
            BudgetLimits {
                max_tokens: 10_000,
                ..Default::default()
            },
            now,
        );

        tracker.get_or_create(
            agent_scope.clone(),
            BudgetLimits {
                max_tokens: 2_000,
                ..Default::default()
            },
            now,
        );

        // 1500 cabe em ambos
        assert!(tracker
            .check_hierarchy(&[project_scope.clone(), agent_scope.clone()], 1_500, 10_000)
            .is_ok());

        // 3000 cabe no projeto, mas excede limite do agente
        assert!(tracker
            .check_hierarchy(&[project_scope, agent_scope], 3_000, 10_000)
            .is_err());
    }
}
