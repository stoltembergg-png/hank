use provider_core::response::{
    FinishReason, NormalizedResponse, OutputPart, OutputPartKind, ProviderErrorCode,
    ProviderErrorInfo, ResponseStatus, Usage,
};
use provider_core::{ModelId, ProviderId};

fn complete_response() -> NormalizedResponse {
    NormalizedResponse {
        schema_version: 1,
        request_id: "req-1".into(),
        correlation_id: "corr-1".into(),
        provider_id: ProviderId::parse("mock-provider").unwrap(),
        model_id: ModelId::parse("mock-model").unwrap(),
        status: ResponseStatus::Complete,
        finish_reason: FinishReason::Stop,
        parts: vec![OutputPart {
            kind: OutputPartKind::Text,
            content: "hello result".into(),
        }],
        usage: Some(Usage {
            input_tokens: 4,
            output_tokens: 3,
        }),
        cost: None,
        error: None,
        provider_version: "0.1".into(),
        latency_ms: Some(12),
    }
}

#[test]
fn normalized_response_roundtrips_and_redacts_output_summary() {
    let response = complete_response();
    response.validate().unwrap();
    let encoded = serde_json::to_value(&response).unwrap();
    let decoded: NormalizedResponse = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
    let summary = response.redacted_summary();
    assert_eq!(summary.part_count, 1);
    assert_eq!(summary.output_bytes, 12);
    assert!(!serde_json::to_string(&summary)
        .unwrap()
        .contains("hello result"));
}

#[test]
fn response_distinguishes_terminal_statuses_and_unknown_finish_reason() {
    for status in [
        ResponseStatus::Complete,
        ResponseStatus::Error,
        ResponseStatus::Cancelled,
        ResponseStatus::Limit,
        ResponseStatus::Unknown,
    ] {
        let mut response = complete_response();
        response.status = status;
        if status == ResponseStatus::Error {
            response.error = Some(ProviderErrorInfo {
                code: ProviderErrorCode::ProviderRejected,
                message: "provider rejected request".into(),
                retryable: false,
            });
        }
        if status != ResponseStatus::Complete {
            response.finish_reason = FinishReason::Unknown;
        }
        if status == ResponseStatus::Error || status == ResponseStatus::Unknown {
            response.parts.clear();
        }
        response.validate().unwrap();
    }

    let mut value = serde_json::to_value(complete_response()).unwrap();
    value["status"] = serde_json::json!("future_status");
    value["finish_reason"] = serde_json::json!("future_finish");
    let decoded: NormalizedResponse = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.status, ResponseStatus::Unknown);
    assert_eq!(decoded.finish_reason, FinishReason::Unknown);
}

#[test]
fn usage_and_cost_are_optional_without_false_zero_values() {
    let mut response = complete_response();
    response.usage = None;
    response.cost = None;
    response.validate().unwrap();
    let encoded = serde_json::to_value(response).unwrap();
    assert!(encoded.get("usage").is_some());
    assert!(encoded.get("cost").is_some());
}

#[test]
fn malformed_response_parts_errors_and_secrets_fail_closed() {
    let mut response = complete_response();
    response.parts[0].content = "x".repeat(1_048_577);
    assert!(response.validate().is_err());

    let mut response = complete_response();
    response.status = ResponseStatus::Error;
    response.error = None;
    assert!(response.validate().is_err());

    let mut response = complete_response();
    response.error = Some(ProviderErrorInfo {
        code: ProviderErrorCode::Internal,
        message: "api_key=secret".into(),
        retryable: false,
    });
    assert!(response.validate().is_err());
}

#[test]
fn error_taxonomy_exposes_retryability_without_raw_provider_payload() {
    let info = ProviderErrorInfo {
        code: ProviderErrorCode::RateLimited,
        message: "provider rate limit".into(),
        retryable: true,
    };
    assert!(info.retryable);
    assert_eq!(info.code, ProviderErrorCode::RateLimited);
    assert!(!info.message.contains("token"));

    let mut response = complete_response();
    response.status = ResponseStatus::Error;
    response.finish_reason = FinishReason::Error;
    response.parts.clear();
    response.error = Some(info);
    response.validate().unwrap();
}
