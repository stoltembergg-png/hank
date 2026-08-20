//! Deterministic, bounded fallback decision policy.

use crate::capabilities::{CapabilityReport, CapabilityRequirement};
use crate::credentials::{CredentialAccount, ProjectScopeId};
use crate::health::HealthStatus;
use crate::{CancellationToken, ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thiserror::Error;

const MAX_ATTEMPTS: u32 = 8;
const MAX_CANDIDATES: usize = 64;
const MAX_LOGICAL_REQUEST_ID_LEN: usize = 128;
const MAX_TOTAL_TOKENS: u64 = 1_000_000;
const MAX_TOTAL_COST_MICROS: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackReason {
    RateLimited,
    Timeout,
    Outage,
    QuotaExceeded,
    Authentication,
    InvalidRequest,
    PolicyDenied,
    Unsupported,
}

impl FallbackReason {
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::Timeout | Self::Outage | Self::QuotaExceeded
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackFailure {
    pub reason: FallbackReason,
}

impl FallbackFailure {
    pub const fn new(reason: FallbackReason) -> Self {
        Self { reason }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackCandidate {
    pub account: CredentialAccount,
    pub model_id: ModelId,
    pub capabilities: CapabilityReport,
    pub health: HealthStatus,
    pub estimated_output_tokens: u32,
    pub estimated_cost_micros: u64,
}

impl FallbackCandidate {
    pub fn new(
        account: CredentialAccount,
        model_id: ModelId,
        capabilities: CapabilityReport,
        health: HealthStatus,
        estimated_output_tokens: u32,
        estimated_cost_micros: u64,
    ) -> Result<Self, FallbackError> {
        if capabilities.provider_id != account.provider_id
            || capabilities.model_id != model_id
            || estimated_output_tokens == 0
            || u64::from(estimated_output_tokens) > MAX_TOTAL_TOKENS
            || estimated_cost_micros > MAX_TOTAL_COST_MICROS
            || capabilities.validate().is_err()
        {
            return Err(FallbackError::InvalidRequest);
        }
        Ok(Self {
            account,
            model_id,
            capabilities,
            health,
            estimated_output_tokens,
            estimated_cost_micros,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FallbackRequest {
    pub logical_request_id: String,
    pub project_id: ProjectScopeId,
    pub failed_account: CredentialAccount,
    pub failed_provider_id: ProviderId,
    pub failed_model_id: ModelId,
    pub requirements: CapabilityRequirement,
    pub candidates: Vec<FallbackCandidate>,
    pub failure: FallbackFailure,
    pub attempts_used: u32,
    pub tokens_used: u64,
    pub cost_used_micros: u64,
    pub cancellation: CancellationToken,
}

impl FallbackRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        logical_request_id: impl Into<String>,
        project_id: ProjectScopeId,
        failed_account: CredentialAccount,
        failed_provider_id: ProviderId,
        failed_model_id: ModelId,
        requirements: CapabilityRequirement,
        candidates: Vec<FallbackCandidate>,
        failure: FallbackFailure,
        attempts_used: u32,
        tokens_used: u64,
        cost_used_micros: u64,
        cancellation: CancellationToken,
    ) -> Result<Self, FallbackError> {
        let logical_request_id = logical_request_id.into();
        if logical_request_id.trim().is_empty()
            || logical_request_id.len() > MAX_LOGICAL_REQUEST_ID_LEN
            || logical_request_id.chars().any(char::is_control)
            || failed_account.project_id != project_id
            || failed_account.provider_id != failed_provider_id
            || candidates.len() > MAX_CANDIDATES
            || attempts_used > MAX_ATTEMPTS
            || tokens_used > MAX_TOTAL_TOKENS
            || cost_used_micros > MAX_TOTAL_COST_MICROS
        {
            return Err(FallbackError::InvalidRequest);
        }
        Ok(Self {
            logical_request_id,
            project_id,
            failed_account,
            failed_provider_id,
            failed_model_id,
            requirements,
            candidates,
            failure,
            attempts_used,
            tokens_used,
            cost_used_micros,
            cancellation,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackPolicy {
    pub max_attempts: u32,
    pub max_total_tokens: u64,
    pub max_total_cost_micros: u64,
}

impl FallbackPolicy {
    pub fn new(
        max_attempts: u32,
        max_total_tokens: u64,
        max_total_cost_micros: u64,
    ) -> Result<Self, FallbackError> {
        if !(1..=MAX_ATTEMPTS).contains(&max_attempts)
            || !(1..=MAX_TOTAL_TOKENS).contains(&max_total_tokens)
            || !(1..=MAX_TOTAL_COST_MICROS).contains(&max_total_cost_micros)
        {
            return Err(FallbackError::InvalidRequest);
        }
        Ok(Self {
            max_attempts,
            max_total_tokens,
            max_total_cost_micros,
        })
    }

    pub fn decide(&self, request: FallbackRequest) -> Result<FallbackDecision, FallbackError> {
        if request.cancellation.is_cancelled() {
            return Ok(FallbackDecision::Terminal(FallbackTerminal {
                reason: TerminalReason::Cancelled,
                attempts_used: request.attempts_used,
            }));
        }
        if !request.failure.reason.is_retryable() {
            return Ok(FallbackDecision::Terminal(FallbackTerminal {
                reason: TerminalReason::NonRetryable,
                attempts_used: request.attempts_used,
            }));
        }
        if request.attempts_used >= self.max_attempts {
            return Ok(FallbackDecision::Terminal(FallbackTerminal {
                reason: TerminalReason::AttemptBudgetExhausted,
                attempts_used: request.attempts_used,
            }));
        }

        let mut candidates = request.candidates;
        candidates.sort_by(compare_candidates);
        let mut saw_budget_block = false;
        for candidate in candidates {
            if candidate.account.project_id != request.project_id
                || candidate.account.provider_id == request.failed_provider_id
                || candidate.health != HealthStatus::Healthy
                || candidate.capabilities.provider_id != candidate.account.provider_id
                || candidate.capabilities.model_id != candidate.model_id
                || candidate.capabilities.validate().is_err()
                || candidate
                    .capabilities
                    .check_compatibility(&request.requirements)
                    .is_err()
            {
                continue;
            }
            if request
                .tokens_used
                .saturating_add(u64::from(candidate.estimated_output_tokens))
                > self.max_total_tokens
                || request
                    .cost_used_micros
                    .saturating_add(candidate.estimated_cost_micros)
                    > self.max_total_cost_micros
            {
                saw_budget_block = true;
                continue;
            }
            let attempt_number = request.attempts_used + 1;
            return Ok(FallbackDecision::Retry(FallbackAttempt {
                attempt_id: format!("{}:attempt_{attempt_number}", request.logical_request_id),
                attempt_number,
                provider_id: candidate.account.provider_id.clone(),
                model_id: candidate.model_id,
                account: candidate.account,
            }));
        }

        Ok(FallbackDecision::Terminal(FallbackTerminal {
            reason: if saw_budget_block {
                TerminalReason::BudgetExhausted
            } else {
                TerminalReason::NoEligibleAlternative
            },
            attempts_used: request.attempts_used,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackAttempt {
    pub attempt_id: String,
    pub attempt_number: u32,
    pub provider_id: ProviderId,
    pub model_id: ModelId,
    pub account: CredentialAccount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalReason {
    NonRetryable,
    AttemptBudgetExhausted,
    BudgetExhausted,
    NoEligibleAlternative,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackTerminal {
    pub reason: TerminalReason,
    pub attempts_used: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackDecision {
    Retry(FallbackAttempt),
    Terminal(FallbackTerminal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FallbackError {
    #[error("fallback request or policy is invalid")]
    InvalidRequest,
}

fn compare_candidates(left: &FallbackCandidate, right: &FallbackCandidate) -> Ordering {
    left.account
        .provider_id
        .cmp(&right.account.provider_id)
        .then_with(|| left.model_id.as_str().cmp(right.model_id.as_str()))
        .then_with(|| left.account.account_id.cmp(&right.account.account_id))
}
