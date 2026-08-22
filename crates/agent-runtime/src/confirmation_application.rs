//! Application boundary for confirmation requests and grants.
//!
//! The boundary transports only the bounded approval artifacts from
//! `tool-core`; raw schemas and arguments never cross this service.

use std::sync::Arc;
use uuid::Uuid;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tool_core::{ApprovalGrant, ApprovalRequest, ConfirmationError, ConfirmationLedger};

/// Input for the human approval command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveConfirmationInput {
    pub request_id: Uuid,
    pub actor_id: String,
    pub now_ms: u64,
}

/// Stable application error for confirmation commands.
#[derive(Debug, Error)]
pub enum ConfirmationApplicationError {
    #[error("confirmation command failed: {0}")]
    Ledger(#[from] ConfirmationError),
}

/// Application-facing facade over the bounded confirmation ledger.
#[derive(Debug, Clone)]
pub struct ConfirmationApplicationService {
    ledger: Arc<ConfirmationLedger>,
}

impl ConfirmationApplicationService {
    pub fn new(ledger: ConfirmationLedger) -> Self {
        Self {
            ledger: Arc::new(ledger),
        }
    }

    pub fn with_ledger(ledger: Arc<ConfirmationLedger>) -> Self {
        Self { ledger }
    }

    /// Registers and returns the redacted request artifact for the UI/API.
    pub fn submit(
        &self,
        request: ApprovalRequest,
    ) -> Result<ApprovalRequest, ConfirmationApplicationError> {
        self.ledger.register(request.clone())?;
        Ok(request)
    }

    /// Approves an already submitted request for the presenting actor.
    pub fn approve(
        &self,
        input: ApproveConfirmationInput,
    ) -> Result<ApprovalGrant, ConfirmationApplicationError> {
        self.ledger
            .approve(input.request_id, &input.actor_id, input.now_ms)
            .map_err(Into::into)
    }

    /// Revokes a request or its bounded ask-once scope.
    pub fn revoke(&self, request: &ApprovalRequest) -> Result<(), ConfirmationApplicationError> {
        self.ledger.revoke(request).map_err(Into::into)
    }

    /// Performs the final fail-closed ledger authorization before execution.
    pub fn authorize(
        &self,
        request: &ApprovalRequest,
        grant: &ApprovalGrant,
        actor_id: &str,
        now_ms: u64,
    ) -> Result<(), ConfirmationApplicationError> {
        self.ledger
            .authorize(request, grant, actor_id, now_ms)
            .map_err(Into::into)
    }
}
