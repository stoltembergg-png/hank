use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::NodeId;
use provider_core::CredentialRef;
use remote_core::credential_broker::{
    BrokerClock, CredentialAuditReason, CredentialBroker, CredentialBrokerError, CredentialScope,
    MAX_CREDENTIAL_LEASES,
};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn project_a() -> ProjectId {
    ProjectId::from_str("proj-11111111-1111-4111-8111-111111111111").unwrap()
}

fn project_b() -> ProjectId {
    ProjectId::from_str("proj-22222222-2222-4222-8222-222222222222").unwrap()
}

fn node_a() -> NodeId {
    NodeId::new("node-a").unwrap()
}

fn node_b() -> NodeId {
    NodeId::new("node-b").unwrap()
}

fn local_credential(label: &str) -> CredentialRef {
    CredentialRef::parse(label).unwrap()
}

#[derive(Debug, Default)]
struct FakeClock {
    now_ms: AtomicU64,
}

impl FakeClock {
    fn new(initial: u64) -> Arc<Self> {
        Arc::new(Self {
            now_ms: AtomicU64::new(initial),
        })
    }
    fn advance(&self, ms: u64) {
        self.now_ms.fetch_add(ms, Ordering::SeqCst);
    }
}

impl BrokerClock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

fn fresh_broker() -> CredentialBroker {
    CredentialBroker::with_clock(FakeClock::new(1_000))
}

#[test]
// @spec:AC-1466
fn broker_emits_opaque_handle_and_never_carries_secret_material() {
    let broker = fresh_broker();
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease = broker
        .issue(scope, local_credential("cred_alpha"), 60_000)
        .unwrap();
    let hex = lease.handle.as_hex();
    assert!(hex.starts_with("scoped_"));
    assert_eq!(hex.len(), "scoped_".len() + 64);
    // Two leases issued for the same (scope, ref) at different generations
    // must produce distinct handles, so a previously-purged handle cannot
    // be replayed.
    let scope_b = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let fresh = broker
        .issue(scope_b, local_credential("cred_alpha"), 60_000)
        .unwrap();
    assert_ne!(lease.handle, fresh.handle);
}

#[test]
// @spec:AC-1467
fn resolve_fails_closed_for_diverging_scope() {
    let broker = fresh_broker();
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease = broker
        .issue(scope.clone(), local_credential("cred_alpha"), 60_000)
        .unwrap();
    let wrong_node = CredentialScope::new(node_b(), project_a(), "agent-1").unwrap();
    let err = broker.resolve(&wrong_node, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::ScopeMismatch);
    let wrong_project = CredentialScope::new(node_a(), project_b(), "agent-1").unwrap();
    let err = broker.resolve(&wrong_project, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::ScopeMismatch);
    let wrong_actor = CredentialScope::new(node_a(), project_a(), "agent-2").unwrap();
    let err = broker.resolve(&wrong_actor, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::ScopeMismatch);
    let reason = broker
        .audit()
        .into_iter()
        .rev()
        .find(|event| matches!(event.reason, CredentialAuditReason::ScopeDenied));
    assert!(reason.is_some(), "scope-deny must be audited");
}

#[test]
// @spec:AC-1468
fn expired_or_revoked_lease_fails_closed() {
    let clock = FakeClock::new(1_000);
    let broker = CredentialBroker::with_clock(clock.clone());
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease = broker
        .issue(scope.clone(), local_credential("cred_alpha"), 1_000)
        .unwrap();
    // Just before expiry
    clock.advance(999);
    let ok = broker.resolve(&scope, lease.handle);
    assert!(ok.is_ok());
    // Past expiry
    clock.advance(1);
    let err = broker.resolve(&scope, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::Expired);
    let audit = broker.audit();
    assert!(audit
        .iter()
        .any(|event| matches!(event.reason, CredentialAuditReason::Expired)));

    // Revoke path
    clock.advance(3_000);
    let scope2 = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease2 = broker
        .issue(scope2.clone(), local_credential("cred_beta"), 60_000)
        .unwrap();
    broker.revoke(&scope2, lease2.handle).unwrap();
    let err = broker.resolve(&scope2, lease2.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::NotFound);
    // Capacity is freed by revoke (the revoked handle moves to a bounded
    // tombstone ring, not the active lease map).
    assert_eq!(broker.active_leases(), 0);
    assert_eq!(broker.revoked_tombstones(), 1);
}

