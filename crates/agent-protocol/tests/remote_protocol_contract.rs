use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::*;
use std::collections::BTreeSet;
use std::str::FromStr;

fn project(id: &str) -> ProjectId {
    ProjectId::from_str(id).unwrap()
}

fn handshake(
    protocol: ProtocolRevision,
    api: ProtocolRevision,
    peer: &str,
    node: &str,
    capabilities: &[&str],
) -> Handshake {
    Handshake {
        protocol,
        api,
        peer: PeerId::new(peer).unwrap(),
        node: NodeId::new(node).unwrap(),
        project: project("proj-11111111-1111-4111-8111-111111111111"),
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
    }
}

fn supported() -> BTreeSet<String> {
    ["run", "observe", "cancel"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[test]
// @spec:AC-1453
fn handshake_negotiates_protocol_api_and_capabilities() {
    let ok = handshake(
        ProtocolRevision::V1_0,
        ProtocolRevision::V1_0,
        "peer-a",
        "node-1",
        &["run", "cancel"],
    )
    .negotiate(ProtocolRevision::V1_0, ProtocolRevision::V1_0, &supported())
    .unwrap();
    assert_eq!(ok.protocol, ProtocolRevision::V1_0);
    assert_eq!(ok.api, ProtocolRevision::V1_0);
    assert_eq!(ok.peer.0.as_str(), "peer-a");
    assert_eq!(ok.node.0.as_str(), "node-1");
    assert!(ok.capabilities.contains("run"));
    assert!(ok.capabilities.contains("cancel"));
    assert!(!ok.capabilities.contains("observe"));

    // Unknown major version rejected.
    let unknown_major = handshake(
        ProtocolRevision {
            major: 99,
            minor: 0,
        },
        ProtocolRevision::V1_0,
        "peer-a",
        "node-1",
        &["run"],
    )
    .negotiate(ProtocolRevision::V1_0, ProtocolRevision::V1_0, &supported());
    assert_eq!(unknown_major, Err(ProtocolError::UnsupportedProtocol));

    // Incompatible API revision rejected.
    let bad_api = handshake(
        ProtocolRevision::V1_0,
        ProtocolRevision { major: 2, minor: 0 },
        "peer-a",
        "node-1",
        &["run"],
    )
    .negotiate(ProtocolRevision::V1_0, ProtocolRevision::V1_0, &supported());
    assert_eq!(bad_api, Err(ProtocolError::UnsupportedApi));

    // Unknown capability rejected — negotiated, not granted.
    let unknown_cap = handshake(
        ProtocolRevision::V1_0,
        ProtocolRevision::V1_0,
        "peer-a",
        "node-1",
        &["run", "unsafe-admin"],
    )
    .negotiate(ProtocolRevision::V1_0, ProtocolRevision::V1_0, &supported());
    assert_eq!(unknown_cap, Err(ProtocolError::UnknownCapability));
}

#[test]
// @spec:AC-1454
fn command_catalog_is_typed_and_rejects_unknown() {
    let catalog = CommandCatalog::default_v1();
    assert!(catalog.lookup("ping").unwrap().idempotent);
    assert!(catalog.lookup("get_state").unwrap().idempotent);
    assert!(!catalog.lookup("subscribe").unwrap().idempotent);
    assert!(catalog.lookup("cancel").unwrap().idempotent);
    assert_eq!(
        catalog.lookup("rm-rf-remote"),
        Err(ProtocolError::UnknownCommand)
    );
}

#[test]
// @spec:AC-1455
fn correlation_tracking_is_fail_closed() {
    let mut tracker = RequestTracker::new(4);

    // Pending begin succeeds.
    assert!(tracker.begin(7).is_ok());
    // Duplicate pending begin rejected.
    assert_eq!(tracker.begin(7), Err(ProtocolError::DuplicateCorrelation));

    // Unknown correlation rejected on complete/cancel.
    assert_eq!(
        tracker.complete(999),
        Err(ProtocolError::UnknownCorrelation)
    );
    assert_eq!(tracker.cancel(998), Err(ProtocolError::UnknownCorrelation));

    // Complete then cancel -> stale.
    assert!(tracker.complete(7).is_ok());
    assert_eq!(tracker.cancel(7), Err(ProtocolError::StaleCorrelation));

    // Cancel then complete -> stale.
    assert!(tracker.begin(8).is_ok());
    assert!(tracker.cancel(8).is_ok());
    assert_eq!(tracker.complete(8), Err(ProtocolError::StaleCorrelation));

    // Event sequence must advance monotonically.
    let mut events = EventSequence::new();
    assert!(events.accept(1).is_ok());
    assert!(events.accept(2).is_ok());
    assert_eq!(events.accept(2), Err(ProtocolError::OutOfOrder));
    assert_eq!(events.accept(1), Err(ProtocolError::OutOfOrder));
    assert!(events.accept(3).is_ok());
}

#[test]
// @spec:AC-1456
fn identity_mismatch_and_oversized_payload_are_rejected() {
    let expected = ExpectedIdentity::new("peer-a", "node-1").unwrap();

    let mismatched_peer = handshake(
        ProtocolRevision::V1_0,
        ProtocolRevision::V1_0,
        "peer-EVIL",
        "node-1",
        &["run"],
    );
    assert_eq!(
        expected.verify(&mismatched_peer),
        Err(ProtocolError::IdentityMismatch)
    );

    let mismatched_node = handshake(
        ProtocolRevision::V1_0,
        ProtocolRevision::V1_0,
        "peer-a",
        "node-2",
        &["run"],
    );
    assert_eq!(
        expected.verify(&mismatched_node),
        Err(ProtocolError::IdentityMismatch)
    );

    let matching = handshake(
        ProtocolRevision::V1_0,
        ProtocolRevision::V1_0,
        "peer-a",
        "node-1",
        &["run"],
    );
    assert!(expected.verify(&matching).is_ok());

    // Oversized serialized payload rejected.
    let oversized = vec![b'x'; MAX_PAYLOAD + 1];
    assert_eq!(
        PayloadBound::check(&oversized),
        Err(ProtocolError::PayloadTooLarge)
    );
    let bounded = vec![b'x'; MAX_PAYLOAD];
    assert!(PayloadBound::check(&bounded).is_ok());
}
