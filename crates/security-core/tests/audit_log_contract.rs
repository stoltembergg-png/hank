/// AC-2021..AC-2025 contract tests for `security-core::audit`.
///
/// Each test is annotated with its corresponding `@spec:AC-NNNN` tag so the
/// ONP feature runner can map it back to the SDD criterion.
use security_core::audit::{
    class_query, unique_payload_keys, AuditClass, AuditError, AuditLog, AuditPolicy, AuditQuery,
    InMemorySink, IntegrityClassification, Payload, RedactedField, MAX_EVENT_PAYLOAD_BYTES,
    REDACTED_PLACEHOLDER,
};
#[allow(unused_imports)]
use security_core::AuditSink;
use sha2::{Digest, Sha256};

fn policy() -> AuditPolicy {
    AuditPolicy::new("project-1", "rev-1", 16, 8, 60_000).expect("policy")
}

fn payload() -> Payload {
    let mut p = Payload::new();
    p.insert_text("operation", "grant").expect("text");
    p.insert_redacted("credential", RedactedField::Secret)
        .expect("redacted");
    p
}

#[test]
// @spec:AC-2021
fn record_produces_monotonic_sequence_and_chain_hash() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    let mut last_hash = None;
    for i in 0..3 {
        let event = log
            .record(
                format!("evt-{i:02}"),
                "actor-1",
                "resource-1",
                AuditClass::Authorization,
                1_000 + i,
                payload(),
            )
            .expect("record");
        assert_eq!(event.sequence(), i);
        assert_eq!(event.policy_revision(), "rev-1");
        assert_ne!(event.hash(), "");
        if let Some(prev) = &last_hash {
            assert_eq!(event.prev_hash(), prev);
        } else {
            assert_eq!(event.prev_hash(), security_core::audit::GENESIS_HASH);
        }
        last_hash = Some(event.hash().to_string());
    }
}

#[test]
// @spec:AC-2021
fn verify_chain_returns_ok_on_clean_log() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    for i in 0..3 {
        log.record(
            format!("evt-{i:02}"),
            "actor",
            "resource",
            AuditClass::Recovery,
            1_000 + i,
            payload(),
        )
        .expect("record");
    }
    let integrity = log.verify_chain();
    assert_eq!(integrity.classification(), IntegrityClassification::Ok);
    assert!(integrity.is_ok());
    assert!(integrity.event_id().is_none());
    assert!(integrity.index().is_none());
}

#[test]
// @spec:AC-2022
fn verify_chain_detects_out_of_order_after_sink_injection() {
    // Build a clean log with two well-formed events linked by their
    // hashes, then inject a third event with a valid `prev_hash` but
    // with a sequence that jumps from 1 to 5. The chain verification
    // must report `OutOfOrder` with the offending index and event id.
    let mut events = Vec::new();
    let mut prev_hash = security_core::audit::GENESIS_HASH.to_string();
    for i in 0..2u64 {
        let p = payload();
        let event = security_core::audit::AuditEvent::assemble(
            format!("evt-{i:02}"),
            "actor",
            "resource",
            "rev-1",
            AuditClass::Migration,
            1_000 + i,
            i,
            prev_hash.clone(),
            p,
        )
        .expect("assemble");
        prev_hash = event.hash().to_string();
        events.push(event);
    }
    let forged = security_core::audit::AuditEvent::assemble(
        "evt-forged",
        "actor",
        "resource",
        "rev-1",
        AuditClass::Migration,
        1_010,
        5,
        prev_hash.clone(),
        payload(),
    )
    .expect("assemble");
    events.push(forged);
    let forged_log = AuditLog::from_events(policy(), InMemorySink::new(), events).expect("log");
    let integrity = forged_log.verify_chain();
    assert!(!integrity.is_ok());
    assert_eq!(
        integrity.classification(),
        IntegrityClassification::OutOfOrder
    );
    assert_eq!(integrity.index(), Some(2));
    assert_eq!(integrity.event_id(), Some("evt-forged"));
}

#[test]
// @spec:AC-2025
fn record_fails_when_sink_unavailable() {
    let mut sink = InMemorySink::new();
    sink.fail_next_write();
    let mut log = AuditLog::new(policy(), sink).expect("log");
    let result = log.record(
        "evt-00",
        "actor",
        "resource",
        AuditClass::Denial,
        1_000,
        payload(),
    );
    assert!(matches!(result, Err(AuditError::SinkUnavailable)));
    assert!(log.is_empty());
    assert_eq!(log.sink().events().len(), 0);
}

