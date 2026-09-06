use provider_core::capabilities::{
    CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilityRequirement, CapabilitySource,
    CapabilityState, ModelModality,
};
use provider_core::fallback::{FallbackCandidate, FallbackFailure, FallbackReason};
use provider_core::health::HealthStatus;
use provider_core::{ModelId, ProviderId};
use std::collections::{BTreeMap, BTreeSet};

fn report(provider: &str, model: &str, streaming: CapabilityState) -> CapabilityReport {
    let provider_id = ProviderId::parse(provider).unwrap();
    let model_id = ModelId::parse(model).unwrap();
    CapabilityReport {
        schema_version: provider_core::capabilities::MODEL_CAPABILITY_SCHEMA_VERSION,
        provider_id,
        model_id,
        version: "fixture-1".into(),
        source: CapabilitySource::Provider,
        modalities: BTreeMap::from([(ModelModality::Text, CapabilityState::Supported)]),
        features: BTreeMap::from([
            (CapabilityFeature::Streaming, streaming),
            (CapabilityFeature::ToolUse, CapabilityState::Supported),
        ]),
        limits: CapabilityLimits {
            max_context_tokens: Some(128_000),
            max_output_tokens: Some(8_192),
        },
    }
}

// @spec:AC-2652
#[test]
fn supported_contract_accepts_complete_stream_and_tools() {
    let capabilities = report(
        "fixture-openai",
        "fixture-model",
        CapabilityState::Supported,
    );
    let requirement = CapabilityRequirement {
        modalities: BTreeSet::from([ModelModality::Text]),
        features: BTreeSet::from([CapabilityFeature::Streaming, CapabilityFeature::ToolUse]),
        min_context_tokens: Some(4_096),
        min_output_tokens: Some(512),
    };
    assert!(capabilities.validate().is_ok());
    assert!(capabilities.check_compatibility(&requirement).is_ok());
}

// @spec:AC-2652
#[test]
fn unsupported_capability_is_explicit_and_fail_closed() {
    let capabilities = report(
        "fixture-anthropic",
        "fixture-model",
        CapabilityState::Unsupported,
    );
    let requirement = CapabilityRequirement {
        features: BTreeSet::from([CapabilityFeature::Streaming]),
        ..CapabilityRequirement::default()
    };
    assert!(matches!(
        capabilities.check_compatibility(&requirement),
        Err(
            provider_core::capabilities::CapabilityError::UnsupportedFeature(
                CapabilityFeature::Streaming
            )
        )
    ));
}

// @spec:AC-2652
#[test]
fn unknown_capability_is_not_treated_as_supported() {
    let capabilities = report("fixture-gemini", "fixture-model", CapabilityState::Unknown);
    let requirement = CapabilityRequirement {
        features: BTreeSet::from([CapabilityFeature::Streaming]),
        ..CapabilityRequirement::default()
    };
    assert!(matches!(
        capabilities.check_compatibility(&requirement),
        Err(
            provider_core::capabilities::CapabilityError::UnknownFeature(
                CapabilityFeature::Streaming
            )
        )
    ));
}

// @spec:AC-2653
#[test]
fn fallback_reason_classification_is_bounded_and_policy_preserving() {
    assert!(FallbackReason::RateLimited.is_retryable());
    assert!(FallbackReason::Timeout.is_retryable());
    assert!(!FallbackReason::Authentication.is_retryable());
    assert!(!FallbackReason::InvalidRequest.is_retryable());
    let _failure = FallbackFailure::new(FallbackReason::RateLimited);
}

// @spec:AC-2651
#[test]
fn provider_and_model_identity_must_match_fixture() {
    let account = provider_core::credentials::CredentialAccount::new(
        provider_core::credentials::ProjectScopeId::parse("project_1").unwrap(),
        ProviderId::parse("fixture-openai").unwrap(),
        provider_core::credentials::AccountId::parse("account_fixture").unwrap(),
    )
    .unwrap();
    let capabilities = report(
        "fixture-openai",
        "fixture-model",
        CapabilityState::Supported,
    );
    assert!(FallbackCandidate::new(
        account.clone(),
        ModelId::parse("fixture-model").unwrap(),
        capabilities,
        HealthStatus::Healthy,
        128,
        100,
    )
    .is_ok());
    let mismatch = report("fixture-openai", "other-model", CapabilityState::Supported);
    assert!(FallbackCandidate::new(
        account,
        ModelId::parse("fixture-model").unwrap(),
        mismatch,
        HealthStatus::Healthy,
        128,
        100,
    )
    .is_err());
}

// @spec:AC-2654
#[test]
fn fixture_contract_never_requires_live_credentials_or_network() {
    let serialized = serde_json::to_string(&report(
        "fixture-ollama",
        "fixture-model",
        CapabilityState::Supported,
    ))
    .unwrap();
    assert!(!serialized.contains("api_key"));
    assert!(!serialized.contains("authorization:"));
    assert!(!serialized.contains("http://"));
    assert!(!serialized.contains("https://"));
}
