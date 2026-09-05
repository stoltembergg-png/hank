use security_core::{
    RateLimitClass, RateLimitDecision, RateLimitDenial, RateLimitError, RateLimitKey,
    RateLimitPolicy, RateLimitRequest, RateLimitResponseClass, RateLimitScope, RateLimiter,
    RetryClass,
};

fn policy() -> RateLimitPolicy {
    RateLimitPolicy::new("policy-1", 3, 3, 1_000, 1, 8, 4).unwrap()
}

fn key(scope: RateLimitScope, project: &str, subject: &str) -> RateLimitKey {
    RateLimitKey::new(scope, project, subject).unwrap()
}

fn request(
    policy: &str,
    key: RateLimitKey,
    request_id: &str,
    now_ms: u64,
    class: RateLimitClass,
    retry: RetryClass,
) -> RateLimitRequest {
    RateLimitRequest::new(policy, key, request_id, 1, now_ms, class, retry).unwrap()
}

#[test]
// @spec:AC-2571
fn token_bucket_enforces_burst_window_and_explicit_retry_after() {
    let limiter = RateLimiter::new(policy()).unwrap();
    let project_a = key(RateLimitScope::Project, "project-a", "project-a");

    for id in ["one", "two", "three"] {
        assert!(matches!(
            limiter.check(request(
                "policy-1",
                project_a.clone(),
                id,
                0,
                RateLimitClass::Normal,
                RetryClass::NonIdempotent,
            )),
            Ok(RateLimitDecision::Allowed { charged: true, .. })
        ));
    }

    let denied = limiter
        .check(request(
            "policy-1",
            project_a.clone(),
            "four",
            0,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        ))
        .unwrap();
    assert_eq!(denied.response_class(), RateLimitResponseClass::RetryAfter);
    assert_eq!(
        denied,
        RateLimitDecision::Denied {
            reason: RateLimitDenial::NormalExhausted,
            retry_after_ms: 334,
            remaining: 0,
            policy_revision: "policy-1".into(),
        }
    );

    assert!(matches!(
        limiter.check(request(
            "policy-1",
            project_a,
            "five",
            334,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Ok(RateLimitDecision::Allowed { charged: true, .. })
    ));
}

#[test]
// @spec:AC-2572
fn scope_keys_are_isolated_and_policy_revision_is_fail_closed() {
    let limiter = RateLimiter::new(policy()).unwrap();
    let project_a = key(RateLimitScope::Project, "project-a", "project-a");
    let project_b = key(RateLimitScope::Project, "project-b", "project-b");

    for id in ["one", "two", "three"] {
        limiter
            .check(request(
                "policy-1",
                project_a.clone(),
                id,
                0,
                RateLimitClass::Normal,
                RetryClass::NonIdempotent,
            ))
            .unwrap();
    }
    assert!(matches!(
        limiter.check(request(
            "policy-1",
            project_b,
            "one",
            0,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Ok(RateLimitDecision::Allowed { .. })
    ));

    assert_eq!(
        limiter.check(request(
            "policy-0",
            project_a,
            "four",
            0,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Err(RateLimitError::PolicyRevisionMismatch)
    );
}

#[test]
// @spec:AC-2573
fn idempotent_retry_is_not_charged_but_non_idempotent_retry_is_bounded() {
    let limiter = RateLimiter::new(policy()).unwrap();
    let key = key(RateLimitScope::Agent, "project-a", "agent-a");

    assert!(matches!(
        limiter.check(request(
            "policy-1",
            key.clone(),
            "retry-1",
            0,
            RateLimitClass::Normal,
            RetryClass::Idempotent,
        )),
        Ok(RateLimitDecision::Allowed { charged: true, .. })
    ));
    assert!(matches!(
        limiter.check(request(
            "policy-1",
            key.clone(),
            "retry-1",
            0,
            RateLimitClass::Normal,
            RetryClass::Idempotent,
        )),
        Ok(RateLimitDecision::Allowed { charged: false, .. })
    ));

    for id in ["retry-2", "retry-3"] {
        limiter
            .check(request(
                "policy-1",
                key.clone(),
                id,
                0,
                RateLimitClass::Normal,
                RetryClass::NonIdempotent,
            ))
            .unwrap();
    }
    assert!(matches!(
        limiter.check(request(
            "policy-1",
            key,
            "retry-4",
            0,
            RateLimitClass::Normal,
            RetryClass::Idempotent,
        )),
        Ok(RateLimitDecision::Denied {
            reason: RateLimitDenial::NormalExhausted,
            ..
        })
    ));

    let conflict_key = RateLimitKey::new(RateLimitScope::Tool, "project-a", "tool-a").unwrap();
    limiter
        .check(request(
            "policy-1",
            conflict_key.clone(),
            "same-id",
            0,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        ))
        .unwrap();
    assert_eq!(
        limiter.check(request(
            "policy-1",
            conflict_key,
            "same-id",
            0,
            RateLimitClass::Normal,
            RetryClass::Idempotent,
        )),
        Err(RateLimitError::ReplayConflict)
    );
}

#[test]
// @spec:AC-2574
fn recovery_has_a_separate_but_bounded_budget_and_never_bypasses_limits() {
    let limiter = RateLimiter::new(policy()).unwrap();
    let key = key(RateLimitScope::Node, "project-a", "node-a");

    for class in [RateLimitClass::Normal, RateLimitClass::Recovery] {
        assert!(matches!(
            limiter.check(request(
                "policy-1",
                key.clone(),
                if class == RateLimitClass::Normal {
                    "normal"
                } else {
                    "recovery-1"
                },
                0,
                class,
                RetryClass::NonIdempotent,
            )),
            Ok(RateLimitDecision::Allowed { .. })
        ));
    }
    assert!(matches!(
        limiter.check(request(
            "policy-1",
            key,
            "recovery-2",
            0,
            RateLimitClass::Recovery,
            RetryClass::NonIdempotent,
        )),
        Ok(RateLimitDecision::Denied {
            reason: RateLimitDenial::RecoveryExhausted,
            ..
        })
    ));
}

#[test]
// @spec:AC-2575
fn clock_regression_snapshot_restore_and_state_bounds_fail_closed() {
    let limiter = RateLimiter::new(policy()).unwrap();
    let tool_key = key(RateLimitScope::Tool, "project-a", "tool-a");
    limiter
        .check(request(
            "policy-1",
            tool_key.clone(),
            "one",
            100,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        ))
        .unwrap();
    assert_eq!(
        limiter.check(request(
            "policy-1",
            tool_key.clone(),
            "two",
            99,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Err(RateLimitError::ClockRegression)
    );

    let snapshot = limiter.snapshot(100).unwrap();
    let restored = RateLimiter::from_snapshot(policy(), snapshot, 433).unwrap();
    assert!(matches!(
        restored.check(request(
            "policy-1",
            tool_key,
            "two",
            433,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Ok(RateLimitDecision::Allowed { .. })
    ));

    let bounded = RateLimitPolicy::new("policy-1", 1, 1, 1_000, 1, 1, 1).unwrap();
    let bounded = RateLimiter::new(bounded).unwrap();
    bounded
        .check(request(
            "policy-1",
            key(RateLimitScope::User, "project-a", "user-a"),
            "one",
            0,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        ))
        .unwrap();
    assert_eq!(
        bounded.check(request(
            "policy-1",
            key(RateLimitScope::User, "project-b", "user-b"),
            "two",
            0,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Err(RateLimitError::StateExhausted)
    );
}

#[test]
// @spec:AC-2576
fn metrics_are_bounded_and_do_not_echo_scope_values() {
    let limiter = RateLimiter::new(policy()).unwrap();
    let key = key(RateLimitScope::Provider, "secret-project", "provider-a");
    for id in ["one", "two", "three", "four"] {
        let _ = limiter.check(request(
            "policy-1",
            key.clone(),
            id,
            0,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        ));
    }
    let metrics = limiter.metrics();
    assert_eq!(metrics.allowed, 3);
    assert_eq!(metrics.denied, 1);
    assert_eq!(metrics.delayed, 1);
    assert_eq!(metrics.remaining_tokens, 1);
    assert_eq!(metrics.saturated_keys, 1);
    assert_eq!(metrics.window_ms, 1_000);
    assert_eq!(metrics.policy_revision, "policy-1");
    assert_eq!(metrics.tracked_keys, 1);
    assert!(!format!("{metrics:?}").contains("secret-project"));
    assert!(!format!("{metrics:?}").contains("provider-a"));
}

#[test]
// @spec:AC-2575
fn reset_window_is_explicit_bounded_and_does_not_create_unknown_state() {
    let limiter = RateLimiter::new(policy()).unwrap();
    let project_a = key(RateLimitScope::Project, "project-a", "project-a");
    for id in ["one", "two", "three"] {
        limiter
            .check(request(
                "policy-1",
                project_a.clone(),
                id,
                100,
                RateLimitClass::Normal,
                RetryClass::NonIdempotent,
            ))
            .unwrap();
    }
    assert!(matches!(
        limiter.check(request(
            "policy-1",
            project_a.clone(),
            "four",
            100,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Ok(RateLimitDecision::Denied { .. })
    ));
    assert_eq!(limiter.reset_window(&project_a, 100), Ok(true));
    assert!(matches!(
        limiter.check(request(
            "policy-1",
            project_a.clone(),
            "five",
            100,
            RateLimitClass::Normal,
            RetryClass::NonIdempotent,
        )),
        Ok(RateLimitDecision::Allowed { .. })
    ));
    let project_b = key(RateLimitScope::Project, "project-b", "project-b");
    assert_eq!(limiter.reset_window(&project_b, 100), Ok(false));
    assert_eq!(
        limiter.reset_window(&project_a, 99),
        Err(RateLimitError::ClockRegression)
    );
}