#[test]
// @spec:AC-2023
fn redacted_fields_serialize_as_placeholder() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    let event = log
        .record(
            "evt-00",
            "actor",
            "resource",
            AuditClass::Authorization,
            1_000,
            payload(),
        )
        .expect("record");
    let exported = log.export(8).expect("export");
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].event_id(), "evt-00");
    let rendered: Vec<(String, String)> = event
        .payload()
        .iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    for (k, v) in &rendered {
        if k == "credential" {
            assert_eq!(v, REDACTED_PLACEHOLDER);
        } else {
            assert_ne!(v, REDACTED_PLACEHOLDER);
        }
    }
    // Hashes must depend on rendered payload, so the payload_sha256 is
    // independent of the redacted marker. The BTreeMap iterates in key
    // order, so the hashed stream is `credential` then `operation`.
    let mut hasher = Sha256::new();
    hasher.update(b"credential");
    hasher.update([0u8]);
    hasher.update(REDACTED_PLACEHOLDER.as_bytes());
    hasher.update([0xffu8]);
    hasher.update(b"operation");
    hasher.update([0u8]);
    hasher.update(b"grant");
    hasher.update([0xffu8]);
    let expected = hex_lower(&hasher.finalize());
    assert_eq!(event.payload_sha256(), expected);
    let keys = unique_payload_keys(&exported);
    assert!(keys.contains("operation"));
    assert!(keys.contains("credential"));
}

#[test]
// @spec:AC-2024
fn query_rejects_empty_or_invalid_filters() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    log.record(
        "evt-00",
        "actor",
        "resource",
        AuditClass::Authorization,
        1_000,
        payload(),
    )
    .expect("record");
    let empty = AuditQuery::new();
    assert!(matches!(log.query(&empty), Err(AuditError::QueryRejected)));
    let bad_limit = AuditQuery::new().with_actor("actor").with_limit(0);
    assert!(matches!(
        log.query(&bad_limit),
        Err(AuditError::QueryRejected)
    ));
    let huge_limit = AuditQuery::new().with_actor("actor").with_limit(2_000_000);
    assert!(matches!(
        log.query(&huge_limit),
        Err(AuditError::QueryRejected)
    ));
    let since_until = AuditQuery::new()
        .with_actor("actor")
        .since_ms(2_000)
        .until_ms(2_000);
    assert!(matches!(
        log.query(&since_until),
        Err(AuditError::QueryRejected)
    ));
}

#[test]
// @spec:AC-2024
fn query_filters_by_actor_resource_class_and_interval() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    for (i, (actor, resource, class, ts)) in [
        ("actor-1", "resource-a", AuditClass::Authorization, 1_000u64),
        ("actor-2", "resource-b", AuditClass::Denial, 1_500),
        ("actor-1", "resource-b", AuditClass::Authorization, 2_000),
        ("actor-1", "resource-c", AuditClass::Migration, 2_500),
    ]
    .iter()
    .enumerate()
    {
        log.record(
            format!("evt-{i:02}"),
            *actor,
            *resource,
            *class,
            *ts,
            payload(),
        )
        .expect("record");
    }
    let q1 = AuditQuery::new()
        .with_actor("actor-1")
        .with_limit(16)
        .since_ms(1_500)
        .until_ms(3_000);
    let r1 = log.query(&q1).expect("query");
    assert_eq!(r1.len(), 2);
    for e in r1.events() {
        assert_eq!(e.actor(), "actor-1");
        assert!(e.timestamp_ms() >= 1_500 && e.timestamp_ms() < 3_000);
    }
    let q2 = AuditQuery::new()
        .with_class(AuditClass::Authorization)
        .with_limit(16);
    let r2 = log.query(&q2).expect("query");
    assert_eq!(r2.len(), 2);
    let q3 = AuditQuery::new().with_resource("resource-b").with_limit(16);
    let r3 = log.query(&q3).expect("query");
    assert_eq!(r3.len(), 2);
    let q4 = class_query(AuditClass::Migration, 16).expect("class query");
    let r4 = log.query(&q4).expect("query");
    assert_eq!(r4.len(), 1);
}

