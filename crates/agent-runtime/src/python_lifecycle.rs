//! Bounded supervision for the optional Python worker sidecar.
//!
//! This module owns process identity, lifecycle transitions, operation
//! idempotency and budget reservations. It deliberately does not execute tools
//! or provide a Python SDK; those are later queue cards.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::process::{Child, Command};
use tracing::{info, warn};

const MAX_COMMAND_ARGS: usize = 32;
const MAX_OPERATION_KEY_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Stopped,
    Starting,
    Ready,
    Busy,
    Crashed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerIdentity {
    pub project_id: String,
    pub session_id: String,
    pub task_id: String,
    pub trace_id: String,
}

impl WorkerIdentity {
    fn validate(&self) -> Result<(), LifecycleError> {
        for (name, value) in [
            ("project_id", &self.project_id),
            ("session_id", &self.session_id),
            ("task_id", &self.task_id),
            ("trace_id", &self.trace_id),
        ] {
            if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
                return Err(LifecycleError::InvalidIdentity(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PythonLifecycleConfig {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub startup_timeout: Duration,
    pub request_timeout: Duration,
    pub max_restarts: u32,
    pub restart_backoff: Duration,
}

impl PythonLifecycleConfig {
    fn validate(&self) -> Result<(), LifecycleError> {
        if self.command.as_os_str().is_empty() || self.args.len() > MAX_COMMAND_ARGS {
            return Err(LifecycleError::InvalidConfiguration);
        }
        if self
            .args
            .iter()
            .any(|arg| arg.len() > 1024 || arg.chars().any(char::is_control))
        {
            return Err(LifecycleError::InvalidConfiguration);
        }
        if self.startup_timeout.is_zero() || self.request_timeout.is_zero() {
            return Err(LifecycleError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    StateChanged {
        from: LifecycleState,
        to: LifecycleState,
    },
    RequestStarted {
        operation_key: String,
        budget: u64,
    },
    RequestFinished {
        operation_key: String,
    },
    BudgetReleased {
        operation_key: String,
        budget: u64,
    },
    WorkerExited {
        status: Option<i32>,
    },
    Restarted {
        count: u32,
    },
}

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("invalid worker identity: {0}")]
    InvalidIdentity(&'static str),
    #[error("invalid lifecycle configuration")]
    InvalidConfiguration,
    #[error("invalid lifecycle transition from {state:?} for operation {operation}")]
    InvalidTransition {
        state: LifecycleState,
        operation: &'static str,
    },
    #[error("worker spawn failed: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("worker cleanup failed: {0}")]
    Cleanup(#[source] std::io::Error),
    #[error("operation key is invalid: {0}")]
    InvalidOperation(String),
    #[error("operation key is already active or was already completed: {0}")]
    DuplicateOperation(String),
    #[error("operation does not match the active worker request: {0}")]
    UnknownOperation(String),
    #[error("restart policy exhausted")]
    RestartLimit,
}

struct ActiveOperation {
    key: String,
    budget: u64,
    deadline: Instant,
}

pub struct PythonLifecycle {
    config: PythonLifecycleConfig,
    identity: WorkerIdentity,
    state: LifecycleState,
    child: Option<Child>,
    restart_count: u32,
    active: Option<ActiveOperation>,
    startup_deadline: Option<Instant>,
    reserved_budget: u64,
    seen_operations: HashSet<String>,
    events: Vec<LifecycleEvent>,
}

impl PythonLifecycle {
    pub fn new(
        config: PythonLifecycleConfig,
        identity: WorkerIdentity,
    ) -> Result<Self, LifecycleError> {
        config.validate()?;
        identity.validate()?;
        Ok(Self {
            config,
            identity,
            state: LifecycleState::Stopped,
            child: None,
            restart_count: 0,
            active: None,
            startup_deadline: None,
            reserved_budget: 0,
            seen_operations: HashSet::new(),
            events: Vec::new(),
        })
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }
    pub fn identity(&self) -> &WorkerIdentity {
        &self.identity
    }
    pub fn restart_count(&self) -> u32 {
        self.restart_count
    }
    pub fn reserved_budget(&self) -> u64 {
        self.reserved_budget
    }
    pub fn events(&self) -> &[LifecycleEvent] {
        &self.events
    }
    pub fn request_deadline(&self) -> Option<Instant> {
        self.active.as_ref().map(|active| active.deadline)
    }
    pub fn readiness_deadline(&self) -> Option<Instant> {
        self.startup_deadline
    }

    pub async fn spawn(&mut self) -> Result<(), LifecycleError> {
        self.require_state(LifecycleState::Stopped, "spawn")?;
        self.transition(LifecycleState::Starting);
        self.startup_deadline = Some(Instant::now() + self.config.startup_timeout);

        let mut command = Command::new(&self.config.command);
        command
            .args(&self.config.args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .env_clear();

        match command.spawn() {
            Ok(child) => {
                self.child = Some(child);
                info!(project_id = %self.identity.project_id, task_id = %self.identity.task_id, "python worker spawned");
                Ok(())
            }
            Err(error) => {
                self.child = None;
                self.startup_deadline = None;
                self.transition(LifecycleState::Crashed);
                Err(LifecycleError::Spawn(error))
            }
        }
    }

    pub fn mark_ready(&mut self) -> Result<(), LifecycleError> {
        self.require_state(LifecycleState::Starting, "mark_ready")?;
        self.startup_deadline = None;
        self.transition(LifecycleState::Ready);
        Ok(())
    }

    pub fn begin_request(
        &mut self,
        operation_key: &str,
        budget: u64,
    ) -> Result<(), LifecycleError> {
        self.validate_operation(operation_key)?;
        if self.seen_operations.contains(operation_key) || self.active.is_some() {
            return Err(LifecycleError::DuplicateOperation(operation_key.to_owned()));
        }
        self.require_state(LifecycleState::Ready, "begin_request")?;
        self.seen_operations.insert(operation_key.to_owned());
        self.reserved_budget = self.reserved_budget.saturating_add(budget);
        self.active = Some(ActiveOperation {
            key: operation_key.to_owned(),
            budget,
            deadline: Instant::now() + self.config.request_timeout,
        });
        self.transition(LifecycleState::Busy);
        self.events.push(LifecycleEvent::RequestStarted {
            operation_key: operation_key.to_owned(),
            budget,
        });
        Ok(())
    }

    pub fn complete_request(&mut self, operation_key: &str) -> Result<(), LifecycleError> {
        self.finish_request(operation_key)?;
        self.transition(LifecycleState::Ready);
        Ok(())
    }

    pub async fn timeout_request(&mut self, operation_key: &str) -> Result<(), LifecycleError> {
        self.finish_request(operation_key)?;
        self.transition(LifecycleState::TimedOut);
        self.cleanup_child().await?;
        self.transition(LifecycleState::Stopped);
        Ok(())
    }

    pub async fn cancel_request(&mut self, operation_key: &str) -> Result<(), LifecycleError> {
        self.finish_request(operation_key)?;
        self.transition(LifecycleState::Cancelled);
        self.cleanup_child().await?;
        self.transition(LifecycleState::Stopped);
        Ok(())
    }

    pub async fn crash(&mut self) -> Result<(), LifecycleError> {
        self.release_active_budget();
        self.cleanup_child().await?;
        self.transition(LifecycleState::Crashed);
        Ok(())
    }

    pub async fn poll_exit(&mut self) -> Result<Option<ExitStatus>, LifecycleError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        let status = child.try_wait().map_err(LifecycleError::Cleanup)?;
        if let Some(status) = status {
            self.child = None;
            self.release_active_budget();
            self.events.push(LifecycleEvent::WorkerExited {
                status: status.code(),
            });
            self.transition(LifecycleState::Crashed);
        }
        Ok(status)
    }

    pub async fn restart(&mut self) -> Result<(), LifecycleError> {
        if !matches!(
            self.state,
            LifecycleState::Crashed
                | LifecycleState::Stopped
                | LifecycleState::TimedOut
                | LifecycleState::Cancelled
        ) {
            return Err(LifecycleError::InvalidTransition {
                state: self.state,
                operation: "restart",
            });
        }
        if self.restart_count >= self.config.max_restarts {
            warn!(project_id = %self.identity.project_id, "python worker restart policy exhausted");
            return Err(LifecycleError::RestartLimit);
        }
        if !self.config.restart_backoff.is_zero() {
            tokio::time::sleep(self.config.restart_backoff).await;
        }
        self.restart_count += 1;
        self.transition(LifecycleState::Stopped);
        self.spawn().await?;
        self.events.push(LifecycleEvent::Restarted {
            count: self.restart_count,
        });
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), LifecycleError> {
        if self.state == LifecycleState::Stopped {
            return Ok(());
        }
        self.release_active_budget();
        self.cleanup_child().await?;
        self.transition(LifecycleState::Stopped);
        Ok(())
    }

    fn finish_request(&mut self, operation_key: &str) -> Result<(), LifecycleError> {
        let active = self
            .active
            .take()
            .ok_or_else(|| LifecycleError::UnknownOperation(operation_key.to_owned()))?;
        if active.key != operation_key {
            self.active = Some(active);
            return Err(LifecycleError::UnknownOperation(operation_key.to_owned()));
        }
        self.reserved_budget = self.reserved_budget.saturating_sub(active.budget);
        self.events.push(LifecycleEvent::RequestFinished {
            operation_key: operation_key.to_owned(),
        });
        self.events.push(LifecycleEvent::BudgetReleased {
            operation_key: operation_key.to_owned(),
            budget: active.budget,
        });
        Ok(())
    }

    fn release_active_budget(&mut self) {
        if let Some(active) = self.active.take() {
            self.reserved_budget = self.reserved_budget.saturating_sub(active.budget);
            self.events.push(LifecycleEvent::BudgetReleased {
                operation_key: active.key,
                budget: active.budget,
            });
        }
    }

    async fn cleanup_child(&mut self) -> Result<(), LifecycleError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        if child.try_wait().map_err(LifecycleError::Cleanup)?.is_none() {
            child.kill().await.map_err(LifecycleError::Cleanup)?;
        }
        let _ = child.wait().await.map_err(LifecycleError::Cleanup)?;
        Ok(())
    }

    fn validate_operation(&self, key: &str) -> Result<(), LifecycleError> {
        if key.is_empty()
            || key.len() > MAX_OPERATION_KEY_LENGTH
            || key.chars().any(char::is_control)
        {
            return Err(LifecycleError::InvalidOperation(key.to_owned()));
        }
        Ok(())
    }

    fn require_state(
        &self,
        expected: LifecycleState,
        operation: &'static str,
    ) -> Result<(), LifecycleError> {
        if self.state != expected {
            return Err(LifecycleError::InvalidTransition {
                state: self.state,
                operation,
            });
        }
        Ok(())
    }

    fn transition(&mut self, next: LifecycleState) {
        if self.state != next {
            let from = self.state;
            self.state = next;
            self.events
                .push(LifecycleEvent::StateChanged { from, to: next });
        }
    }
}
