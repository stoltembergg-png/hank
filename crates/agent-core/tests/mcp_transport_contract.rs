use agent_core::mcp_transport::*;

fn valid() -> Envelope {
    Envelope::new(
        ProtocolVersion::V1,
        7,
        FrameKind::Request,
        64,
        CapabilitySet::READ | CapabilitySet::CANCEL,
    )
    .unwrap()
}

// @spec:AC-1377
#[test]
fn supported_handshake_and_frame_preserve_correlation() {
    let frame = Transport::accept(valid()).unwrap();
    assert_eq!(frame.correlation_id(), 7);
    assert!(frame.capabilities().contains(CapabilitySet::READ));
    assert!(matches!(
        Transport::accept({
            let mut e = valid();
            e.version = ProtocolVersion::Unknown;
            e
        }),
        Err(TransportError::UnsupportedVersion)
    ));
    assert!(matches!(
        Transport::accept({
            let mut e = valid();
            e.size = 65;
            e
        }),
        Err(TransportError::FrameTooLarge)
    ));
}

// @spec:AC-1378
#[test]
fn lifecycle_is_idempotent_and_queue_fail_closed() {
    let mut session = Session::new(1, 1);
    assert_eq!(session.cancel(), SessionState::Cancelled);
    assert_eq!(session.cancel(), SessionState::Cancelled);
    assert_eq!(session.close(), SessionState::Closed);
    assert_eq!(session.enqueue(), Err(TransportError::Backpressure));
    assert_eq!(session.reconnect(), Err(TransportError::ReconnectDenied));
}
