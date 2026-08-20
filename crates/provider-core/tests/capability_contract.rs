use std::collections::{BTreeMap, BTreeSet};

use provider_core::capabilities::{
    CapabilityError, CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilityRequirement,
    CapabilitySource, CapabilityState, ModelModality,
};
use provider_core::{ModelId, ProviderId};

fn report() -> CapabilityReport {
    CapabilityReport {
        schema_version: 1,
        provider_id: ProviderId::parse("mock-provider").unwrap(),
        model_id: ModelId::parse("mock-model").unwrap(),
        version: "cap-1".into(),
        source: CapabilitySource::Provider,
        modalities: BTreeMap::from([
            (ModelModality::Text, CapabilityState::Supported),
            (ModelModality::Image, CapabilityState::Unknown),
            (ModelModality::Audio, CapabilityState::Unsupported),
        ]),
        features: BTreeMap::from([
            (CapabilityFeature::Streaming, CapabilityState::Supported),
            (CapabilityFeature::ToolUse, CapabilityState::Unknown),
        ]),
        limits: CapabilityLimits {
            max_context_tokens: Some(32_768),
            max_output_tokens: Some(8_192),
        },
    }
}

#[test]
fn capability_report_roundtrips_and_validates_deterministically() {
    let report = report();
    report.validate().unwrap();
    let encoded = serde_json::to_value(&report).unwrap();
    let decoded: CapabilityReport = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
    assert_eq!(
        report
            .modalities
            .keys()
            .map(|m| format!("{m:?}"))
            .collect::<Vec<_>>(),
        vec!["Text", "Image", "Audio"]
    );
}

#[test]
fn unknown_capability_is_not_treated_as_supported() {
    let report = report();
    assert!(report.supports_modality(ModelModality::Text));
    assert!(!report.supports_modality(ModelModality::Image));
    assert!(!report.supports_feature(CapabilityFeature::ToolUse));
}

#[test]
fn compatibility_rejects_unsupported_and_unknown_before_adapter() {
    let report = report();
    let mut required = BTreeSet::from([ModelModality::Audio]);
    let error = report
        .check_compatibility(&CapabilityRequirement {
            modalities: required.clone(),
            features: BTreeSet::new(),
            min_context_tokens: None,
            min_output_tokens: None,
        })
        .unwrap_err();
    assert_eq!(
        error,
        CapabilityError::UnsupportedModality(ModelModality::Audio)
    );

    required = BTreeSet::from([ModelModality::Image]);
    let error = report
        .check_compatibility(&CapabilityRequirement {
            modalities: required,
            features: BTreeSet::new(),
            min_context_tokens: None,
            min_output_tokens: None,
        })
        .unwrap_err();
    assert_eq!(
        error,
        CapabilityError::UnknownModality(ModelModality::Image)
    );
}

#[test]
fn incompatible_limits_and_features_are_typed() {
    let report = report();
    let error = report
        .check_compatibility(&CapabilityRequirement {
            modalities: BTreeSet::from([ModelModality::Text]),
            features: BTreeSet::from([CapabilityFeature::ToolUse]),
            min_context_tokens: Some(64_000),
            min_output_tokens: Some(16_000),
        })
        .unwrap_err();
    assert!(matches!(
        error,
        CapabilityError::UnknownFeature(CapabilityFeature::ToolUse)
    ));

    let error = report
        .check_compatibility(&CapabilityRequirement {
            modalities: BTreeSet::from([ModelModality::Text]),
            features: BTreeSet::new(),
            min_context_tokens: Some(64_000),
            min_output_tokens: None,
        })
        .unwrap_err();
    assert!(matches!(error, CapabilityError::InsufficientContext { .. }));
}

#[test]
fn malformed_or_oversized_capability_reports_fail_closed() {
    let mut invalid = report();
    invalid.schema_version = 2;
    assert!(invalid.validate().is_err());

    let mut invalid = report();
    invalid.version = "x".repeat(65);
    assert!(invalid.validate().is_err());

    let mut invalid = report();
    invalid.limits.max_context_tokens = Some(2_000_001);
    assert!(invalid.validate().is_err());
}
