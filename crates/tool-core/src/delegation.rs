//! Guarded delegation request builder. It never executes a worker/provider.

use agent_core::{AgentGroupSession, AgentGroupSessionStatus, AgentId};
use agent_protocol::{InvocationError, InvocationRequest, InvocationStatus, SessionId};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DelegationError {
    #[error("delegation target is not a session member")]
    TargetNotMember,
    #[error("delegation caller is not a session member")]
    CallerNotMember,
    #[error("delegation session is terminal")]
    Terminal,
    #[error(transparent)]
    InvalidInvocation(#[from] InvocationError),
}

pub struct DelegationTool;

impl DelegationTool {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        session: &AgentGroupSession,
        caller_id: AgentId,
        callee_id: AgentId,
        task: String,
        context_refs: Vec<String>,
        max_tokens: u64,
        invocation_id: uuid::Uuid,
    ) -> Result<InvocationRequest, DelegationError> {
        if matches!(
            session.status,
            AgentGroupSessionStatus::Cancelled | AgentGroupSessionStatus::Closed
        ) {
            return Err(DelegationError::Terminal);
        }
        if !session
            .memberships
            .iter()
            .any(|membership| membership.agent_id == caller_id)
        {
            return Err(DelegationError::CallerNotMember);
        }
        if !session
            .memberships
            .iter()
            .any(|membership| membership.agent_id == callee_id)
        {
            return Err(DelegationError::TargetNotMember);
        }
        let request = InvocationRequest {
            schema_version: agent_protocol::INVOCATION_SCHEMA_VERSION,
            invocation_id,
            project_id: session.project_id,
            group_id: session.group_id,
            session_id: SessionId::from(session.id),
            caller_id,
            callee_id,
            trace_id: session.trace_id,
            task,
            context_refs,
            max_tokens,
            depth: 0,
            status: InvocationStatus::Pending,
        };
        request.validate()?;
        Ok(request)
    }
}

#[derive(Debug, Default)]
pub struct PendingDelegationLedger {
    pub pending: HashSet<uuid::Uuid>,
}

impl PendingDelegationLedger {
    pub fn register(&mut self, request: InvocationRequest) -> bool {
        self.pending.insert(request.invocation_id)
    }

    pub fn cancel(&mut self, invocation_id: uuid::Uuid) -> bool {
        self.pending.remove(&invocation_id)
    }
}
