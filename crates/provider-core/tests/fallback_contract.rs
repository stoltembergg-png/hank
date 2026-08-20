use provider_core::capabilities::{
    CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilityRequirement, CapabilitySource,
    CapabilityState, ModelModality,
};
use provider_core::credentials::{AccountId, CredentialAccount, ProjectScopeId};
use provider_core::fallback::{
    FallbackCandidate, FallbackDecision, FallbackFailure, FallbackPolicy, FallbackReason,
    FallbackRequest, TerminalReason,
};
use provider_core::health::HealthStatus;
use provider_core::{CancellationToken, ModelId, ProviderId};
use std::collections::{BTreeMap, BTreeSet};

fn account(project: &str, provider: &str, id: &str) -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse(project).unwrap(),
        ProviderId::parse(provider).unwrap(),
        AccountId::parse(id).unwrap(),
    )
    .unwrap()
}

fn capabilities(provider: &str, model: &str, image: CapabilityState) -> CapabilityReport {
    CapabilityReport {
        schema_version: 1,
        provider_id: ProviderId::parse(provider).unwrap(),
        model_id: ModelId::parse(model).unwrap(),
        version: "1".into(),
        source: CapabilitySource::Provider,
        modalities: BTreeMap::from([
            (ModelModality::Text, CapabilityState::Supported),
            (ModelModality::Image, image),
        ]),
        features: BTreeMap::from([(CapabilityFeature::Streaming, CapabilityState::Supported)]),
        limits: CapabilityLimits {
            max_context_tokens: Some(32_768),
            max_output_tokens: Some(8_192),
        },
    }
}

fn candidate(provider: &str, model: &str, health: HealthStatus) -> FallbackCandidate {
    FallbackCandidate::new(
        account("project_1", provider, &format!("account_{provider}")),
        ModelId::parse(model).unwrap(),
        capabilities(provider, model, CapabilityState::Unsupported),
        health,
        512,
        100,
    )
    .unwrap()
}

fn request(reason: FallbackReason) -> FallbackRequest {
    FallbackRequest::new(
        "request_1",
        ProjectScopeId::parse("project_1").unwrap(),
        account("project_1", "provider_failed", "account_failed"),
        ProviderId::parse("provider_failed").unwrap(),
        ModelId::parse("model_failed").unwrap(),
        CapabilityRequirement {
            modalities: BTreeSet::from([ModelModality::Text]),
            features: BTreeSet::new(),
            min_context_tokens: Some(4_096),
            min_output_tokens: Some(256),
        },
        vec![],
        FallbackFailure::new(reason),
        0,
        0,
        0,
        CancellationToken::new(),
    )
    .unwrap()
}

fn policy() -> FallbackPolicy {
    FallbackPolicy::new(2, 1_000, 1_024).unwrap()
}

#[test]
fn retryable_matrix_selects_only_eligible_deterministic_alternative() {
    let mut request = request(FallbackReason::RateLimited);
    request.candidates = vec![
        candidate("provider-z", "model-z", HealthStatus::Healthy),
        candidate(
            "provider-disabled",
            "model-disabled",
            HealthStatus::Disabled,
        ),
        candidate("provider-outage", "model-outage", HealthStatus::Outage),
        candidate("provider-a", "model-a", HealthStatus::Healthy),
    ];
    let decision = policy().decide(request).unwrap();
    let FallbackDecision::Retry(attempt) = decision else {
        panic!("retryable failure should select an eligible alternative")
    };
    assert_eq!(attempt.provider_id.as_str(), "provider-a");
    assert_eq!(attempt.model_id.as_str(), "model-a");
    assert_eq!(attempt.attempt_number, 1);
    assert_eq!(attempt.attempt_id, "request_1:attempt_1");
}

#[test]
fn non_retryable_auth_and_invalid_request_terminate_without_fallback() {
    for reason in [
        FallbackReason::Authentication,
        FallbackReason::InvalidRequest,
    ] {
        let mut input = request(reason);
        input.candidates = vec![candidate("provider-a", "model-a", HealthStatus::Healthy)];
        let decision = policy().decide(input).unwrap();
        assert!(matches!(
            decision,
            FallbackDecision::Terminal(terminal) if terminal.reason == TerminalReason::NonRetryable
        ));
    }
}

#[test]
fn attempt_budget_prevents_extra_retry() {
    let mut input = request(FallbackReason::Timeout);
    input.attempts_used = 2;
    input.candidates = vec![candidate("provider-a", "model-a", HealthStatus::Healthy)];
    let decision = policy().decide(input).unwrap();
    assert!(matches!(
        decision,
        FallbackDecision::Terminal(terminal) if terminal.reason == TerminalReason::AttemptBudgetExhausted
    ));
}

#[test]
fn token_and_cost_budget_prevent_extra_attempt() {
    let mut input = request(FallbackReason::QuotaExceeded);
    input.tokens_used = 600;
    input.cost_used_micros = 950;
    input.candidates = vec![candidate("provider-a", "model-a", HealthStatus::Healthy)];
    let decision = policy().decide(input).unwrap();
    assert!(matches!(
        decision,
        FallbackDecision::Terminal(terminal) if terminal.reason == TerminalReason::BudgetExhausted
    ));
}

#[test]
fn project_scope_and_capability_mismatch_are_never_bypassed() {
    let mut input = request(FallbackReason::Outage);
    let mut wrong_scope = candidate("provider-a", "model-a", HealthStatus::Healthy);
    wrong_scope.account = account("project_2", "provider-a", "account_provider-a");
    let mut image_only = FallbackCandidate::new(
        account("project_1", "provider-b", "account_provider-b"),
        ModelId::parse("model-image").unwrap(),
        capabilities("provider-b", "model-image", CapabilityState::Supported),
        HealthStatus::Healthy,
        512,
        100,
    )
    .unwrap();
    image_only
        .capabilities
        .modalities
        .insert(ModelModality::Text, CapabilityState::Unsupported);
    input.candidates = vec![wrong_scope, image_only];
    let decision = policy().decide(input).unwrap();
    assert!(matches!(
        decision,
        FallbackDecision::Terminal(terminal) if terminal.reason == TerminalReason::NoEligibleAlternative
    ));
}

#[test]
fn cancellation_terminates_without_hidden_retry() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut input = request(FallbackReason::Timeout);
    input.cancellation = cancellation;
    input.candidates = vec![candidate("provider-a", "model-a", HealthStatus::Healthy)];
    let decision = policy().decide(input).unwrap();
    assert!(matches!(
        decision,
        FallbackDecision::Terminal(terminal) if terminal.reason == TerminalReason::Cancelled
    ));
}

#[test]
fn later_attempts_preserve_logical_identity_and_bounded_count() {
    let mut input = request(FallbackReason::Outage);
    input.attempts_used = 1;
    input.candidates = vec![candidate("provider-a", "model-a", HealthStatus::Healthy)];
    let decision = policy().decide(input).unwrap();
    let FallbackDecision::Retry(attempt) = decision else {
        panic!("second attempt should remain eligible")
    };
    assert_eq!(attempt.attempt_number, 2);
    assert_eq!(attempt.attempt_id, "request_1:attempt_2");
    assert!(attempt.attempt_id.len() <= 128);
}

#[test]
fn fallback_debug_contains_no_credential_material_or_raw_payload() {
    let mut input = request(FallbackReason::Timeout);
    input.candidates = vec![candidate("provider-a", "model-a", HealthStatus::Healthy)];
    let debug = format!("{input:?}");
    assert!(!debug.contains("cred_"));
    assert!(!debug.contains("api_key"));
    assert!(!debug.contains("secret"));
}