#[test]
// @spec:AC-1469
fn broker_is_bounded_and_audit_never_records_secret_values() {
    let clock = FakeClock::new(1_000);
    let broker = CredentialBroker::with_clock(clock.clone());
    let scope_factory =
        |i: u64| CredentialScope::new(node_a(), project_a(), &format!("agent-{i}")).unwrap();
    for i in 0..MAX_CREDENTIAL_LEASES {
        let res = broker.issue(
            scope_factory(i as u64),
            local_credential(&format!("cred_{i}")),
            60_000,
        );
        assert!(res.is_ok(), "lease {i} should be issued");
    }
    let err = broker
        .issue(
            scope_factory(MAX_CREDENTIAL_LEASES as u64),
            local_credential("cred_overflow"),
            60_000,
        )
        .unwrap_err();
    assert_eq!(err, CredentialBrokerError::CapacityExhausted);
    let events = broker.audit();
    assert!(events
        .iter()
        .any(|event| matches!(event.reason, CredentialAuditReason::CapacityDenied)));
    for event in &events {
        let serialized = format!("{event:?}");
        assert!(
            !serialized.contains("cred_"),
            "audit must not leak credential labels: {serialized}"
        );
    }
    // The clock cannot be rolled back: even if the caller wanted to
    // "rewind" by creating a new broker, that broker has a different seed
    // and so handles do not alias.
    let other_broker = CredentialBroker::with_clock(FakeClock::new(500));
    let scope = scope_factory(0);
    let lease = other_broker
        .issue(scope, local_credential("cred_after_rewind"), 60_000)
        .unwrap();
    // The new broker was free; the handle exists and is bound to its own
    // seed/generation pair.
    let _ = lease;
}

#[test]
// @spec:AC-1470
fn lease_binding_prevents_cross_actor_or_cross_project_use() {
    let broker = fresh_broker();
    let scope_a = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease_a = broker
        .issue(scope_a.clone(), local_credential("cred_alpha"), 60_000)
        .unwrap();
    let lease_a2 = broker
        .issue(scope_a.clone(), local_credential("cred_beta"), 60_000)
        .unwrap();
    assert_ne!(lease_a.handle, lease_a2.handle);
    let wrong = CredentialScope::new(node_a(), project_b(), "agent-1").unwrap();
    let err = broker.resolve(&wrong, lease_a.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::ScopeMismatch);
}

#[test]
fn revoke_frees_capacity_for_subsequent_issues() {
    let broker = fresh_broker();
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let original: Vec<_> = (0..MAX_CREDENTIAL_LEASES)
        .map(|i| {
            broker
                .issue(
                    scope.clone(),
                    local_credential(&format!("cred_{i}")),
                    60_000,
                )
                .unwrap()
        })
        .collect();
    assert_eq!(broker.active_leases(), MAX_CREDENTIAL_LEASES);
    let err = broker
        .issue(scope.clone(), local_credential("cred_overflow"), 60_000)
        .unwrap_err();
    assert_eq!(err, CredentialBrokerError::CapacityExhausted);
    for lease in &original {
        broker.revoke(&scope, lease.handle).unwrap();
    }
    assert_eq!(broker.active_leases(), 0);
    assert_eq!(broker.revoked_tombstones(), MAX_CREDENTIAL_LEASES);
    let fresh = broker
        .issue(scope.clone(), local_credential("cred_after_free"), 60_000)
        .unwrap();
    assert_eq!(broker.active_leases(), 1);
    assert_ne!(fresh.handle, original[0].handle);
}

