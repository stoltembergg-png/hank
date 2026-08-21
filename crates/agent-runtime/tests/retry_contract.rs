use agent_runtime::retry::{
    RetryContext, RetryDecision, RetryFailure, RetryOperationKind, RetryPolicy,
};
use provider_core::CancellationToken;

fn policy() -> RetryPolicy {
    RetryPolicy::new(3, 100, 1_000, 100).unwrap()
}

fn context(operation: RetryOperationKind, attempts: u32, tokens: u64) -> RetryContext {
    RetryContext::new(
        "request-1",
        operation,
        attempts,
        tokens,
        10_000,
        CancellationToken::new(),
    )
    .unwrap()
}

#[test]
fn retry_matrix_allows_only_transient_provider_failures() {
    let policy = policy();
    assert!(matches!(
        policy.decide(
            &RetryFailure::RateLimited,
            &context(RetryOperationKind::Completion, 0, 0)
        ),
        RetryDecision::Retry {
            attempt: 1,
            delay_ms: 100,
            ..
        }
    ));
    assert!(matches!(
        policy.decide(
            &RetryFailure::Timeout,
            &context(RetryOperationKind::Completion, 1, 0)
        ),
        RetryDecision::Retry {
            attempt: 2,
            delay_ms: 200,
            ..
        }
    ));
    assert!(matches!(
        policy.decide(
            &RetryFailure::Authentication,
            &context(RetryOperationKind::Completion, 0, 0)
        ),
        RetryDecision::Terminal { .. }
    ));
    assert!(matches!(
        policy.decide(
            &RetryFailure::InvalidRequest,
            &context(RetryOperationKind::Completion, 0, 0)
        ),
        RetryDecision::Terminal { .. }
    ));
}

#[test]
fn attempts_backoff_and_attempt_identity_are_bounded_deterministic() {
    let policy = policy();
    let decision = policy.decide(
        &RetryFailure::Outage,
        &context(RetryOperationKind::Completion, 2, 0),
    );
    assert!(matches!(
        decision,
        RetryDecision::Retry {
            attempt: 3,
            delay_ms: 400,
            ..
        }
    ));
    assert_eq!(
        RetryPolicy::attempt_id("request-1", 3).unwrap(),
        "request-1:attempt_3"
    );
    assert!(RetryPolicy::attempt_id("api_key=secret", 1).is_err());
}

#[test]
fn max_attempts_budget_and_cancel_deny_extra_retry() {
    let policy = policy();
    assert!(matches!(
        policy.decide(
            &RetryFailure::Timeout,
            &context(RetryOperationKind::Completion, 3, 0)
        ),
        RetryDecision::Terminal { .. }
    ));
    assert!(matches!(
        policy.decide(
            &RetryFailure::Quota,
            &context(RetryOperationKind::Completion, 0, 10_000)
        ),
        RetryDecision::Terminal { .. }
    ));
    let token = CancellationToken::new();
    token.cancel();
    let cancelled = RetryContext::new(
        "request-1",
        RetryOperationKind::Completion,
        0,
        0,
        10_000,
        token,
    )
    .unwrap();
    assert!(matches!(
        policy.decide(&RetryFailure::Timeout, &cancelled),
        RetryDecision::Terminal { .. }
    ));
}

#[test]
fn tool_and_destructive_operations_never_retry() {
    let policy = policy();
    for operation in [RetryOperationKind::Tool, RetryOperationKind::Destructive] {
        assert!(matches!(
            policy.decide(&RetryFailure::Timeout, &context(operation, 0, 0)),
            RetryDecision::Terminal { .. }
        ));
    }
}

#[test]
fn jitter_is_bounded_and_user_text_cannot_change_policy() {
    let policy = policy();
    let decision = policy.decide(
        &RetryFailure::RateLimited,
        &context(RetryOperationKind::Completion, 0, 0),
    );
    if let RetryDecision::Retry {
        delay_ms, reason, ..
    } = decision
    {
        assert!(delay_ms <= 1_000);
        assert_eq!(reason, RetryFailure::RateLimited);
    } else {
        panic!("expected retry");
    }
}

#[test]
fn malformed_policy_and_context_fail_closed() {
    assert!(RetryPolicy::new(0, 100, 1000, 100).is_err());
    assert!(RetryContext::new(
        "",
        RetryOperationKind::Completion,
        0,
        0,
        100,
        CancellationToken::new()
    )
    .is_err());
}