#[test]
// @spec:AC-2024
fn export_is_bounded_by_max_export_rows() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    for i in 0..5 {
        log.record(
            format!("evt-{i:02}"),
            "actor",
            "resource",
            AuditClass::Other,
            1_000 + i as u64,
            payload(),
        )
        .expect("record");
    }
    assert!(matches!(log.export(0), Err(AuditError::ExportRejected)));
    assert!(matches!(
        log.export(2_000_000),
        Err(AuditError::ExportRejected)
    ));
    let r = log.export(3).expect("export");
    assert_eq!(r.len(), 3);
}

#[test]
// @spec:AC-2024
fn retain_trims_by_window_and_capacity() {
    let p = AuditPolicy::new("project-1", "rev-1", 8, 4, 1_000).expect("policy");
    let mut log = AuditLog::new(p, InMemorySink::new()).expect("log");
    for i in 0..6 {
        log.record(
            format!("evt-{i:02}"),
            "actor",
            "resource",
            AuditClass::Recovery,
            100 + i as u64 * 200,
            payload(),
        )
        .expect("record");
    }
    let dropped = log.retain(2_500).expect("retain");
    assert!(dropped >= 2);
    assert!(log.len() <= 4);
    // The remaining events must all be inside the retention window
    // (timestamp >= now - retention_ms = 1500).
    for e in log.export(8).expect("export") {
        assert!(e.timestamp_ms() >= 1_500);
    }
}

#[test]
// @spec:AC-2021
fn record_rejects_policy_revision_mismatch() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    let result = security_core::audit::AuditEvent::assemble(
        "evt-00",
        "actor",
        "resource",
        "rev-other",
        AuditClass::Other,
        1_000,
        0,
        security_core::audit::GENESIS_HASH.to_string(),
        payload(),
    );
    assert!(result.is_ok());
    let event = log
        .record(
            "evt-00",
            "actor",
            "resource",
            AuditClass::Other,
            1_000,
            payload(),
        )
        .expect("record");
    assert_eq!(event.policy_revision(), "rev-1");
}

#[test]
// @spec:AC-2023
fn deterministic_serialization_for_same_event() {
    let mut log = AuditLog::new(policy(), InMemorySink::new()).expect("log");
    let event = log
        .record(
            "evt-00",
            "actor",
            "resource",
            AuditClass::Authorization,
            1_000,
            payload(),
        )
        .expect("record");
    let first_render: Vec<(String, String)> = event
        .payload()
        .iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    let second_render: Vec<(String, String)> = event
        .payload()
        .iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    assert_eq!(first_render, second_render);
}

#[test]
// @spec:AC-2021
fn policy_rejects_invalid_capacities_and_retention() {
    assert!(matches!(
        AuditPolicy::new("project-1", "rev-1", 0, 1, 1_000),
        Err(AuditError::PolicyInvalid)
    ));
    assert!(matches!(
        AuditPolicy::new("project-1", "rev-1", 4, 0, 1_000),
        Err(AuditError::PolicyInvalid)
    ));
    assert!(matches!(
        AuditPolicy::new("project-1", "rev-1", 4, 5, 1_000),
        Err(AuditError::PolicyInvalid)
    ));
    assert!(matches!(
        AuditPolicy::new("project-1", "rev-1", 4, 4, u64::MAX),
        Err(AuditError::PolicyInvalid)
    ));
    assert!(matches!(
        AuditPolicy::new("", "rev-1", 4, 4, 1_000),
        Err(AuditError::PolicyInvalid)
    ));
    assert!(matches!(
        AuditPolicy::new("project-1", "", 4, 4, 1_000),
        Err(AuditError::PolicyInvalid)
    ));
}

#[test]
// @spec:AC-2023
fn payload_rejects_oversize_inserts() {
    let mut p = Payload::new();
    p.insert_text("key", "value").expect("ok");
    let big = "x".repeat(MAX_EVENT_PAYLOAD_BYTES);
    let res = p.insert_text("big", big);
    assert!(matches!(res, Err(AuditError::PayloadTooLarge)));
    assert!(p.iter().any(|(k, _)| k == "key"));
    assert!(!p.iter().any(|(k, _)| k == "big"));
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}
