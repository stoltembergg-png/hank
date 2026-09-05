use security_core::rate_limit::{
    RateLimitClass, RateLimitDecision, RateLimitError, RateLimitIdentity, RateLimitPolicy,
    RateLimitReason, RateLimitRequest, RateLimiter,
};

fn policy() -> RateLimitPolicy {
    RateLimitPolicy::new("rate-v1", 100, 2, 8).unwrap()
}

fn identity(project: &str) -> RateLimitIdentity {
    RateLimitIdentity::authenticated("user-a", project)
        .unwrap()
        .with_agent("agent-a")
        .unwrap()
}

fn request(
    identity: RateLimitIdentity,
    class: RateLimitClass,
    request_id: Option<&str>,
) -> RateLimitRequest {
    RateLimitRequest::new(identity, class, 1, "rate-v1", request_id.map(str::to_owned)).unwrap()
}

#[test]
// @spec:AC-2001
fn burst_refill_is_bounded_and_clock_is_monotonic() {
    let limiter = RateLimiter::new(policy());
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Trigger, None),
            0
        ),
        Ok(RateLimitDecision::Allowed { remaining: 1, .. })
    ));
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Trigger, None),
            0
        ),
        Ok(RateLimitDecision::Allowed { remaining: 0, .. })
    ));
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Trigger, None),
            0
        ),
        Ok(RateLimitDecision::Denied {
            reason: RateLimitReason::BurstExhausted,
            retry_after_ms: 50,
            ..
        })
    ));
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Trigger, None),
            100
        ),
        Ok(RateLimitDecision::Allowed { remaining: 1, .. })
    ));
    assert_eq!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Trigger, None),
            99
        ),
        Err(RateLimitError::ClockWentBackwards)
    );
}

#[test]
// @spec:AC-2002
fn project_and_scope_identity_cannot_share_quota_or_change_policy_revision() {
    let limiter = RateLimiter::new(policy());
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Trigger, None),
            0
        ),
        Ok(RateLimitDecision::Allowed { .. })
    ));
    assert!(matches!(
        limiter.check(
            request(identity("project-b"), RateLimitClass::Trigger, None),
            0
        ),
        Ok(RateLimitDecision::Allowed { remaining: 1, .. })
    ));
    let wrong_revision = RateLimitRequest::new(
        identity("project-a"),
        RateLimitClass::Trigger,
        1,
        "stale-policy",
        None,
    )
    .unwrap();
    assert_eq!(
        limiter.check(wrong_revision, 0),
        Err(RateLimitError::PolicyRevisionMismatch)
    );
}

#[test]
// @spec:AC-2003
fn retries_are_idempotent_and_recovery_has_a_finite_bucket() {
    let limiter = RateLimiter::new(policy());
    let first = request(
        identity("project-a"),
        RateLimitClass::Recovery,
        Some("retry-1"),
    );
    assert!(matches!(
        limiter.check(first.clone(), 0),
        Ok(RateLimitDecision::Allowed { remaining: 1, .. })
    ));
    assert!(matches!(
        limiter.check(first, 0),
        Ok(RateLimitDecision::Duplicate {
            reason: RateLimitReason::IdempotentRetry,
            remaining: 1,
            ..
        })
    ));
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Recovery, None),
            0
        ),
        Ok(RateLimitDecision::Allowed { remaining: 0, .. })
    ));
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Recovery, None),
            0
        ),
        Ok(RateLimitDecision::Denied { .. })
    ));
}

#[test]
// @spec:AC-2003
fn idempotency_keys_expire_after_the_replay_window() {
    let limiter = RateLimiter::new(policy());
    let first = request(
        identity("project-a"),
        RateLimitClass::Recovery,
        Some("retry-window"),
    );
    assert!(matches!(
        limiter.check(first.clone(), 0),
        Ok(RateLimitDecision::Allowed { .. })
    ));
    assert!(matches!(
        limiter.check(first.clone(), 99),
        Ok(RateLimitDecision::Duplicate { .. })
    ));
    assert!(matches!(
        limiter.check(first, 100),
        Ok(RateLimitDecision::Allowed { .. })
    ));
}

#[test]
// @spec:AC-2003
fn refill_preserves_fractional_credit_between_checks() {
    let limiter = RateLimiter::new(RateLimitPolicy::new("rate-v1", 100, 3, 8).unwrap());
    for now_ms in [0, 34, 68, 100] {
        assert!(matches!(
            limiter.check(
                request(identity("project-a"), RateLimitClass::Trigger, None),
                now_ms
            ),
            Ok(RateLimitDecision::Allowed { .. })
        ));
    }
}

#[test]
// @spec:AC-2003
fn metrics_are_bounded_not_an_unlimited_exemption() {
    let limiter = RateLimiter::new(policy());
    for _ in 0..2 {
        assert!(matches!(
            limiter.check(
                request(identity("project-a"), RateLimitClass::Metrics, None),
                0
            ),
            Ok(RateLimitDecision::Allowed { .. })
        ));
    }
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Metrics, None),
            0
        ),
        Ok(RateLimitDecision::Denied { .. })
    ));
}

#[test]
// @spec:AC-2005
fn state_capacity_and_invalid_cost_fail_closed() {
    let small = RateLimitPolicy::new("rate-v1", 100, 2, 1).unwrap();
    let limiter = RateLimiter::new(small);
    assert!(matches!(
        limiter.check(
            request(identity("project-a"), RateLimitClass::Trigger, None),
            0
        ),
        Ok(RateLimitDecision::Allowed { .. })
    ));
    assert_eq!(
        limiter.check(
            request(identity("project-b"), RateLimitClass::Trigger, None),
            0
        ),
        Err(RateLimitError::StateCapacityExceeded)
    );
    let invalid = RateLimitRequest::new(
        identity("project-a"),
        RateLimitClass::Trigger,
        0,
        "rate-v1",
        None,
    );
    assert_eq!(invalid, Err(RateLimitError::InvalidRequest));
}
