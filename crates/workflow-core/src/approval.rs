//! Bounded, fail-closed ApprovalNode ledger.

use std::collections::BTreeMap;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

const MAX_ID_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBinding {
    pub project_id: String,
    pub workflow_id: String,
    pub run_id: String,
    pub node_id: String,
    pub generation: u64,
}

impl ApprovalBinding {
    pub fn new(
        project_id: impl Into<String>,
        workflow_id: impl Into<String>,
        run_id: impl Into<String>,
        node_id: impl Into<String>,
        generation: u64,
    ) -> Result<Self, ApprovalError> {
        let binding = Self {
            project_id: project_id.into(),
            workflow_id: workflow_id.into(),
            run_id: run_id.into(),
            node_id: node_id.into(),
            generation,
        };
        for value in [
            &binding.project_id,
            &binding.workflow_id,
            &binding.run_id,
            &binding.node_id,
        ] {
            if value.trim().is_empty() || value.len() > MAX_ID_BYTES {
                return Err(ApprovalError::InvalidBinding);
            }
        }
        Ok(binding)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    Pending,
    Approved,
    Denied,
    Expired,
    Cancelled,
    Consumed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub request_id: Uuid,
    pub binding: ApprovalBinding,
    pub approver_id: String,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalToken {
    request_id: Uuid,
    token_id: Uuid,
    binding: ApprovalBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ApprovalError {
    #[error("approval ledger capacity is invalid")]
    InvalidCapacity,
    #[error("approval binding is invalid")]
    InvalidBinding,
    #[error("approval actor is invalid")]
    InvalidActor,
    #[error("approval lifetime is invalid")]
    InvalidLifetime,
    #[error("approval ledger is full")]
    CapacityFull,
    #[error("approval request was not found")]
    NotFound,
    #[error("approval actor does not match")]
    ActorMismatch,
    #[error("approval binding does not match")]
    BindingMismatch,
    #[error("approval request has expired")]
    Expired,
    #[error("approval decision was already made")]
    DuplicateDecision,
    #[error("approval request was cancelled")]
    Cancelled,
    #[error("approval token replay was rejected")]
    Replay,
    #[error("approval token is invalid")]
    InvalidToken,
    #[error("approval is not approved")]
    NotApproved,
    #[error("approval ledger lock is unavailable")]
    LedgerUnavailable,
}

#[derive(Debug)]
struct Entry {
    request: ApprovalRequest,
    state: ApprovalState,
    token: Option<ApprovalToken>,
}

#[derive(Debug)]
pub struct ApprovalLedger {
    max_pending: usize,
    entries: Mutex<BTreeMap<Uuid, Entry>>,
}

impl ApprovalLedger {
    pub fn new(max_pending: usize) -> Result<Self, ApprovalError> {
        if max_pending == 0 {
            return Err(ApprovalError::InvalidCapacity);
        }
        Ok(Self {
            max_pending,
            entries: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn submit(
        &self,
        binding: ApprovalBinding,
        approver_id: impl Into<String>,
        created_at_ms: u64,
        lifetime_ms: u64,
        max_lifetime_ms: u64,
    ) -> Result<ApprovalRequest, ApprovalError> {
        let approver_id = approver_id.into();
        if approver_id.trim().is_empty() || approver_id.len() > MAX_ID_BYTES {
            return Err(ApprovalError::InvalidActor);
        }
        if lifetime_ms == 0 || lifetime_ms > max_lifetime_ms {
            return Err(ApprovalError::InvalidLifetime);
        }
        let expires_at_ms = created_at_ms
            .checked_add(lifetime_ms)
            .ok_or(ApprovalError::InvalidLifetime)?;
        let request = ApprovalRequest {
            request_id: Uuid::new_v4(),
            binding,
            approver_id,
            created_at_ms,
            expires_at_ms,
        };
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| ApprovalError::LedgerUnavailable)?;
        if entries.len() >= self.max_pending {
            return Err(ApprovalError::CapacityFull);
        }
        entries.insert(
            request.request_id,
            Entry {
                request: request.clone(),
                state: ApprovalState::Pending,
                token: None,
            },
        );
        Ok(request)
    }

    pub fn state(&self, request_id: Uuid) -> Option<ApprovalState> {
        self.entries
            .lock()
            .ok()
            .and_then(|entries| entries.get(&request_id).map(|entry| entry.state))
    }

    pub fn decide(
        &self,
        request_id: Uuid,
        binding: &ApprovalBinding,
        actor_id: &str,
        decision: ApprovalDecision,
        now_ms: u64,
    ) -> Result<Option<ApprovalToken>, ApprovalError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| ApprovalError::LedgerUnavailable)?;
        let entry = entries
            .get_mut(&request_id)
            .ok_or(ApprovalError::NotFound)?;
        if &entry.request.binding != binding {
            return Err(ApprovalError::BindingMismatch);
        }
        if entry.request.approver_id != actor_id {
            return Err(ApprovalError::ActorMismatch);
        }
        if entry.state != ApprovalState::Pending {
            return Err(ApprovalError::DuplicateDecision);
        }
        if now_ms >= entry.request.expires_at_ms {
            entry.state = ApprovalState::Expired;
            return Err(ApprovalError::Expired);
        }
        match decision {
            ApprovalDecision::Deny => {
                entry.state = ApprovalState::Denied;
                Ok(None)
            }
            ApprovalDecision::Allow => {
                let token = ApprovalToken {
                    request_id,
                    token_id: Uuid::new_v4(),
                    binding: binding.clone(),
                };
                entry.token = Some(token.clone());
                entry.state = ApprovalState::Approved;
                Ok(Some(token))
            }
        }
    }

    pub fn cancel(&self, request_id: Uuid, binding: &ApprovalBinding) -> Result<(), ApprovalError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| ApprovalError::LedgerUnavailable)?;
        let entry = entries
            .get_mut(&request_id)
            .ok_or(ApprovalError::NotFound)?;
        if &entry.request.binding != binding {
            return Err(ApprovalError::BindingMismatch);
        }
        if entry.state != ApprovalState::Pending {
            return Err(ApprovalError::DuplicateDecision);
        }
        entry.state = ApprovalState::Cancelled;
        Ok(())
    }

    pub fn resume(
        &self,
        request_id: Uuid,
        binding: &ApprovalBinding,
        token: &ApprovalToken,
        now_ms: u64,
    ) -> Result<(), ApprovalError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| ApprovalError::LedgerUnavailable)?;
        let entry = entries
            .get_mut(&request_id)
            .ok_or(ApprovalError::NotFound)?;
        if entry.state == ApprovalState::Consumed {
            return Err(ApprovalError::Replay);
        }
        if entry.state != ApprovalState::Approved {
            return Err(ApprovalError::NotApproved);
        }
        if now_ms >= entry.request.expires_at_ms {
            entry.state = ApprovalState::Expired;
            return Err(ApprovalError::Expired);
        }
        if &entry.request.binding != binding
            || &token.binding != binding
            || token.request_id != request_id
            || entry.token.as_ref() != Some(token)
        {
            return Err(ApprovalError::InvalidToken);
        }
        entry.state = ApprovalState::Consumed;
        Ok(())
    }
}
