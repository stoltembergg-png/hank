//! Bounded provider-neutral cancellation boundary.

use crate::execution::{Execution, ExecutionError, ExecutionEvent, ExecutionState};
use agent_core::session::{Message, MessageError, MessageStatus};
use provider_core::CancellationToken;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;

const MAX_EXECUTION_ID_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationOutcome {
    Applied,
    AlreadyCancelled,
    AlreadyTerminal,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CancellationError {
    #[error("execution identity is invalid")]
    InvalidIdentity,
    #[error("cancellation registry capacity was reached")]
    Capacity,
    #[error("execution is not registered for cancellation")]
    UnknownExecution,
    #[error("execution cancellation state transition failed")]
    State,
    #[error("cancellation registry lock is unavailable")]
    Lock,
}

#[derive(Clone)]
pub struct CancellationHandle {
    token: CancellationToken,
}

impl CancellationHandle {
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

pub struct CancellationRegistry {
    max: usize,
    entries: Arc<Mutex<BTreeMap<String, CancellationToken>>>,
}

impl fmt::Debug for CancellationRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ids = self
            .entries
            .lock()
            .map(|entries| entries.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        formatter
            .debug_struct("CancellationRegistry")
            .field("max", &self.max)
            .field("execution_ids", &ids)
            .finish()
    }
}

impl CancellationRegistry {
    pub fn new(max: usize) -> Result<Self, CancellationError> {
        if max == 0 {
            return Err(CancellationError::Capacity);
        }
        Ok(Self {
            max,
            entries: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    pub fn register(
        &self,
        execution_id: impl Into<String>,
    ) -> Result<CancellationHandle, CancellationError> {
        let execution_id = execution_id.into();
        if !valid_execution_id(&execution_id) {
            return Err(CancellationError::InvalidIdentity);
        }
        let mut entries = self.entries.lock().map_err(|_| CancellationError::Lock)?;
        if entries.contains_key(&execution_id) {
            return Err(CancellationError::InvalidIdentity);
        }
        if entries.len() >= self.max {
            return Err(CancellationError::Capacity);
        }
        let token = CancellationToken::new();
        entries.insert(execution_id, token.clone());
        Ok(CancellationHandle { token })
    }

    pub fn cancel(&self, execution_id: &str) -> Result<CancellationOutcome, CancellationError> {
        let entries = self.entries.lock().map_err(|_| CancellationError::Lock)?;
        let token = entries
            .get(execution_id)
            .ok_or(CancellationError::UnknownExecution)?;
        if token.is_cancelled() {
            return Ok(CancellationOutcome::AlreadyCancelled);
        }
        token.cancel();
        Ok(CancellationOutcome::Applied)
    }

    pub fn unregister(&self, execution_id: &str) -> Result<(), CancellationError> {
        let mut entries = self.entries.lock().map_err(|_| CancellationError::Lock)?;
        entries
            .remove(execution_id)
            .map(|_| ())
            .ok_or(CancellationError::UnknownExecution)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }
}

pub fn cancel_turn(
    execution: &mut Execution,
    message: &mut Message,
    token: &CancellationToken,
) -> Result<CancellationOutcome, CancellationError> {
    token.cancel();
    if execution.state() == ExecutionState::Cancelled && message.status == MessageStatus::Cancelled
    {
        return Ok(CancellationOutcome::AlreadyCancelled);
    }
    if matches!(
        execution.state(),
        ExecutionState::Completed | ExecutionState::Failed | ExecutionState::Cancelled
    ) || message.status.is_terminal()
    {
        return Ok(CancellationOutcome::AlreadyTerminal);
    }
    execution
        .apply(ExecutionEvent::Cancelled)
        .map_err(map_execution_error)?;
    message.cancel().map_err(map_message_error)?;
    Ok(CancellationOutcome::Applied)
}

fn valid_execution_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_EXECUTION_ID_LEN
        && value.chars().all(|character| !character.is_control())
}

fn map_execution_error(_: ExecutionError) -> CancellationError {
    CancellationError::State
}

fn map_message_error(_: MessageError) -> CancellationError {
    CancellationError::State
}
