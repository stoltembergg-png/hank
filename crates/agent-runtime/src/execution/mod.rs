//! Provider-neutral Agent execution state machine.

use crate::provider_service::{
    InvocationError, InvocationRequest, InvocationResult, ProviderApplicationService,
};
use agent_core::ids::{AgentId, SessionId};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use thiserror::Error;

const MAX_ID_LEN: usize = 128;
const MAX_FAILURE_CODE_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Preparing,
    Running,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

impl ExecutionState {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionEvent {
    Start,
    ProviderInvoked(String),
    StreamStarted,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("execution identity is invalid")]
    InvalidIdentity,
    #[error("execution transition is illegal from the current state")]
    IllegalTransition {
        state: ExecutionState,
        event: &'static str,
    },
    #[error("execution is already terminal")]
    TerminalState,
    #[error("provider invocation identity was duplicated")]
    DuplicateInvocation,
    #[error("execution generation is stale")]
    StaleGeneration,
    #[error("execution budget was exceeded")]
    BudgetExceeded,
    #[error("execution concurrency limit was reached")]
    ConcurrencyLimit,
    #[error("execution was cancelled")]
    Cancelled,
    #[error("provider execution failed")]
    ProviderFailed,
    #[error("execution snapshot is invalid")]
    InvalidSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSnapshot {
    pub execution_id: String,
    pub session_id: SessionId,
    pub agent_id: AgentId,
    pub correlation_id: String,
    pub generation: u64,
    pub state: ExecutionState,
    pub provider_invocation_id: Option<String>,
    pub failure_code: Option<String>,
    pub used_tokens: u64,
    pub used_cost_micros: u64,
    pub max_tokens: u64,
    pub max_cost_micros: u64,
}

#[derive(Debug, Clone)]
pub struct Execution {
    execution_id: String,
    session_id: SessionId,
    agent_id: AgentId,
    correlation_id: String,
    generation: u64,
    state: ExecutionState,
    provider_invocation_id: Option<String>,
    failure_code: Option<String>,
    used_tokens: u64,
    used_cost_micros: u64,
    max_tokens: u64,
    max_cost_micros: u64,
}

impl Execution {
    pub fn new(
        execution_id: impl Into<String>,
        session_id: SessionId,
        agent_id: AgentId,
        correlation_id: impl Into<String>,
        generation: u64,
        max_tokens: u64,
        max_cost_micros: u64,
    ) -> Result<Self, ExecutionError> {
        let execution_id = execution_id.into();
        let correlation_id = correlation_id.into();
        if !valid_id(&execution_id)
            || !valid_id(&correlation_id)
            || generation == 0
            || max_tokens == 0
        {
            return Err(ExecutionError::InvalidIdentity);
        }
        Ok(Self {
            execution_id,
            session_id,
            agent_id,
            correlation_id,
            generation,
            state: ExecutionState::Preparing,
            provider_invocation_id: None,
            failure_code: None,
            used_tokens: 0,
            used_cost_micros: 0,
            max_tokens,
            max_cost_micros,
        })
    }

    pub fn apply(&mut self, event: ExecutionEvent) -> Result<(), ExecutionError> {
        if self.state.is_terminal() {
            return Err(ExecutionError::TerminalState);
        }
        match event {
            ExecutionEvent::Start if self.state == ExecutionState::Preparing => {
                self.state = ExecutionState::Running;
                Ok(())
            }
            ExecutionEvent::ProviderInvoked(invocation_id)
                if self.state == ExecutionState::Running =>
            {
                if self.provider_invocation_id.is_some() || !valid_id(&invocation_id) {
                    return Err(ExecutionError::DuplicateInvocation);
                }
                self.provider_invocation_id = Some(invocation_id);
                Ok(())
            }
            ExecutionEvent::StreamStarted if self.state == ExecutionState::Running => {
                self.state = ExecutionState::Streaming;
                Ok(())
            }
            ExecutionEvent::Completed
                if matches!(
                    self.state,
                    ExecutionState::Running | ExecutionState::Streaming
                ) =>
            {
                self.state = ExecutionState::Completed;
                Ok(())
            }
            ExecutionEvent::Failed(code)
                if matches!(
                    self.state,
                    ExecutionState::Preparing | ExecutionState::Running | ExecutionState::Streaming
                ) =>
            {
                self.state = ExecutionState::Failed;
                self.failure_code = Some(sanitize_code(&code));
                Ok(())
            }
            ExecutionEvent::Cancelled
                if matches!(
                    self.state,
                    ExecutionState::Preparing | ExecutionState::Running | ExecutionState::Streaming
                ) =>
            {
                self.state = ExecutionState::Cancelled;
                Ok(())
            }
            other => Err(ExecutionError::IllegalTransition {
                state: self.state,
                event: event_name(&other),
            }),
        }
    }

    pub fn accept_generation(&self, generation: u64) -> Result<(), ExecutionError> {
        if generation != self.generation {
            return Err(ExecutionError::StaleGeneration);
        }
        Ok(())
    }

