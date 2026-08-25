//! Provider-neutral AgentNode handoff; no provider or storage access.

use crate::provider_service::{InvocationError, InvocationRequest};
use crate::session_service::TurnInvoker;
use agent_core::ids::{AgentId, SessionId};
use std::sync::Arc;
use thiserror::Error;

const MAX_ID_BYTES: usize = 128;

#[derive(Debug, Clone)]
pub struct AgentNodeRequest {
    pub run_id: String,
    pub node_id: String,
    pub project_id: String,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub generation: u64,
    pub max_tokens: u64,
    pub invocation: InvocationRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNodeResult {
    pub run_id: String,
    pub node_id: String,
    pub session_id: SessionId,
    pub generation: u64,
    pub text: String,
    pub usage_tokens: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AgentNodeError {
    #[error("agent node identity is invalid")]
    InvalidIdentity,
    #[error("agent node request identity is unauthorized")]
    Unauthorized,
    #[error("agent node request was cancelled")]
    Cancelled,
    #[error("agent node token budget was exceeded")]
    BudgetExceeded,
    #[error("agent node result generation is stale")]
    StaleGeneration,
    #[error("agent node invocation failed")]
    InvocationFailed,
}

pub struct AgentNodeAdapter {
    invoker: Arc<dyn TurnInvoker>,
}

impl AgentNodeAdapter {
    pub fn new(invoker: Arc<dyn TurnInvoker>) -> Self {
        Self { invoker }
    }

    pub async fn execute(
        &self,
        request: AgentNodeRequest,
    ) -> Result<AgentNodeResult, AgentNodeError> {
        validate_request(&request)?;
        if request.invocation.access.cancellation.is_cancelled() {
            return Err(AgentNodeError::Cancelled);
        }
        let result = self
            .invoker
            .complete(request.invocation)
            .await
            .map_err(map_invocation_error)?;
        let usage_tokens = u64::from(result.usage.input_tokens)
            .saturating_add(u64::from(result.usage.output_tokens));
        if usage_tokens > request.max_tokens {
            return Err(AgentNodeError::BudgetExceeded);
        }
        Ok(AgentNodeResult {
            run_id: request.run_id,
            node_id: request.node_id,
            session_id: request.session_id,
            generation: request.generation,
            text: result.text,
            usage_tokens,
        })
    }

    pub fn accept_result(
        &self,
        result: &AgentNodeResult,
        expected_generation: u64,
    ) -> Result<(), AgentNodeError> {
        if result.generation != expected_generation {
            return Err(AgentNodeError::StaleGeneration);
        }
        Ok(())
    }
}

fn validate_request(request: &AgentNodeRequest) -> Result<(), AgentNodeError> {
    if !valid_id(&request.run_id)
        || !valid_id(&request.node_id)
        || request.generation == 0
        || request.max_tokens == 0
    {
        return Err(AgentNodeError::InvalidIdentity);
    }
    let normalized = &request.invocation.normalized;
    if normalized.project_id != request.project_id
        || normalized.agent_id != request.agent_id.to_string()
        || normalized.session_id.as_deref() != Some(request.session_id.to_string().as_str())
        || request.invocation.access.project_id.as_str() != request.project_id
    {
        return Err(AgentNodeError::Unauthorized);
    }
    if normalized
        .budget
        .max_tokens
        .is_some_and(|value| u64::from(value) > request.max_tokens)
    {
        return Err(AgentNodeError::BudgetExceeded);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.chars().all(|c| !c.is_control())
}

fn map_invocation_error(error: InvocationError) -> AgentNodeError {
    match error {
        InvocationError::Cancelled => AgentNodeError::Cancelled,
        InvocationError::InvalidRequest | InvocationError::Unauthorized => {
            AgentNodeError::Unauthorized
        }
        _ => AgentNodeError::InvocationFailed,
    }
}
