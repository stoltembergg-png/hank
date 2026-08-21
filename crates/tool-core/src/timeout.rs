//! Shared monotonic deadline and cancellation state for tool adapters.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTerminalState {
    Completed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolExecutionStatus {
    Active,
    Terminal(ToolTerminalState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutError {
    ZeroDuration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("tool timeout must be greater than zero")
    }
}

impl std::error::Error for TimeoutError {}

#[derive(Debug, Clone)]
pub struct ToolCancellation {
    flag: Arc<AtomicBool>,
}

impl ToolCancellation {
    pub fn new() -> Self {
        Self::from_flag(Arc::new(AtomicBool::new(false)))
    }

    pub fn from_flag(flag: Arc<AtomicBool>) -> Self {
        Self { flag }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }
}

impl Default for ToolCancellation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ToolExecutionWindow {
    started_at: Instant,
    deadline: Instant,
    timeout: Duration,
    cancellation: ToolCancellation,
    terminal: Mutex<Option<ToolTerminalState>>,
}

impl ToolExecutionWindow {
    pub fn new(timeout: Duration) -> Result<Self, TimeoutError> {
        Self::with_cancellation(timeout, ToolCancellation::new())
    }

    pub fn with_cancellation(
        timeout: Duration,
        cancellation: ToolCancellation,
    ) -> Result<Self, TimeoutError> {
        if timeout.is_zero() {
            return Err(TimeoutError::ZeroDuration);
        }
        let started_at = Instant::now();
        Ok(Self {
            started_at,
            deadline: started_at + timeout,
            timeout,
            cancellation,
            terminal: Mutex::new(None),
        })
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    pub fn cancellation(&self) -> ToolCancellation {
        self.cancellation.clone()
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn is_active(&self) -> bool {
        matches!(self.poll(), ToolExecutionStatus::Active)
    }

    pub fn poll(&self) -> ToolExecutionStatus {
        let mut terminal = self.terminal.lock().expect("tool execution window lock");
        if let Some(state) = *terminal {
            return ToolExecutionStatus::Terminal(state);
        }
        let state = if self.cancellation.is_cancelled() {
            Some(ToolTerminalState::Cancelled)
        } else if self.remaining().is_zero() {
            Some(ToolTerminalState::TimedOut)
        } else {
            None
        };
        if let Some(state) = state {
            *terminal = Some(state);
            ToolExecutionStatus::Terminal(state)
        } else {
            ToolExecutionStatus::Active
        }
    }

    pub fn finish(&self) -> ToolTerminalState {
        let mut terminal = self.terminal.lock().expect("tool execution window lock");
        if let Some(state) = *terminal {
            return state;
        }
        let state = if self.cancellation.is_cancelled() {
            ToolTerminalState::Cancelled
        } else if self.remaining().is_zero() {
            ToolTerminalState::TimedOut
        } else {
            ToolTerminalState::Completed
        };
        *terminal = Some(state);
        state
    }
}
