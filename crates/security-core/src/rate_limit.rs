//! Monotonic, bounded rate-limiting policy.
//!
//! The limiter owns policy state only; callers provide an authenticated scope
//! key and a monotonic timestamp. It never reads a wall clock, payload field,
//! credential or network identity. Recovery has its own finite bucket rather
//! than an exemption, and snapshots carry the policy revision so a stale
//! persisted window cannot silently authorize work.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Mutex;
use thiserror::Error;

pub const MAX_RATE_LIMIT_TEXT: usize = 128;
pub const MAX_RATE_LIMIT_KEYS: usize = 4_096;
pub const MAX_RATE_LIMIT_RECENT_REQUESTS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RateLimitScope {
    User,
    Project,
    Agent,
    Provider,
    Tool,
    Node,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RateLimitKey {
    scope: RateLimitScope,
    project_id: String,
    subject_id: String,
}

impl RateLimitKey {
    pub fn new(
        scope: RateLimitScope,
        project_id: impl Into<String>,
        subject_id: impl Into<String>,
    ) -> Result<Self, RateLimitError> {
        let project_id = project_id.into();
        let subject_id = subject_id.into();
        validate_text(&project_id)?;
        validate_text(&subject_id)?;
        Ok(Self {
            scope,
            project_id,
            subject_id,
        })
    }

    pub fn scope(&self) -> RateLimitScope {
        self.scope
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitClass {
    Normal,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    NonIdempotent,
    Idempotent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitPolicy {
    policy_revision: String,
    capacity: u64,
    refill_tokens: u64,
    window_ms: u64,
    recovery_capacity: u64,
    max_tracked_keys: usize,
    max_recent_requests: usize,
}

impl RateLimitPolicy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy_revision: impl Into<String>,
        capacity: u64,
        refill_tokens: u64,
        window_ms: u64,
        recovery_capacity: u64,
        max_tracked_keys: usize,
        max_recent_requests: usize,
    ) -> Result<Self, RateLimitError> {
        let policy_revision = policy_revision.into();
        validate_policy_text(&policy_revision)?;
        if capacity == 0
            || refill_tokens == 0
            || window_ms == 0
            || recovery_capacity == 0
            || recovery_capacity > capacity
            || max_tracked_keys == 0
            || max_tracked_keys > MAX_RATE_LIMIT_KEYS
            || max_recent_requests == 0
            || max_recent_requests > MAX_RATE_LIMIT_RECENT_REQUESTS
        {
            return Err(RateLimitError::InvalidPolicy);
        }
        Ok(Self {
            policy_revision,
            capacity,
            refill_tokens,
            window_ms,
            recovery_capacity,
            max_tracked_keys,
            max_recent_requests,
        })
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn recovery_capacity(&self) -> u64 {
        self.recovery_capacity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRequest {
    policy_revision: String,
    key: RateLimitKey,
    request_id: String,
    cost: u64,
    now_ms: u64,
    class: RateLimitClass,
    retry: RetryClass,
}

impl RateLimitRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy_revision: impl Into<String>,
        key: RateLimitKey,
        request_id: impl Into<String>,
        cost: u64,
        now_ms: u64,
        class: RateLimitClass,
        retry: RetryClass,
    ) -> Result<Self, RateLimitError> {
        let policy_revision = policy_revision.into();
        let request_id = request_id.into();
        validate_request_text(&policy_revision)?;
        validate_request_text(&request_id)?;
        if cost == 0 {
            return Err(RateLimitError::InvalidRequest);
        }
        Ok(Self {
            policy_revision,
            key,
            request_id,
            cost,
            now_ms,
            class,
            retry,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResponseClass {
    Allowed,
    RetryAfter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDenial {
    NormalExhausted,
    RecoveryExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allowed {
        charged: bool,
        remaining: u64,
        policy_revision: String,
    },
    Denied {
        reason: RateLimitDenial,
        retry_after_ms: u64,
        remaining: u64,
        policy_revision: String,
    },
}

impl RateLimitDecision {
    pub fn response_class(&self) -> RateLimitResponseClass {
        match self {
            Self::Allowed { .. } => RateLimitResponseClass::Allowed,
            Self::Denied { .. } => RateLimitResponseClass::RetryAfter,
        }
    }

    pub fn retry_after_ms(&self) -> u64 {
        match self {
            Self::Allowed { .. } => 0,
            Self::Denied { retry_after_ms, .. } => *retry_after_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RateLimitError {
    #[error("rate limit policy is invalid")]
    InvalidPolicy,
    #[error("rate limit request is invalid")]
    InvalidRequest,
    #[error("rate limit key is invalid")]
    InvalidKey,
    #[error("rate limit policy revision does not match")]
    PolicyRevisionMismatch,
    #[error("rate limit state is exhausted")]
    StateExhausted,
    #[error("rate limit clock moved backwards")]
    ClockRegression,
    #[error("rate limit request replay conflicts with prior request")]
    ReplayConflict,
    #[error("rate limit snapshot is invalid")]
    InvalidSnapshot,
    #[error("rate limit snapshot policy does not match")]
    SnapshotPolicyMismatch,
    #[error("rate limit state lock is unavailable")]
    StateUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RateLimitMetrics {
    pub policy_revision: String,
    pub window_ms: u64,
    pub allowed: u64,
    pub denied: u64,
    pub delayed: u64,
    pub idempotent_replays: u64,
    pub recovery_allowed: u64,
    pub recovery_denied: u64,
    pub tracked_keys: usize,
    pub saturated_keys: usize,
    pub remaining_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitRequestReceipt {
    pub request_id: String,
    pub cost: u64,
    pub class: RateLimitClass,
    pub retry: RetryClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitBucketSnapshot {
    pub key: RateLimitKey,
    pub normal_tokens: u64,
    pub recovery_tokens: u64,
    pub last_refill_ms: u64,
    pub refill_remainder: u64,
    pub recent_requests: Vec<RateLimitRequestReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitSnapshot {
    pub policy_revision: String,
    pub captured_at_ms: u64,
    pub buckets: Vec<RateLimitBucketSnapshot>,
}

#[derive(Debug, Clone)]
struct RecentRequest {
    request_id: String,
    cost: u64,
    class: RateLimitClass,
    retry: RetryClass,
}

#[derive(Debug, Clone)]
struct BucketState {
    normal_tokens: u64,
    recovery_tokens: u64,
    last_refill_ms: u64,
    refill_remainder: u64,
    recent_requests: VecDeque<RecentRequest>,
}

struct LimiterState {
    buckets: BTreeMap<RateLimitKey, BucketState>,
    metrics: RateLimitMetrics,
}

pub struct RateLimiter {
    policy: RateLimitPolicy,
    state: Mutex<LimiterState>,
}

impl RateLimiter {
    pub fn new(policy: RateLimitPolicy) -> Result<Self, RateLimitError> {
        Ok(Self {
            policy,
            state: Mutex::new(LimiterState {
                buckets: BTreeMap::new(),
                metrics: RateLimitMetrics::default(),
            }),
        })
    }

    pub fn policy(&self) -> &RateLimitPolicy {
        &self.policy
    }

    pub fn check(&self, request: RateLimitRequest) -> Result<RateLimitDecision, RateLimitError> {
        if request.policy_revision != self.policy.policy_revision {
            return Err(RateLimitError::PolicyRevisionMismatch);
        }
        let class_capacity = match request.class {
            RateLimitClass::Normal => self.policy.capacity,
            RateLimitClass::Recovery => self.policy.recovery_capacity,
        };
        if request.cost > class_capacity {
            return Err(RateLimitError::InvalidRequest);
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| RateLimitError::StateUnavailable)?;
        let key = request.key.clone();
        if !state.buckets.contains_key(&key) {
            if state.buckets.len() >= self.policy.max_tracked_keys {
                evict_inactive_buckets(&self.policy, &mut state.buckets, request.now_ms)?;
                if state.buckets.len() >= self.policy.max_tracked_keys {
                    return Err(RateLimitError::StateExhausted);
                }
            }
            state.buckets.insert(
                key.clone(),
                BucketState {
                    normal_tokens: self.policy.capacity,
                    recovery_tokens: self.policy.recovery_capacity,
                    last_refill_ms: request.now_ms,
                    refill_remainder: 0,
                    recent_requests: VecDeque::new(),
                },
            );
        }

        let decision = {
            let bucket = state
                .buckets
                .get_mut(&key)
                .ok_or(RateLimitError::StateUnavailable)?;
            refill_bucket(&self.policy, bucket, request.now_ms)?;

            if request.retry == RetryClass::Idempotent {
                if let Some(previous) = bucket
                    .recent_requests
                    .iter()
                    .find(|previous| previous.request_id == request.request_id)
                {
                    if previous.retry != RetryClass::Idempotent
                        || previous.cost != request.cost
                        || previous.class != request.class
                    {
                        return Err(RateLimitError::ReplayConflict);
                    }
                    let remaining = available_tokens(bucket, request.class);
                    Ok(RateLimitDecision::Allowed {
                        charged: false,
                        remaining,
                        policy_revision: self.policy.policy_revision.clone(),
                    })
                } else {
                    admit_request(&self.policy, bucket, &request)
                }
            } else {
                admit_request(&self.policy, bucket, &request)
            }
        }?;

        match &decision {
            RateLimitDecision::Allowed { charged: false, .. } => {
                state.metrics.idempotent_replays =
                    state.metrics.idempotent_replays.saturating_add(1);
            }
            RateLimitDecision::Allowed { charged: true, .. } => {
                state.metrics.allowed = state.metrics.allowed.saturating_add(1);
                if request.class == RateLimitClass::Recovery {
                    state.metrics.recovery_allowed =
                        state.metrics.recovery_allowed.saturating_add(1);
                }
            }
            RateLimitDecision::Denied { .. } => {
                state.metrics.denied = state.metrics.denied.saturating_add(1);
                state.metrics.delayed = state.metrics.delayed.saturating_add(1);
                if request.class == RateLimitClass::Recovery {
                    state.metrics.recovery_denied = state.metrics.recovery_denied.saturating_add(1);
                }
            }
        }
        Ok(decision)
    }

    pub fn metrics(&self) -> RateLimitMetrics {
        self.state
            .lock()
            .map(|state| {
                let mut metrics = state.metrics.clone();
                metrics.policy_revision = self.policy.policy_revision.clone();
                metrics.window_ms = self.policy.window_ms;
                metrics.tracked_keys = state.buckets.len();
                metrics.saturated_keys = state
                    .buckets
                    .values()
                    .filter(|bucket| bucket.normal_tokens == 0)
                    .count();
                metrics.remaining_tokens = state.buckets.values().fold(0_u64, |total, bucket| {
                    total
                        .saturating_add(bucket.normal_tokens)
                        .saturating_add(bucket.recovery_tokens)
                });
                metrics
            })
            .unwrap_or_default()
    }

    /// Resets one existing bucket at an operator-controlled monotonic point.
    /// It never creates state for an unknown key and rejects clock rollback.
    pub fn reset_window(&self, key: &RateLimitKey, now_ms: u64) -> Result<bool, RateLimitError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RateLimitError::StateUnavailable)?;
        let Some(bucket) = state.buckets.get_mut(key) else {
            return Ok(false);
        };
        if now_ms < bucket.last_refill_ms {
            return Err(RateLimitError::ClockRegression);
        }
        bucket.normal_tokens = self.policy.capacity;
        bucket.recovery_tokens = self.policy.recovery_capacity;
        bucket.last_refill_ms = now_ms;
        bucket.refill_remainder = 0;
        bucket.recent_requests.clear();
        Ok(true)
    }

    pub fn snapshot(&self, captured_at_ms: u64) -> Result<RateLimitSnapshot, RateLimitError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RateLimitError::StateUnavailable)?;
        let mut buckets = state.buckets.clone();
        for bucket in buckets.values_mut() {
            refill_bucket(&self.policy, bucket, captured_at_ms)?;
        }
        Ok(RateLimitSnapshot {
            policy_revision: self.policy.policy_revision.clone(),
            captured_at_ms,
            buckets: buckets
                .into_iter()
                .map(|(key, bucket)| RateLimitBucketSnapshot {
                    key,
                    normal_tokens: bucket.normal_tokens,
                    recovery_tokens: bucket.recovery_tokens,
                    last_refill_ms: bucket.last_refill_ms,
                    refill_remainder: bucket.refill_remainder,
                    recent_requests: bucket
                        .recent_requests
                        .into_iter()
                        .map(|request| RateLimitRequestReceipt {
                            request_id: request.request_id,
                            cost: request.cost,
                            class: request.class,
                            retry: request.retry,
                        })
                        .collect(),
                })
                .collect(),
        })
    }

    pub fn from_snapshot(
        policy: RateLimitPolicy,
        snapshot: RateLimitSnapshot,
        now_ms: u64,
    ) -> Result<Self, RateLimitError> {
        let limiter = Self::new(policy)?;
        limiter.restore(snapshot, now_ms)?;
        Ok(limiter)
    }

    pub fn restore(&self, snapshot: RateLimitSnapshot, now_ms: u64) -> Result<(), RateLimitError> {
        if snapshot.policy_revision != self.policy.policy_revision {
            return Err(RateLimitError::SnapshotPolicyMismatch);
        }
        if now_ms < snapshot.captured_at_ms || snapshot.buckets.len() > self.policy.max_tracked_keys
        {
            return Err(RateLimitError::InvalidSnapshot);
        }
        let mut keys = BTreeSet::new();
        let mut buckets = BTreeMap::new();
        for snapshot_bucket in snapshot.buckets {
            if !keys.insert(snapshot_bucket.key.clone())
                || snapshot_bucket.normal_tokens > self.policy.capacity
                || snapshot_bucket.recovery_tokens > self.policy.recovery_capacity
                || snapshot_bucket.last_refill_ms > snapshot.captured_at_ms
                || snapshot_bucket.refill_remainder >= self.policy.window_ms
                || snapshot_bucket.recent_requests.len() > self.policy.max_recent_requests
            {
                return Err(RateLimitError::InvalidSnapshot);
            }
            let mut recent_requests = VecDeque::new();
            for request in snapshot_bucket.recent_requests {
                if request.request_id.trim().is_empty()
                    || request.request_id.len() > MAX_RATE_LIMIT_TEXT
                    || request.request_id.chars().any(char::is_control)
                {
                    return Err(RateLimitError::InvalidSnapshot);
                }
                let class_capacity = match request.class {
                    RateLimitClass::Normal => self.policy.capacity,
                    RateLimitClass::Recovery => self.policy.recovery_capacity,
                };
                if request.cost == 0 || request.cost > class_capacity {
                    return Err(RateLimitError::InvalidSnapshot);
                }
                recent_requests.push_back(RecentRequest {
                    request_id: request.request_id,
                    cost: request.cost,
                    class: request.class,
                    retry: request.retry,
                });
            }
            let mut bucket = BucketState {
                normal_tokens: snapshot_bucket.normal_tokens,
                recovery_tokens: snapshot_bucket.recovery_tokens,
                last_refill_ms: snapshot_bucket.last_refill_ms,
                refill_remainder: snapshot_bucket.refill_remainder,
                recent_requests,
            };
            refill_bucket(&self.policy, &mut bucket, now_ms)?;
            buckets.insert(snapshot_bucket.key, bucket);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| RateLimitError::StateUnavailable)?;
        state.buckets = buckets;
        state.metrics.tracked_keys = state.buckets.len();
        Ok(())
    }
}

fn evict_inactive_buckets(
    policy: &RateLimitPolicy,
    buckets: &mut BTreeMap<RateLimitKey, BucketState>,
    now_ms: u64,
) -> Result<(), RateLimitError> {
    let mut refreshed = buckets.clone();
    for bucket in refreshed.values_mut() {
        refill_bucket(policy, bucket, now_ms)?;
    }
    refreshed.retain(|_, bucket| {
        !(bucket.normal_tokens == policy.capacity
            && bucket.recovery_tokens == policy.recovery_capacity
            && bucket.recent_requests.is_empty())
    });
    *buckets = refreshed;
    Ok(())
}

fn validate_text(value: &str) -> Result<(), RateLimitError> {
    if value.trim().is_empty()
        || value.len() > MAX_RATE_LIMIT_TEXT
        || value.chars().any(char::is_control)
    {
        return Err(RateLimitError::InvalidKey);
    }
    Ok(())
}

fn validate_policy_text(value: &str) -> Result<(), RateLimitError> {
    if value.trim().is_empty()
        || value.len() > MAX_RATE_LIMIT_TEXT
        || value.chars().any(char::is_control)
    {
        return Err(RateLimitError::InvalidPolicy);
    }
    Ok(())
}

fn validate_request_text(value: &str) -> Result<(), RateLimitError> {
    if value.trim().is_empty()
        || value.len() > MAX_RATE_LIMIT_TEXT
        || value.chars().any(char::is_control)
    {
        return Err(RateLimitError::InvalidRequest);
    }
    Ok(())
}

fn refill_bucket(
    policy: &RateLimitPolicy,
    bucket: &mut BucketState,
    now_ms: u64,
) -> Result<(), RateLimitError> {
    if now_ms < bucket.last_refill_ms {
        return Err(RateLimitError::ClockRegression);
    }
    let elapsed = now_ms - bucket.last_refill_ms;
    let numerator = u128::from(elapsed)
        .saturating_mul(u128::from(policy.refill_tokens))
        .saturating_add(u128::from(bucket.refill_remainder));
    let window = u128::from(policy.window_ms);
    let added = (numerator / window).min(u128::from(u64::MAX)) as u64;
    bucket.refill_remainder = (numerator % window) as u64;
    bucket.normal_tokens = bucket
        .normal_tokens
        .saturating_add(added)
        .min(policy.capacity);
    bucket.recovery_tokens = bucket
        .recovery_tokens
        .saturating_add(added)
        .min(policy.recovery_capacity);
    bucket.last_refill_ms = now_ms;
    Ok(())
}

fn available_tokens(bucket: &BucketState, class: RateLimitClass) -> u64 {
    match class {
        RateLimitClass::Normal => bucket.normal_tokens,
        RateLimitClass::Recovery => bucket.recovery_tokens,
    }
}

fn admit_request(
    policy: &RateLimitPolicy,
    bucket: &mut BucketState,
    request: &RateLimitRequest,
) -> Result<RateLimitDecision, RateLimitError> {
    let available = available_tokens(bucket, request.class);
    if available < request.cost {
        let reason = match request.class {
            RateLimitClass::Normal => RateLimitDenial::NormalExhausted,
            RateLimitClass::Recovery => RateLimitDenial::RecoveryExhausted,
        };
        return Ok(RateLimitDecision::Denied {
            reason,
            retry_after_ms: retry_after_ms(
                request.cost - available,
                policy.refill_tokens,
                policy.window_ms,
            ),
            remaining: available,
            policy_revision: policy.policy_revision.clone(),
        });
    }

    subtract_tokens(bucket, request.class, request.cost);
    bucket.recent_requests.push_back(RecentRequest {
        request_id: request.request_id.clone(),
        cost: request.cost,
        class: request.class,
        retry: request.retry,
    });
    while bucket.recent_requests.len() > policy.max_recent_requests {
        bucket.recent_requests.pop_front();
    }
    Ok(RateLimitDecision::Allowed {
        charged: true,
        remaining: available - request.cost,
        policy_revision: policy.policy_revision.clone(),
    })
}

fn subtract_tokens(bucket: &mut BucketState, class: RateLimitClass, cost: u64) {
    match class {
        RateLimitClass::Normal => bucket.normal_tokens -= cost,
        RateLimitClass::Recovery => bucket.recovery_tokens -= cost,
    }
}

fn retry_after_ms(deficit: u64, refill_tokens: u64, window_ms: u64) -> u64 {
    let numerator = u128::from(deficit).saturating_mul(u128::from(window_ms));
    let denominator = u128::from(refill_tokens);
    let rounded = numerator
        .saturating_add(denominator.saturating_sub(1))
        .checked_div(denominator)
        .unwrap_or(u128::from(u64::MAX));
    rounded.min(u128::from(u64::MAX)) as u64
}