    pub fn record_usage(&mut self, tokens: u64, cost_micros: u64) -> Result<(), ExecutionError> {
        let next_tokens = self.used_tokens.saturating_add(tokens);
        let next_cost = self.used_cost_micros.saturating_add(cost_micros);
        if next_tokens > self.max_tokens || next_cost > self.max_cost_micros {
            if !self.state.is_terminal() {
                self.apply(ExecutionEvent::Failed("budget_exceeded".into()))?;
            }
            return Err(ExecutionError::BudgetExceeded);
        }
        self.used_tokens = next_tokens;
        self.used_cost_micros = next_cost;
        Ok(())
    }

    pub fn snapshot(&self) -> ExecutionSnapshot {
        ExecutionSnapshot {
            execution_id: self.execution_id.clone(),
            session_id: self.session_id,
            agent_id: self.agent_id,
            correlation_id: self.correlation_id.clone(),
            generation: self.generation,
            state: self.state,
            provider_invocation_id: self.provider_invocation_id.clone(),
            failure_code: self.failure_code.clone(),
            used_tokens: self.used_tokens,
            used_cost_micros: self.used_cost_micros,
            max_tokens: self.max_tokens,
            max_cost_micros: self.max_cost_micros,
        }
    }

    pub fn restore(snapshot: ExecutionSnapshot) -> Result<Self, ExecutionError> {
        let execution = Self::new(
            snapshot.execution_id,
            snapshot.session_id,
            snapshot.agent_id,
            snapshot.correlation_id,
            snapshot.generation,
            snapshot.max_tokens,
            snapshot.max_cost_micros,
        )?;
        if snapshot.used_tokens > snapshot.max_tokens
            || snapshot.used_cost_micros > snapshot.max_cost_micros
            || (snapshot.state == ExecutionState::Failed && snapshot.failure_code.is_none())
        {
            return Err(ExecutionError::InvalidSnapshot);
        }
        Ok(Self {
            state: snapshot.state,
            provider_invocation_id: snapshot.provider_invocation_id,
            failure_code: snapshot.failure_code.map(|code| sanitize_code(&code)),
            used_tokens: snapshot.used_tokens,
            used_cost_micros: snapshot.used_cost_micros,
            ..execution
        })
    }

    pub fn state(&self) -> ExecutionState {
        self.state
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn terminal_reason(&self) -> Option<&str> {
        match self.state {
            ExecutionState::Completed => Some("completed"),
            ExecutionState::Cancelled => Some("cancelled"),
            ExecutionState::Failed => self.failure_code.as_deref(),
            _ => None,
        }
    }
}

/// Coordinates one complete invocation through the application service only.
pub struct ExecutionCoordinator;

impl ExecutionCoordinator {
    pub async fn complete(
        execution: &mut Execution,
        service: &ProviderApplicationService,
        request: InvocationRequest,
        generation: u64,
    ) -> Result<InvocationResult, ExecutionError> {
        execution.accept_generation(generation)?;
        if request.access.cancellation.is_cancelled() {
            execution.apply(ExecutionEvent::Cancelled)?;
            return Err(ExecutionError::Cancelled);
        }
        execution.apply(ExecutionEvent::Start)?;
        let invocation_id = request.normalized.request_id.clone();
        execution.apply(ExecutionEvent::ProviderInvoked(invocation_id))?;
        match service.complete(request).await {
            Ok(result) => {
                execution.record_usage(
                    u64::from(result.usage.input_tokens) + u64::from(result.usage.output_tokens),
                    0,
                )?;
                execution.apply(ExecutionEvent::Completed)?;
                Ok(result)
            }
            Err(InvocationError::Cancelled) => {
                execution.apply(ExecutionEvent::Cancelled)?;
                Err(ExecutionError::Cancelled)
            }
            Err(_) => {
                execution.apply(ExecutionEvent::Failed("provider_error".into()))?;
                Err(ExecutionError::ProviderFailed)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionConcurrency {
    max: usize,
    active: Arc<AtomicUsize>,
}

impl ExecutionConcurrency {
    pub fn new(max: usize) -> Result<Self, ExecutionError> {
        if max == 0 {
            return Err(ExecutionError::ConcurrencyLimit);
        }
        Ok(Self {
            max,
            active: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn try_acquire(&self) -> Result<ExecutionLease, ExecutionError> {
        loop {
            let current = self.active.load(Ordering::Acquire);
            if current >= self.max {
                return Err(ExecutionError::ConcurrencyLimit);
            }
            if self
                .active
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(ExecutionLease {
                    active: Arc::clone(&self.active),
                });
            }
        }
    }
}

#[derive(Debug)]
pub struct ExecutionLease {
    active: Arc<AtomicUsize>,
}

impl Drop for ExecutionLease {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ID_LEN
        && value.chars().all(|character| !character.is_control())
}

fn sanitize_code(value: &str) -> String {
    if value.len() <= MAX_FAILURE_CODE_LEN
        && !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
    {
        value.to_owned()
    } else {
        "redacted_error".into()
    }
}

fn event_name(event: &ExecutionEvent) -> &'static str {
    match event {
        ExecutionEvent::Start => "start",
        ExecutionEvent::ProviderInvoked(_) => "provider_invoked",
        ExecutionEvent::StreamStarted => "stream_started",
        ExecutionEvent::Completed => "completed",
        ExecutionEvent::Failed(_) => "failed",
        ExecutionEvent::Cancelled => "cancelled",
    }
}
