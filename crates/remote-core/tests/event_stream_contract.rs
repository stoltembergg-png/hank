//! Contract tests for the bounded authenticated event stream (PR-247).

use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::{Handshake, NodeId, PeerId, ProtocolRevision};
use provider_core::CredentialRef;
use remote_core::event_stream::{EventStream, EventStreamError, EventStreamPolicy};
use remote_core::{
    AuthenticatedDaemon, DaemonError, DaemonLease, DaemonPolicy, DaemonSessionState,
    PeerAuthenticator,
};
use std::str::FromStr;

struct AcceptedAuthenticator;

impl PeerAuthenticator for AcceptedAuthenticator {
    fn authenticate(
        &self,
        _credential: &CredentialRef,
    ) -> Result<remote_core::AuthenticatedPeer, DaemonError> {
        Ok(remote_core::AuthenticatedPeer::new("peer-a", "node-1").unwrap())
    }
}

fn project() -> ProjectId {
    ProjectId::from_str("proj-11111111-1111-4111-8111-111111111111").unwrap()
}

fn policy(project: ProjectId) -> DaemonPolicy {
    DaemonPolicy::exact("peer-a", "node-1", project, 60_000).unwrap()
}

fn credential() -> CredentialRef {
    CredentialRef::parse("cred_remote_1").unwrap()
}

fn handshake(project: ProjectId) -> Handshake {
    Handshake {
        protocol: ProtocolRevision::V1_0,
        api: ProtocolRevision::V1_0,
        peer: PeerId::new("peer-a").unwrap(),
        node: NodeId::new("node-1").unwrap(),
        project,
        capabilities: [String::from("observe")].into_iter().collect(),
    }
}

fn bounded_daemon() -> AuthenticatedDaemon<AcceptedAuthenticator> {
    AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project()))
}

fn active_lease(daemon: &AuthenticatedDaemon<AcceptedAuthenticator>) -> DaemonLease {
    daemon
        .bootstrap(Some(credential()), handshake(project()), 1_000)
        .unwrap()
}

fn small_policy() -> EventStreamPolicy {
    EventStreamPolicy::bounded(4, 64, 2).unwrap()
}

#[test]
// @spec:AC-1461
fn stream_rejects_without_valid_lease() {
    let daemon = bounded_daemon();
    let stream = EventStream::new(&daemon, small_policy());
    let lease = active_lease(&daemon);

    // Unbound stream refuses to push.
    assert_eq!(
        stream.push(1_000, b"event".to_vec()),
        Err(EventStreamError::NoActiveLease)
    );

    // Binding to an active lease succeeds.
    assert_eq!(stream.bind(&lease, 1_000), Ok(()));
    assert_eq!(
        stream.bind(&lease, 1_000),
        Err(EventStreamError::StaleLease)
    );

    // Push succeeds while lease is active.
    assert_eq!(stream.push(1_000, b"ok".to_vec()), Ok(1));

    // After the lease is revoked the stream refuses new events.
    assert_eq!(daemon.revoke(lease.id), Ok(DaemonSessionState::Closed));
    assert_eq!(
        stream.push(1_000, b"x".to_vec()),
        Err(EventStreamError::NoActiveLease)
    );
    // Resume also fails closed on a revoked lease.
    assert_eq!(
        stream.resume(1_000, 0),
        Err(EventStreamError::NoActiveLease)
    );
}

#[test]
// @spec:AC-1462
fn sequence_rejects_duplicates_and_out_of_window_replays() {
    let daemon = bounded_daemon();
    let stream = EventStream::new(&daemon, small_policy());
    let lease = active_lease(&daemon);
    stream.bind(&lease, 1_000).unwrap();

    assert_eq!(stream.push(1_000, b"a".to_vec()), Ok(1));
    assert_eq!(stream.push(1_000, b"b".to_vec()), Ok(2));
    assert_eq!(stream.push(1_000, b"c".to_vec()), Ok(3));
    assert_eq!(stream.push(1_000, b"d".to_vec()), Ok(4));
    assert_eq!(stream.ack(4), Ok(()));

    // Ack beyond the last emitted sequence fails closed.
    assert_eq!(stream.ack(99), Err(EventStreamError::UnknownAck));

    // Replay outside the bounded window fails closed (window_start = 4-2 = 2).
    assert_eq!(
        stream.resume(1_000, 1),
        Err(EventStreamError::ReplayOutOfWindow)
    );
    // Replay from within the window succeeds (all events are acked, none returned).
    assert_eq!(stream.resume(1_000, 4), Ok(vec![]));
    // Replay beyond the last emitted sequence fails closed.
    assert_eq!(
        stream.resume(1_000, 5),
        Err(EventStreamError::ReplayOutOfWindow)
    );
}

#[test]
// @spec:AC-1463
fn buffer_is_bounded_by_items_and_total_bytes() {
    let daemon = bounded_daemon();
    let stream = EventStream::new(&daemon, small_policy());
    let lease = active_lease(&daemon);
    stream.bind(&lease, 1_000).unwrap();

    // max_event_payload = 64
    assert_eq!(
        stream.push(1_000, vec![0u8; 65]),
        Err(EventStreamError::PayloadTooLarge)
    );
    // max_buffered_events = 4
    assert_eq!(stream.push(1_000, b"1".to_vec()), Ok(1));
    assert_eq!(stream.push(1_000, b"2".to_vec()), Ok(2));
    assert_eq!(stream.push(1_000, b"3".to_vec()), Ok(3));
    assert_eq!(stream.push(1_000, b"4".to_vec()), Ok(4));
    assert_eq!(
        stream.push(1_000, b"5".to_vec()),
        Err(EventStreamError::BufferFull)
    );
    // Acking evicts and frees buffer capacity.
    assert_eq!(stream.ack(2), Ok(()));
    assert_eq!(stream.push(1_000, b"6".to_vec()), Ok(5));
    assert_eq!(stream.buffered_len(), 3);
}

