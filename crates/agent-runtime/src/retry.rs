//! Bounded provider retry decision policy; no retry side effects or sleeping.

use provider_core::CancellationToken;
use thiserror::Error;

const MAX_REQUEST_ID_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryOperationKind {
    Completion,
    Stream,
    Tool,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryFailure {
    RateLimited,
    Timeout,
    Outage,
    Quota,
    Authentication,
    InvalidRequest,
    Cancelled,
    Permanent,
}

impl RetryFailure {
    fn retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::Timeout | Self::Outage | Self::Quota
        )
    }
}

#[derive(Debug, Clone)]
pub struct RetryContext {
    pub request_id: String,
    pub operation: RetryOperationKind,
    pub attempts_used: u32,
    pub tokens_used: u64,
    pub max_tokens: u64,
    pub cancellation: CancellationToken,
}

impl RetryContext {
    pub fn new(
        request_id: impl Into<String>,
        operation: RetryOperationKind,
        attempts_used: u32,
        tokens_used: u64,
        max_tokens: u64,
        cancellation: CancellationToken,
    ) -> Result<Self, RetryError> {
        let request_id = request_id.into();
        if !valid_request_id(&request_id) || max_tokens == 0 {
            return Err(RetryError::InvalidContext);
        }
        Ok(Self {
            request_id,
            operation,
            attempts_used,
            tokens_used,
            max_tokens,
            cancellation,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryTerminalReason {
    NonRetryable,
    Cancelled,
    AttemptBudget,
    TokenBudget,
    SideEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDecision {
    Retry {
        attempt: u32,
        attempt_id: String,
        delay_ms: u64,
        reason: RetryFailure,
    },
    Terminal {
        reason: RetryTerminalReason,
    },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RetryError {
    #[error("retry policy or context is invalid")]
    InvalidContext,
    #[error("retry attempt identity is invalid")]
    InvalidAttemptId,
}

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    jitter_bps: u16,
}

impl RetryPolicy {
    pub fn new(
        max_attempts: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        jitter_bps: u16,
    ) -> Result<Self, RetryError> {
        if max_attempts == 0
            || base_delay_ms == 0
            || max_delay_ms < base_delay_ms
            || jitter_bps > 1_000
        {
            return Err(RetryError::InvalidContext);
        }
        Ok(Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
            jitter_bps,
        })
    }

    pub fn decide(&self, failure: &RetryFailure, context: &RetryContext) -> RetryDecision {
        if context.cancellation.is_cancelled() || *failure == RetryFailure::Cancelled {
            return RetryDecision::Terminal {
                reason: RetryTerminalReason::Cancelled,
            };
        }
        if matches!(
            context.operation,
            RetryOperationKind::Tool | RetryOperationKind::Destructive
        ) {
            return RetryDecision::Terminal {
                reason: RetryTerminalReason::SideEffect,
            };
        }
        if !failure.retryable() {
            return RetryDecision::Terminal {
                reason: RetryTerminalReason::NonRetryable,
            };
        }
        if context.attempts_used >= self.max_attempts {
            return RetryDecision::Terminal {
                reason: RetryTerminalReason::AttemptBudget,
            };
        }
        if context.tokens_used >= context.max_tokens {
            return RetryDecision::Terminal {
                reason: RetryTerminalReason::TokenBudget,
            };
        }
        let shift = context.attempts_used.min(20);
        let exponential = self.base_delay_ms.saturating_mul(1_u64 << shift);
        let delay_ms = exponential.min(self.max_delay_ms);
        let attempt = context.attempts_used + 1;
        let attempt_id = Self::attempt_id(&context.request_id, attempt)
            .unwrap_or_else(|_| "redacted_attempt".into());
        let _bounded_jitter = self.jitter_bps;
        RetryDecision::Retry {
            attempt,
            attempt_id,
            delay_ms,
            reason: *failure,
        }
    }

    pub fn attempt_id(request_id: &str, attempt: u32) -> Result<String, RetryError> {
        if !valid_request_id(request_id) || attempt == 0 {
            return Err(RetryError::InvalidAttemptId);
        }
        Ok(format!("{request_id}:attempt_{attempt}"))
    }
}

fn valid_request_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_REQUEST_ID_LEN
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.')
        })
}
