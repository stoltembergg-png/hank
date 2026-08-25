//! Monotonic, non-blocking DelayNode state machine.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayState {
    Waiting,
    Paused,
    Ready,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DelayError {
    #[error("delay duration exceeds configured limit")]
    DurationExceeded,
    #[error("delay deadline overflows monotonic clock")]
    DeadlineOverflow,
    #[error("delay state transition is invalid")]
    InvalidTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayPlan {
    state: DelayState,
    deadline: u64,
    remaining: u64,
    last_now: u64,
}

impl DelayPlan {
    pub fn new(now: u64, duration: u64, max_duration: u64) -> Result<Self, DelayError> {
        if duration > max_duration {
            return Err(DelayError::DurationExceeded);
        }
        let deadline = now
            .checked_add(duration)
            .ok_or(DelayError::DeadlineOverflow)?;
        let state = if duration == 0 {
            DelayState::Ready
        } else {
            DelayState::Waiting
        };
        Ok(Self {
            state,
            deadline,
            remaining: duration,
            last_now: now,
        })
    }

    pub fn state(&self) -> DelayState {
        self.state
    }

    pub fn poll(&mut self, now: u64) -> DelayState {
        let now = now.max(self.last_now);
        self.last_now = now;
        if self.state == DelayState::Waiting && now >= self.deadline {
            self.state = DelayState::Ready;
            self.remaining = 0;
        }
        self.state
    }

    pub fn pause(&mut self, now: u64) -> DelayState {
        if self.poll(now) != DelayState::Waiting {
            return self.state;
        }
        self.remaining = self.deadline.saturating_sub(self.last_now);
        self.state = DelayState::Paused;
        self.state
    }

    pub fn resume(&mut self, now: u64) -> Result<DelayState, DelayError> {
        if self.state != DelayState::Paused {
            return Err(DelayError::InvalidTransition);
        }
        let now = now.max(self.last_now);
        self.last_now = now;
        self.deadline = now
            .checked_add(self.remaining)
            .ok_or(DelayError::DeadlineOverflow)?;
        self.state = if self.remaining == 0 {
            DelayState::Ready
        } else {
            DelayState::Waiting
        };
        Ok(self.state)
    }

    pub fn cancel(&mut self) -> bool {
        if matches!(self.state, DelayState::Cancelled | DelayState::Ready) {
            return false;
        }
        self.state = DelayState::Cancelled;
        true
    }
}
