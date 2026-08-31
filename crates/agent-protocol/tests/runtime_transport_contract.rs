use agent_protocol::runtime_transport::*;

fn envelope() -> RuntimeEnvelope {
    RuntimeEnvelope::new(
        ProtocolVersion::V1,
        ConnectionId::new("conn-1").unwrap(),
        SessionId::new("session-1").unwrap(),
        7,
        FrameKind::Request,
        32,
        CapabilitySet::CANCEL,
    )
    .unwrap()
}

#[test]
// @spec:AC-1401
fn framing_validates_identity_version_and_size() {
    let accepted = RuntimeTransport::accept(envelope()).unwrap();
    assert_eq!(accepted.correlation_id(), 7);
    assert!(RuntimeEnvelope::new(
        ProtocolVersion::Unknown,
        ConnectionId::new("c").unwrap(),
        SessionId::new("s").unwrap(),
        1,
        FrameKind::Request,
        1,
        CapabilitySet::empty(),
    )
    .is_ok());
    let oversized = RuntimeEnvelope::new(
        ProtocolVersion::V1,
        ConnectionId::new("c").unwrap(),
        SessionId::new("s").unwrap(),
        1,
        FrameKind::Request,
        65_537,
        CapabilitySet::empty(),
    )
    .unwrap();
    assert_eq!(
        RuntimeTransport::accept(oversized),
        Err(TransportError::FrameTooLarge)
    );
}

#[test]
// @spec:AC-1402
fn session_lifecycle_is_idempotent_and_bounded() {
    let mut session = RuntimeSession::new(1, 2).unwrap();
    assert!(session.enqueue().is_ok());
    assert_eq!(session.enqueue(), Err(TransportError::Backpressure));
    assert_eq!(session.cancel(), SessionState::Cancelled);
    assert_eq!(session.cancel(), SessionState::Cancelled);
    assert_eq!(session.close(), SessionState::Closed);
    assert_eq!(session.close(), SessionState::Closed);
    assert_eq!(session.reconnect(), Err(TransportError::ReconnectDenied));
}