#[test]
// @spec:AC-1463
fn buffer_normalizes_over_capacity_payload_vectors() {
    let daemon = bounded_daemon();
    let stream = EventStream::new(&daemon, small_policy());
    let lease = active_lease(&daemon);
    stream.bind(&lease, 1_000).unwrap();

    // A caller-owned Vec with huge capacity but tiny length must not inflate
    // the accounted byte budget (AC-1463). Four 1-byte payloads fit well
    // under the 256-byte budget only if capacity is normalized on admission.
    for i in 0..4 {
        let mut over_cap = Vec::with_capacity(1 << 20);
        over_cap.push(i as u8);
        assert_eq!(stream.push(1_000, over_cap), Ok(i as u64 + 1));
    }
    assert_eq!(stream.buffered_len(), 4);

    // Replayed payloads are bounded to the admitted logical bytes.
    let replayed = stream.resume(1_000, 0).unwrap();
    for (i, event) in replayed.iter().enumerate() {
        assert_eq!(event.sequence, i as u64 + 1);
        assert_eq!(event.payload(), &[i as u8]);
    }
}

#[test]
// @spec:AC-1463
fn buffer_enforces_total_byte_budget() {
    let daemon = bounded_daemon();
    // 4 events × 64 bytes each → total budget = 256 bytes.
    let stream = EventStream::new(&daemon, small_policy());
    let lease = active_lease(&daemon);
    stream.bind(&lease, 1_000).unwrap();

    // Fill exactly 256 bytes with 4 events of 64 bytes each.
    assert_eq!(stream.push(1_000, vec![0u8; 64]), Ok(1));
    assert_eq!(stream.push(1_000, vec![0u8; 64]), Ok(2));
    assert_eq!(stream.push(1_000, vec![0u8; 64]), Ok(3));
    assert_eq!(stream.push(1_000, vec![0u8; 64]), Ok(4));
    assert_eq!(
        stream.push(1_000, vec![0u8; 1]),
        Err(EventStreamError::BufferFull)
    );

    // Acking 2 events frees 128 bytes; admission of a 1-byte event succeeds.
    assert_eq!(stream.ack(2), Ok(()));
    assert_eq!(stream.push(1_000, vec![0u8; 1]), Ok(5));
}

#[test]
// @spec:AC-1464
fn reconnect_resumes_only_within_window() {
    let daemon = bounded_daemon();
    let stream = EventStream::new(&daemon, small_policy());
    let lease = active_lease(&daemon);
    stream.bind(&lease, 1_000).unwrap();

    for i in 0..3 {
        assert_eq!(stream.push(1_000, vec![i as u8]), Ok(i as u64 + 1));
    }
    assert_eq!(stream.ack(3), Ok(()));
    // Window = 2, acked = 3 => window_start = 1; resume from 0 is outside.
    assert_eq!(
        stream.resume(1_000, 0),
        Err(EventStreamError::ReplayOutOfWindow)
    );
    // Resume from acked point returns no unacknowledged events.
    assert_eq!(stream.resume(1_000, 3), Ok(vec![]));
}

#[test]
// @spec:AC-1465
fn events_never_contain_credential_material() {
    let daemon = bounded_daemon();
    let stream = EventStream::new(&daemon, small_policy());
    let lease = active_lease(&daemon);
    stream.bind(&lease, 1_000).unwrap();

    // Deterministic redaction boundary: sensitive material is rejected
    // before buffering, so it can never be delivered or replayed.
    assert_eq!(
        stream.push(1_000, b"cred_remote_1".to_vec()),
        Err(EventStreamError::SensitiveContent)
    );
    assert_eq!(
        stream.push(1_000, b"Bearer abc123".to_vec()),
        Err(EventStreamError::SensitiveContent)
    );
    assert_eq!(
        stream.push(1_000, b"api_key=secret-value".to_vec()),
        Err(EventStreamError::SensitiveContent)
    );
    assert_eq!(stream.buffered_len(), 0);

    // Non-sensitive payload is admitted.
    assert_eq!(stream.push(1_000, b"ok".to_vec()), Ok(1));
    let replayed = stream.resume(1_000, 0).unwrap();
    assert_eq!(replayed.len(), 1);
    assert_eq!(replayed[0].sequence, 1);
    assert_eq!(replayed[0].payload(), b"ok");

    // Audit trail never exposes the credential string.
    let audit = daemon.audit();
    assert!(!format!("{audit:?}").contains("cred_remote_1"));
}

#[test]
// @spec:AC-1462
fn policy_rejects_unbounded_values() {
    // Absolute ceilings defend construction against OOM panics.
    assert_eq!(
        EventStreamPolicy::bounded(usize::MAX, 64, 2),
        Err(EventStreamError::InvalidPolicy)
    );
    assert_eq!(
        EventStreamPolicy::bounded(4, usize::MAX, 2),
        Err(EventStreamError::InvalidPolicy)
    );
    assert_eq!(
        EventStreamPolicy::bounded(4, 64, usize::MAX),
        Err(EventStreamError::InvalidPolicy)
    );
    assert_eq!(
        EventStreamPolicy::bounded(0, 64, 2),
        Err(EventStreamError::InvalidPolicy)
    );
    // replay_window cannot exceed max_buffered_events.
    assert_eq!(
        EventStreamPolicy::bounded(2, 64, 4),
        Err(EventStreamError::InvalidPolicy)
    );
    // Valid small policy still constructs.
    assert!(EventStreamPolicy::bounded(4, 64, 2).is_ok());
}