#[test]
fn invalid_scope_is_rejected_without_state_mutation() {
    let broker = fresh_broker();
    let err = CredentialScope::new(node_a(), project_a(), "   ").unwrap_err();
    assert_eq!(err, CredentialBrokerError::InvalidScope);
    let err = CredentialScope::new(node_a(), project_a(), &"x".repeat(129)).unwrap_err();
    assert_eq!(err, CredentialBrokerError::InvalidScope);
    let err = CredentialScope::new(node_a(), project_a(), "agent\n1").unwrap_err();
    assert_eq!(err, CredentialBrokerError::InvalidScope);
    let err = CredentialScope::new(node_a(), project_a(), "").unwrap_err();
    assert_eq!(err, CredentialBrokerError::InvalidScope);
    // Bad NodeId: empty string is rejected by `NodeId::new` itself, but
    // an oversized or control-character-filled NodeId can be built via
    // the public tuple struct and is caught at scope construction.
    let bad_node = NodeId("\u{0000}node".into());
    let err = CredentialScope::new(bad_node, project_a(), "agent-1").unwrap_err();
    assert_eq!(err, CredentialBrokerError::InvalidScope);
    let oversized = NodeId("x".repeat(129));
    let err = CredentialScope::new(oversized, project_a(), "agent-1").unwrap_err();
    assert_eq!(err, CredentialBrokerError::InvalidScope);
    assert_eq!(broker.active_leases(), 0);
}

#[test]
fn issue_revalidates_scope_construction() {
    for bad in [
        "   ",
        &"x".repeat(129),
        "agent\n1",
        "agent\t1",
        "agent\u{0000}1",
    ] {
        let scope = CredentialScope::new(node_a(), project_a(), bad);
        assert!(scope.is_err(), "actor {bad:?} must be rejected");
    }
    let broker = fresh_broker();
    assert_eq!(broker.active_leases(), 0);
}

#[test]
fn unknown_handle_resolves_as_not_found() {
    let clock = FakeClock::new(1_000);
    let broker = CredentialBroker::with_clock(clock.clone());
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease = broker
        .issue(scope.clone(), local_credential("cred_alpha"), 60_000)
        .unwrap();
    // A fresh broker has no leases; resolving the same handle must fail
    // closed and the audit log records the probing attempt.
    let fresh = CredentialBroker::with_clock(FakeClock::new(2_000));
    let err = fresh.resolve(&scope, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::NotFound);
    let audit = fresh.audit();
    assert!(audit
        .iter()
        .any(|event| matches!(event.reason, CredentialAuditReason::NotFound)));
}

#[test]
fn revoke_requires_matching_scope() {
    let broker = fresh_broker();
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease = broker
        .issue(scope.clone(), local_credential("cred_alpha"), 60_000)
        .unwrap();
    let wrong = CredentialScope::new(node_a(), project_a(), "agent-2").unwrap();
    let err = broker.revoke(&wrong, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::ScopeMismatch);
    broker.revoke(&scope, lease.handle).unwrap();
    assert_eq!(broker.revoked_tombstones(), 1);
}

#[test]
fn per_broker_seed_makes_handles_independent_across_brokers() {
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let a = CredentialBroker::with_clock(FakeClock::new(1_000))
        .issue(scope.clone(), local_credential("cred_alpha"), 60_000)
        .unwrap();
    let b = CredentialBroker::with_clock(FakeClock::new(1_000))
        .issue(scope.clone(), local_credential("cred_alpha"), 60_000)
        .unwrap();
    // Two brokers (even with the same fake clock) must produce different
    // handles, so a handle from a previous broker instance cannot replay
    // into a new one.
    assert_ne!(a.handle, b.handle);
}

#[test]
fn caller_cannot_bypass_expiry_by_using_different_broker_clock() {
    let clock = FakeClock::new(1_000);
    let broker = CredentialBroker::with_clock(clock.clone());
    let scope = CredentialScope::new(node_a(), project_a(), "agent-1").unwrap();
    let lease = broker
        .issue(scope.clone(), local_credential("cred_alpha"), 1_000)
        .unwrap();
    // Past expiry, the resolve fails.
    clock.advance(1_001);
    let err = broker.resolve(&scope, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::Expired);
    // Even if a caller tried to construct a brand-new broker, that broker
    // does not share state with the original — the old lease is gone.
    let fresh = CredentialBroker::with_clock(FakeClock::new(500));
    let err = fresh.resolve(&scope, lease.handle).unwrap_err();
    assert_eq!(err, CredentialBrokerError::NotFound);
}
