use agent_core::ids::{AgentId, ProjectId, SessionId};
use thiserror::Error;

const MAX_ID: usize = 128;
const MAX_TOKENS: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDispatchInput {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub job_id: String,
    pub run_id: String,
    pub autonomy_allowed: bool,
    pub budget_remaining: u64,
    pub max_tokens: u64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDispatchRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub job_id: String,
    pub run_id: String,
    pub idempotency_key: String,
    pub max_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AgentSchedulerError {
    #[error("agent scheduler identity is invalid")]
    InvalidIdentity,
    #[error("agent scheduler policy denied autonomy")]
    AutonomyDenied,
    #[error("agent scheduler budget exhausted")]
    BudgetExhausted,
    #[error("agent scheduler request was cancelled")]
    Cancelled,
}

impl AgentDispatchRequest {
    pub fn prepare(input: AgentDispatchInput) -> Result<Self, AgentSchedulerError> {
        for value in [&input.job_id, &input.run_id] {
            if value.is_empty() || value.len() > MAX_ID || value.chars().any(char::is_control) {
                return Err(AgentSchedulerError::InvalidIdentity);
            }
        }
        if input.cancelled {
            return Err(AgentSchedulerError::Cancelled);
        }
        if !input.autonomy_allowed {
            return Err(AgentSchedulerError::AutonomyDenied);
        }
        if input.budget_remaining == 0
            || input.max_tokens == 0
            || input.max_tokens > MAX_TOKENS
            || input.max_tokens > input.budget_remaining
        {
            return Err(AgentSchedulerError::BudgetExhausted);
        }
        Ok(Self {
            project_id: input.project_id,
            agent_id: input.agent_id,
            session_id: input.session_id,
            job_id: input.job_id,
            run_id: input.run_id.clone(),
            idempotency_key: format!("scheduler:agent:{}:{}", input.project_id, input.run_id),
            max_tokens: input.max_tokens,
        })
    }
}
