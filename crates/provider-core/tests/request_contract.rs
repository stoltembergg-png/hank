use std::collections::BTreeSet;

use provider_core::capabilities::{CapabilityFeature, CapabilityRequirement, ModelModality};
use provider_core::request::{
    CancellationMetadata, MessageRole, NormalizedMessage, NormalizedRequest, RequestBudget,
    ToolContext,
};
use provider_core::{ModelId, ProviderId};

fn valid_request() -> NormalizedRequest {
    NormalizedRequest {
        schema_version: 1,
        request_id: "req-1".into(),
        correlation_id: "corr-1".into(),
        project_id: "prj_1".into(),
        agent_id: "agt_1".into(),
        session_id: Some("ses_1".into()),
        provider_id: ProviderId::parse("mock-provider").unwrap(),
        model_id: ModelId::parse("mock-model").unwrap(),
        messages: vec![NormalizedMessage {
            role: MessageRole::User,
            content: "hello".into(),
        }],
        modalities: BTreeSet::from([ModelModality::Text]),
        capabilities: CapabilityRequirement {
            modalities: BTreeSet::from([ModelModality::Text]),
            features: BTreeSet::from([CapabilityFeature::Streaming]),
            min_context_tokens: Some(4_096),
            min_output_tokens: Some(512),
        },
        tools: vec![ToolContext {
            tool_id: "tool-calendar".into(),
            capability_fingerprint: "cap_123".into(),
        }],
        budget: RequestBudget {
            max_tokens: Some(512),
            max_cost_micros: Some(100_000),
        },
        cancellation: CancellationMetadata {
            cancellation_id: "cancel-1".into(),
            deadline_unix_ms: Some(1_900_000_000_000),
        },
        temperature: Some(0.2),
    }
}

#[test]
fn normalized_request_roundtrips_and_redacts_payload() {
    let request = valid_request();
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    let decoded: NormalizedRequest = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.redacted_summary().message_count, 1);
    assert_eq!(decoded.redacted_summary().tool_count, 1);
    assert!(!serde_json::to_string(&decoded.redacted_summary())
        .unwrap()
        .contains("hello"));
}

#[test]
fn identity_scope_and_cancellation_are_mandatory_and_bounded() {
    let mut request = valid_request();
    request.project_id.clear();
    assert!(request.validate().is_err());

    let mut request = valid_request();
    request.agent_id = "a".repeat(129);
    assert!(request.validate().is_err());

    let mut request = valid_request();
    request.cancellation.cancellation_id.clear();
    assert!(request.validate().is_err());
}

#[test]
fn messages_tools_budget_and_numeric_limits_fail_closed() {
    let mut request = valid_request();
    request.messages = vec![NormalizedMessage {
        role: MessageRole::User,
        content: "x".repeat(1_048_577),
    }];
    assert!(request.validate().is_err());

    let mut request = valid_request();
    request.tools[0].capability_fingerprint = "api_key=secret".into();
    assert!(request.validate().is_err());

    let mut request = valid_request();
    request.budget.max_tokens = Some(0);
    assert!(request.validate().is_err());

    let mut request = valid_request();
    request.temperature = Some(2.1);
    assert!(request.validate().is_err());
}

#[test]
fn capabilities_are_requirements_and_are_checked_before_adapter() {
    let request = valid_request();
    let mut report = provider_core::capabilities::CapabilityReport {
        schema_version: 1,
        provider_id: ProviderId::parse("mock-provider").unwrap(),
        model_id: ModelId::parse("mock-model").unwrap(),
        version: "cap-1".into(),
        source: provider_core::capabilities::CapabilitySource::Provider,
        modalities: std::collections::BTreeMap::from([(
            ModelModality::Text,
            provider_core::capabilities::CapabilityState::Supported,
        )]),
        features: std::collections::BTreeMap::new(),
        limits: provider_core::capabilities::CapabilityLimits {
            max_context_tokens: Some(8_192),
            max_output_tokens: Some(1_024),
        },
    };
    report.validate().unwrap();
    assert!(request.validate_against_capabilities(&report).is_err());

    report.features.insert(
        CapabilityFeature::Streaming,
        provider_core::capabilities::CapabilityState::Supported,
    );
    assert!(request.validate_against_capabilities(&report).is_ok());
}

#[test]
fn empty_modalities_and_messages_are_rejected() {
    let mut request = valid_request();
    request.messages.clear();
    assert!(request.validate().is_err());

    let mut request = valid_request();
    request.modalities.clear();
    assert!(request.validate().is_err());
}
