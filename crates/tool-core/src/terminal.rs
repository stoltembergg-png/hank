//! Terminal adapter over the structured process primitive.

use crate::{ProcessError, ProcessResult, ProcessSpec, run_process};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

pub const MAX_TERMINAL_ROUNDS: u8 = 8;

#[derive(Debug, Clone)]
pub struct TerminalRequest {
    pub process: ProcessSpec,
    pub operation_key: String,
    pub max_rounds: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalResult {
    pub process: ProcessResult,
    pub operation_key: String,
    pub round: u8,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TerminalError {
    #[error("operation key is required")]
    MissingOperationKey,
    #[error("terminal round limit is invalid")]
    InvalidRoundLimit,
    #[error("operation was already executed")]
    DuplicateOperation,
    #[error("process primitive failed")]
    Process(#[from] ProcessError),
}

#[derive(Debug, Default)]
pub struct TerminalAdapter {
    completed: Mutex<BTreeSet<String>>,
}

impl TerminalAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute(
        &self,
        request: TerminalRequest,
        cancel: Arc<AtomicBool>,
    ) -> Result<TerminalResult, TerminalError> {
        if request.operation_key.trim().is_empty() {
            return Err(TerminalError::MissingOperationKey);
        }
        if request.max_rounds == 0 || request.max_rounds > MAX_TERMINAL_ROUNDS {
            return Err(TerminalError::InvalidRoundLimit);
        }
        let mut completed = self
            .completed
            .lock()
            .map_err(|_| TerminalError::DuplicateOperation)?;
        if !completed.insert(request.operation_key.clone()) {
            return Err(TerminalError::DuplicateOperation);
        }
        drop(completed);
        match run_process(&request.process, cancel) {
            Ok(process) => Ok(TerminalResult {
                process,
                operation_key: request.operation_key,
                round: 1,
            }),
            Err(error) => {
                if let Ok(mut completed) = self.completed.lock() {
                    completed.remove(&request.operation_key);
                }
                Err(TerminalError::Process(error))
            }
        }
    }
}
