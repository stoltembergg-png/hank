//! Deterministic, bounded token-bucket admission policy.
//!
//! This module does not authenticate callers, access a clock, persist state or
//! execute a request. Adapters must provide an already authenticated identity
//! and a monotonic timestamp, then enforce the returned decision before any
//! external effect.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use thiserror::Error;

pub const RATE_LIMIT_POLICY_SCHEMA_VERSION: u32 = 1;
pub const MAX_RATE_LIMIT_REVISION_LEN: usize = 128;
pub const MAX_RATE_LIMIT_ID_LEN: usize = 128;
pub const MAX_RATE_LIMIT_WINDOW_MS: u64 = 86_400_000;
pub const MAX_RATE_LIMIT_BURST: u64 = 1_000_000;
pub const MAX_RATE_LIMIT_KEYS: usize = 4_096;
pub const MAX_RATE_LIMIT_COST: u64 = 1_024;
pub const MAX_IDEMPOTENCY_KEYS_PER_BUCKET: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum RateLimitClass {
    Trigger,
    RemoteIngress,
    Recovery,
    Metrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct RateLimitIdentity {
    user_id: String,
    project_id: String,
    agent_id: Option<String>,
    provider_id: Option<String>,
    tool_id: Option<String>,
    node_id: Option<String>,
}

impl RateLimitIdentity {
    /// Creates an identity only after the caller has authenticated it.
    pub fn authenticated(
        user_id: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Result<Self, RateLimitError> {
        let identity = Self {
            user_id: user_id.into(),
            project_id: project_id.into(),
            agent_id: None,
            provider_id: None,
            tool_id: None,
            node_id: None,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn with_agent(mut self, value: impl Into<String>) -> Result<Self, RateLimitError> {
        self.agent_id = Some(value.into());
        self.validate()?;
        Ok(self)
    }

    pub fn with_provider(mut self, value: impl Into<String>) -> Result<Self, RateLimitError> {
        self.provider_id = Some(value.into());
        self.validate()?;
        Ok(self)
    }

    pub fn with_tool(mut self, value: impl Into<String>) -> Result<Self, RateLimitError> {
        self.tool_id = Some(value.into());
        self.validate()?;
        Ok(self)
    }

    pub fn with_node(mut self, value: impl Into<String>) -> Result<Self, RateLimitError> {
        self.node_id = Some(value.into());
        self.validate()?;
        Ok(self)
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    fn validate(&self) -> Result<(), RateLimitError> {
        validate_id(&self.user_id)?;
        validate_id(&self.project_id)?;
        for value in [
            self.agent_id.as_deref(),
            self.provider_id.as_deref(),
            self.tool_id.as_deref(),
            self.node_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_id(value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitPolicy {
    schema_version: u32,
    policy_revision: String,
    window_ms: u64,
    burst: u64,
    max_keys: usize,
}

impl RateLimitPolicy {
    pub fn new(
        policy_revision: impl Into<String>,
        window_ms: u64,
        burst: u64,
        max_keys: usize,
    ) -> Result<Self, RateLimitError> {
        let policy = Self {
            schema_version: RATE_LIMIT_POLICY_SCHEMA_VERSION,
            policy_revision: policy_revision.into(),
            window_ms,
            burst,
            max_keys,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn window_ms(&self) -> u64 {
        self.window_ms
    }

    pub fn burst(&self) -> u64 {
        self.burst
    }

    pub fn max_keys(&self) -> usize {
        self.max_keys
    }

    pub fn validate(&self) -> Result<(), RateLimitError> {
        if self.schema_version != RATE_LIMIT_POLICY_SCHEMA_VERSION
            || self.policy_revision.trim().is_empty()
            || self.policy_revision.len() > MAX_RATE_LIMIT_REVISION_LEN
            || self.policy_revision.chars().any(char::is_control)
            || self.window_ms == 0
            || self.window_ms > MAX_RATE_LIMIT_WINDOW_MS
            || self.burst == 0
            || self.burst > MAX_RATE_LIMIT_BURST
            || self.max_keys == 0
            || self.max_keys > MAX_RATE_LIMIT_KEYS
        {
            return Err(RateLimitError::InvalidPolicy);
        }
        Ok(())
    }
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self::new("rate-v1", 60_000, 60, 256).expect("default rate policy is valid")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRequest {
    identity: RateLimitIdentity,
    class: RateLimitClass,
    cost: u64,
    policy_revision: String,
    idempotency_key: Option<String>,
}

impl RateLimitRequest {
    pub fn new(
        identity: RateLimitIdentity,
        class: RateLimitClass,
        cost: u64,
        policy_revision: impl Into<String>,
        idempotency_key: Option<String>,
    ) -> Result<Self, RateLimitError> {
        let request = Self {
            identity,
            class,
            cost,
            policy_revision: policy_revision.into(),
            idempotency_key,
        };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), RateLimitError> {
        self.identity.validate()?;
        if self.cost == 0
            || self.cost > MAX_RATE_LIMIT_COST
            || self.policy_revision.trim().is_empty()
            || self.policy_revision.len() > MAX_RATE_LIMIT_REVISION_LEN
            || self.policy_revision.chars().any(char::is_control)
        {
            return Err(RateLimitError::InvalidRequest);
        }
        if let Some(key) = &self.idempotency_key {
            validate_id(key)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitReason {
    Admitted,
    IdempotentRetry,
    BurstExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allowed {
        policy_revision: String,
        reason: RateLimitReason,
        remaining: u64,
        retry_after_ms: u64,
    },
    Duplicate {
        policy_revision: String,
        reason: RateLimitReason,
        remaining: u64,
        retry_after_ms: u64,
    },
    Denied {
        policy_revision: String,
        reason: RateLimitReason,
        remaining: u64,
        retry_after_ms: u64,
    },
}

impl RateLimitDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. } | Self::Duplicate { .. })
    }

    pub fn retry_after_ms(&self) -> u64 {
        match self {
            Self::Allowed { retry_after_ms, .. }
            | Self::Duplicate { retry_after_ms, .. }
            | Self::Denied { retry_after_ms, .. } => *retry_after_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RateLimitError {
    #[error("rate limit policy is invalid")]
    InvalidPolicy,
    #[error("rate limit identity or request is invalid")]
    InvalidRequest,
    #[error("rate limit policy revision does not match")]
    PolicyRevisionMismatch,
    #[error("rate limit clock moved backwards")]
    ClockWentBackwards,
    #[error("rate limit state capacity was exceeded")]
    StateCapacityExceeded,
    #[error("rate limit state is unavailable")]
    StateUnavailable,
    #[error("rate limit arithmetic overflow")]
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct BucketKey {
    identity: RateLimitIdentity,
    class: RateLimitClass,
}

#[derive(Debug)]
struct BucketState {
    tokens: u64,
    last_refill_ms: u64,
    seen_requests: BTreeSet<String>,
}

/// Thread-safe bounded state for one policy revision.
pub struct RateLimiter {
    policy: RateLimitPolicy,
    buckets: Mutex<BTreeMap<BucketKey, BucketState>>,
}

impl RateLimiter {
    pub fn new(policy: RateLimitPolicy) -> Self {
        debug_assert!(policy.validate().is_ok());
        Self {
            policy,
            buckets: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn policy(&self) -> &RateLimitPolicy {
        &self.policy
    }

    pub fn check(
        &self,
        request: RateLimitRequest,
        now_ms: u64,
    ) -> Result<RateLimitDecision, RateLimitError> {
        request.validate()?;
        if request.policy_revision != self.policy.policy_revision {
            return Err(RateLimitError::PolicyRevisionMismatch);
        }
        let key = BucketKey {
            identity: request.identity,
            class: request.class,
        };
        let mut buckets = self
            .buckets
            .lock()
            .map_err(|_| RateLimitError::StateUnavailable)?;
        if !buckets.contains_key(&key) && buckets.len() >= self.policy.max_keys {
            return Err(RateLimitError::StateCapacityExceeded);
        }
        let bucket = buckets.entry(key).or_insert_with(|| BucketState {
            tokens: self.policy.burst,
            last_refill_ms: now_ms,
            seen_requests: BTreeSet::new(),
        });
        if now_ms < bucket.last_refill_ms {
            return Err(RateLimitError::ClockWentBackwards);
        }
        refill(bucket, &self.policy, now_ms)?;

        if let Some(request_id) = request.idempotency_key.as_deref() {
            if bucket.seen_requests.contains(request_id) {
                return Ok(RateLimitDecision::Duplicate {
                    policy_revision: self.policy.policy_revision.clone(),
                    reason: RateLimitReason::IdempotentRetry,
                    remaining: bucket.tokens,
                    retry_after_ms: 0,
                });
            }
            if bucket.seen_requests.len() >= MAX_IDEMPOTENCY_KEYS_PER_BUCKET {
                return Err(RateLimitError::StateCapacityExceeded);
            }
        }

        if bucket.tokens < request.cost {
            return Ok(RateLimitDecision::Denied {
                policy_revision: self.policy.policy_revision.clone(),
                reason: RateLimitReason::BurstExhausted,
                remaining: bucket.tokens,
                retry_after_ms: retry_after_ms(bucket.tokens, request.cost, &self.policy)?,
            });
        }
        bucket.tokens -= request.cost;
        if let Some(request_id) = request.idempotency_key {
            bucket.seen_requests.insert(request_id);
        }
        Ok(RateLimitDecision::Allowed {
            policy_revision: self.policy.policy_revision.clone(),
            reason: RateLimitReason::Admitted,
            remaining: bucket.tokens,
            retry_after_ms: 0,
        })
    }
}

fn refill(
    bucket: &mut BucketState,
    policy: &RateLimitPolicy,
    now_ms: u64,
) -> Result<(), RateLimitError> {
    let elapsed = now_ms
        .checked_sub(bucket.last_refill_ms)
        .ok_or(RateLimitError::ClockWentBackwards)?;
    if elapsed == 0 {
        return Ok(());
    }
    let produced = (u128::from(elapsed) * u128::from(policy.burst)) / u128::from(policy.window_ms);
    let produced = u64::try_from(produced).map_err(|_| RateLimitError::ArithmeticOverflow)?;
    bucket.tokens = bucket.tokens.saturating_add(produced).min(policy.burst);
    if produced > 0 || bucket.tokens == policy.burst {
        bucket.last_refill_ms = now_ms;
    }
    Ok(())
}

fn retry_after_ms(
    available: u64,
    cost: u64,
    policy: &RateLimitPolicy,
) -> Result<u64, RateLimitError> {
    let needed = u128::from(cost - available);
    let numerator = needed
        .checked_mul(u128::from(policy.window_ms))
        .ok_or(RateLimitError::ArithmeticOverflow)?;
    let retry = numerator
        .checked_add(u128::from(policy.burst - 1))
        .ok_or(RateLimitError::ArithmeticOverflow)?
        / u128::from(policy.burst);
    u64::try_from(retry).map_err(|_| RateLimitError::ArithmeticOverflow)
}

fn validate_id(value: &str) -> Result<(), RateLimitError> {
    if value.trim().is_empty()
        || value.len() > MAX_RATE_LIMIT_ID_LEN
        || value.chars().any(char::is_control)
    {
        return Err(RateLimitError::InvalidRequest);
    }
    Ok(())
}
