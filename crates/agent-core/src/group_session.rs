//! Bounded, non-executable session state for an AgentGroup.

use crate::{AgentGroup, AgentGroupMembership, BudgetLimits, ProjectId, TraceId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const GROUP_SESSION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentGroupSessionStatus {
    Created,
    Active,
    Cancelled,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentGroupSessionError {
    #[error("session identity or group scope is invalid")]
    InvalidScope,
    #[error("session lifecycle state is terminal")]
    Terminal,
    #[error("session lifecycle transition is invalid")]
    InvalidTransition,
    #[error("session round limit reached")]
    RoundLimit,
    #[error("session budget limit reached")]
    BudgetLimit,
    #[error("session round is not active")]
    RoundNotActive,
    #[error("session group membership snapshot is invalid")]
    InvalidMembershipSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGroupSession {
    pub schema_version: u32,
    pub id: uuid::Uuid,
    pub project_id: ProjectId,
    pub group_id: uuid::Uuid,
    pub trace_id: TraceId,
    pub memberships: Vec<AgentGroupMembership>,
    pub status: AgentGroupSessionStatus,
    pub max_rounds: u32,
    pub current_round: u32,
    pub max_depth: u16,
    pub budget: BudgetLimits,
    pub used_tokens: u64,
    pub context_refs: Vec<String>,
}

impl AgentGroupSession {
    pub fn from_group(group: &AgentGroup) -> Result<Self, AgentGroupSessionError> {
        group
            .validate()
            .map_err(|_| AgentGroupSessionError::InvalidMembershipSnapshot)?;
        Ok(Self {
            schema_version: GROUP_SESSION_SCHEMA_VERSION,
            id: uuid::Uuid::new_v4(),
            project_id: group.project_id,
            group_id: group.id,
            trace_id: group.trace_id,
            memberships: group.memberships.clone(),
            status: AgentGroupSessionStatus::Created,
            max_rounds: group.max_rounds,
            current_round: 0,
            max_depth: group.max_depth,
            budget: group.budget.clone(),
            used_tokens: 0,
            context_refs: group.context_refs.clone(),
        })
    }

    pub fn start(&mut self) -> Result<(), AgentGroupSessionError> {
        if self.status != AgentGroupSessionStatus::Created {
            return Err(AgentGroupSessionError::InvalidTransition);
        }
        self.status = AgentGroupSessionStatus::Active;
        Ok(())
    }

    pub fn begin_round(&mut self) -> Result<(), AgentGroupSessionError> {
        if self.status != AgentGroupSessionStatus::Active {
            return Err(AgentGroupSessionError::Terminal);
        }
        if self.current_round >= self.max_rounds {
            return Err(AgentGroupSessionError::RoundLimit);
        }
        if self.used_tokens >= self.budget.max_tokens {
            return Err(AgentGroupSessionError::BudgetLimit);
        }
        self.current_round += 1;
        Ok(())
    }

    pub fn finish_round(&mut self, tokens: u64) -> Result<(), AgentGroupSessionError> {
        if self.status != AgentGroupSessionStatus::Active || self.current_round == 0 {
            return Err(AgentGroupSessionError::RoundNotActive);
        }
        self.used_tokens = self.used_tokens.saturating_add(tokens);
        if self.used_tokens > self.budget.max_tokens {
            return Err(AgentGroupSessionError::BudgetLimit);
        }
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<bool, AgentGroupSessionError> {
        if self.status == AgentGroupSessionStatus::Cancelled {
            return Ok(false);
        }
        if matches!(self.status, AgentGroupSessionStatus::Closed) {
            return Err(AgentGroupSessionError::Terminal);
        }
        self.status = AgentGroupSessionStatus::Cancelled;
        Ok(true)
    }

    pub fn close(&mut self) -> Result<(), AgentGroupSessionError> {
        if self.status == AgentGroupSessionStatus::Closed {
            return Ok(());
        }
        if matches!(self.status, AgentGroupSessionStatus::Cancelled) {
            return Err(AgentGroupSessionError::Terminal);
        }
        if self.status != AgentGroupSessionStatus::Active {
            return Err(AgentGroupSessionError::InvalidTransition);
        }
        self.status = AgentGroupSessionStatus::Closed;
        Ok(())
    }
}
