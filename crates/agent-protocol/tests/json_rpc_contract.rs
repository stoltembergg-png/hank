use agent_protocol::json_rpc::{
    encode_frame, error_code, is_known_method, CompletionState, FrameDecoder, FrameDecoderState,
    JsonRpcCorrelation, JsonRpcMessage, JsonRpcParseError, MAX_FRAME_BYTES, MAX_PENDING_IDS,
};
use serde_json::json;

fn health_request(id: u64) -> JsonRpcMessage {
    JsonRpcMessage::request(id, "health", json!({"schema_version": 1}))
}

fn cancel_notification() -> JsonRpcMessage {
    JsonRpcMessage::notification(
        "cancel",
        json!({"schema_version": 1, "request_id": "req-00000000-0000-4000-8000-000000000404"}),
    )
}

#[test]
// @spec:AC-688
fn golden_frames_are_exact_and_deterministic() {
    let message = health_request(1);
    let payload = serde_json::to_string(&message).expect("serializes");
    let expected = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
    assert_eq!(String::from_utf8_lossy(&encode_frame(&payload)), expected);
    assert_eq!(
        String::from_utf8_lossy(&encode_frame(&payload)),
        expected,
        "encoding must be byte-stable"
    );
    assert_eq!(
        payload,
        r#"{"jsonrpc":"2.0","id":1,"method":"health","params":{"schema_version":1}}"#
    );

    let notification = cancel_notification();
    let notification_payload = serde_json::to_string(&notification).expect("serializes");
    assert!(notification_payload.contains(r#""method":"cancel""#));
    assert!(
        !notification_payload.contains("\"id\""),
        "notifications carry no id"
    );
}

#[test]
// @spec:AC-688
fn fragmented_and_coalesced_frames_decode_identically() {
    let first = encode_frame(&serde_json::to_string(&health_request(1)).expect("serializes"));
    let second = encode_frame(&serde_json::to_string(&cancel_notification()).expect("serializes"));

    let mut fragmented = FrameDecoder::new();
    for byte in first.iter() {
        fragmented.push(&[*byte]).expect("single byte accepted");
    }
    match fragmented.pop_message().expect("frame complete") {
        Ok(message) => assert_eq!(message, health_request(1)),
        Err(error) => panic!("fragmented frame must decode: {error}"),
    }

    let mut coalesced = FrameDecoder::new();
    coalesced
        .push(&[first, second].concat())
        .expect("two frames within bounds");
    match coalesced.pop_message().expect("first frame") {
        Ok(message) => assert_eq!(message, health_request(1)),
        Err(error) => panic!("coalesced frame must decode: {error}"),
    }
    match coalesced.pop_message().expect("second frame") {
        Ok(message) => assert_eq!(message, cancel_notification()),
        Err(error) => panic!("coalesced frame must decode: {error}"),
    }
    assert!(!coalesced.has_buffered_bytes());
}

#[test]
// @spec:AC-689
fn malformed_and_oversize_inputs_fail_closed_without_panicking() {
    let mut oversize = FrameDecoder::new();
    assert_eq!(
        oversize.push(&[b'x'; MAX_FRAME_BYTES + 1]),
        Err(JsonRpcParseError::OversizeFrame)
    );

    let mut bad_json = FrameDecoder::new();
    bad_json
        .push(&encode_frame("not json"))
        .expect("bytes accepted");
    assert_eq!(
        bad_json.pop_frame().expect("frame present"),
        Err(JsonRpcParseError::InvalidJson)
    );

    let mut wrong_shape = FrameDecoder::new();
    wrong_shape
        .push(&encode_frame(r#"[1,2,3]"#))
        .expect("bytes accepted");
    assert_eq!(
        wrong_shape.pop_message().expect("frame present"),
        Err(JsonRpcParseError::InvalidMessage),
        "payload that is not a JSON-RPC message must be rejected"
    );

    // Fuzz-style: hostile bytes never panic and stay bounded.
    let mut fuzzer = FrameDecoder::new();
    let mut seed = 0x2A_u32;
    for _ in 0..2_000 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let byte = (seed >> 16) as u8;
        let result = fuzzer.push(&[byte]);
        assert!(
            result.is_ok() || result == Err(JsonRpcParseError::OversizeFrame),
            "decoder must stay bounded"
        );
        if let Some(decoded) = fuzzer.pop_message() {
            assert!(
                decoded.is_err(),
                "random bytes must never decode to a valid message"
            );
        }
    }

    // Disconnect with a partial frame leaves a defined state.
    let mut partial = FrameDecoder::new();
    partial
        .push(b"Content-Length: 100\r\n\r\n{\"jsonrpc\"")
        .expect("partial bytes accepted");
    assert_eq!(partial.disconnect(), FrameDecoderState::Disconnected);
    assert!(partial.is_disconnected());
    assert!(
        !partial.has_buffered_bytes(),
        "partial frame discarded on disconnect"
    );
}

#[test]
// @spec:AC-690
fn correlation_states_are_defined_for_duplicate_capacity_and_expiry() {
    let mut correlation = JsonRpcCorrelation::new();

    correlation
        .register(1, 0, 1_000)
        .expect("first id registers");
    assert_eq!(
        correlation.register(1, 10, 1_000),
        Err(JsonRpcParseError::InvalidMessage),
        "duplicate in-flight id must fail closed"
    );
    assert_eq!(correlation.complete(1, 900), CompletionState::Completed);
    assert_eq!(correlation.complete(1, 900), CompletionState::UnknownId);
    assert_eq!(correlation.cancel(42), CompletionState::UnknownId);

    correlation.register(2, 0, 500).expect("registers");
    assert_eq!(
        correlation.complete(2, 500),
        CompletionState::Expired,
        "expired deadline has a defined completion state"
    );

    for id in 3..=(MAX_PENDING_IDS as u64 + 2) {
        correlation
            .register(id, 0, 10_000)
            .expect("bounded ids register");
    }
    assert_eq!(correlation.pending_len(), MAX_PENDING_IDS);
    assert_eq!(
        correlation.register(9_999, 0, 10_000),
        Err(JsonRpcParseError::InvalidMessage),
        "pending capacity must fail closed"
    );
}

#[test]
// @spec:AC-693
fn method_allowlist_and_error_codes_validate_without_python() {
    for method in [
        "handshake",
        "request",
        "cancel",
        "health",
        "error",
        "shutdown",
    ] {
        assert!(is_known_method(method), "{method} must be allowed");
    }
    for method in ["teleport", "exec", "system", "", "HEALTH"] {
        assert!(!is_known_method(method), "{method} must be rejected");
    }

    assert_eq!(error_code::PARSE_ERROR, -32_700);
    assert_eq!(error_code::INVALID_REQUEST, -32_600);
    assert_eq!(error_code::METHOD_NOT_FOUND, -32_601);
    assert_eq!(error_code::INVALID_PARAMS, -32_602);
    assert_eq!(error_code::INTERNAL_ERROR, -32_603);
    assert_eq!(error_code::OVERSIZE_FRAME, -32_010);
    assert_eq!(error_code::DUPLICATE_ID, -32_011);
    assert_eq!(error_code::BACKPRESSURE, -32_012);
    assert_eq!(error_code::REQUEST_EXPIRED, -32_013);

    let message = JsonRpcMessage::request(1, "shutdown", json!({"reason": "user"}));
    message.validate().expect("valid message validates");
    let rendered = format!("{:?}", JsonRpcParseError::InvalidJson);
    assert!(
        !rendered.to_lowercase().contains("secret"),
        "errors must be redacted"
    );

    let mut untagged = json!({"id": 1, "method": "health", "params": {}});
    untagged.as_object_mut().expect("object").remove("jsonrpc");
    let decoded: Result<JsonRpcMessage, _> =
        serde_json::from_value(untagged).map_err(|_| JsonRpcParseError::InvalidMessage);
    // Untagged variants require the jsonrpc discriminator to resolve.
    assert!(
        decoded.is_err() || decoded.expect("resolved").validate().is_err(),
        "messages without the jsonrpc discriminator must not pass"
    );
}
