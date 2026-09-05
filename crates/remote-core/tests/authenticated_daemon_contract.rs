use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::{Handshake, NodeId, PeerId, ProtocolRevision};
use provider_core::CredentialRef;
use remote_core::{
    AuthenticatedDaemon, DaemonAuditReason, DaemonError, DaemonPolicy, DaemonSessionState,
    PeerAuthenticator, MAX_AUDIT_EVENTS,
};
use security_core::{RateLimitPolicy, RateLimiter};
use std::str::FromStr;
use std::sync::Arc;

struct AcceptedAuthenticator;

impl PeerAuthenticator for AcceptedAuthenticator {
    fn authenticate(
        &self,
        credential: &CredentialRef,
    ) -> Result<remote_core::AuthenticatedPeer, DaemonError> {
        if credential.as_str() != "cred_remote_1" {
            return Err(DaemonError::AuthenticationDenied);
        }
        Ok(remote_core::AuthenticatedPeer::new("peer-a", "node-1").unwrap())
    }
}

struct MismatchedAuthenticator;

impl PeerAuthenticator for MismatchedAuthenticator {
    fn authenticate(
        &self,
        _credential: &CredentialRef,
    ) -> Result<remote_core::AuthenticatedPeer, DaemonError> {
        Ok(remote_core::AuthenticatedPeer::new("peer-b", "node-2").unwrap())
    }
}

fn project() -> ProjectId {
    ProjectId::from_str("proj-11111111-1111-4111-8111-111111111111").unwrap()
}

fn handshake(project: ProjectId) -> Handshake {
    Handshake {
        protocol: ProtocolRevision::V1_0,
        api: ProtocolRevision::V1_0,
        peer: PeerId::new("peer-a").unwrap(),
        node: NodeId::new("node-1").unwrap(),
        project,
        capabilities: ["observe"].into_iter().map(String::from).collect(),
    }
}

fn policy(project: ProjectId) -> DaemonPolicy {
    DaemonPolicy::exact("peer-a", "node-1", project, 60_000).unwrap()
}

fn credential() -> CredentialRef {
    CredentialRef::parse("cred_remote_1").unwrap()
}

#[test]
// @spec:AC-2577
fn authenticated_remote_bootstrap_is_rate_limited_by_bound_node_scope() {
    let project = project();
    let limiter = Arc::new(
        RateLimiter::new(RateLimitPolicy::new("remote-policy-1", 1, 1, 1_000, 1, 8, 4).unwrap())
            .unwrap(),
    );
    let mismatch = AuthenticatedDaemon::new_with_rate_limiter(
        MismatchedAuthenticator,
        policy(project),
        limiter.clone(),
    );
    assert_eq!(
        mismatch.bootstrap(Some(credential()), handshake(project), 1_000),
        Err(DaemonError::AuthenticationDenied)
    );
    let daemon =
        AuthenticatedDaemon::new_with_rate_limiter(AcceptedAuthenticator, policy(project), limiter);

    let first = daemon
        .bootstrap(Some(credential()), handshake(project), 1_000)
        .unwrap();
    assert_eq!(daemon.stop(first.id), Ok(DaemonSessionState::Closed));

    assert_eq!(
        daemon.bootstrap(Some(credential()), handshake(project), 1_001),
        Err(DaemonError::RateLimited {
            retry_after_ms: 1_000
        })
    );
    assert_eq!(
        daemon.audit().last().unwrap().reason,
        DaemonAuditReason::RateLimited
    );
}

#[test]
// @spec:AC-1457
fn bootstrap_rejects_missing_invalid_and_mismatched_authentication_with_audit() {
    let project = project();
    let daemon = AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project));

    assert_eq!(
        daemon.bootstrap(None, handshake(project), 1_000),
        Err(DaemonError::AuthenticationDenied)
    );
    assert_eq!(
        daemon.bootstrap(
            Some(CredentialRef::parse("cred_wrong").unwrap()),
            handshake(project),
            1_000,
        ),
        Err(DaemonError::AuthenticationDenied)
    );

    let mismatch = AuthenticatedDaemon::new(MismatchedAuthenticator, policy(project));
    assert_eq!(
        mismatch.bootstrap(Some(credential()), handshake(project), 1_000),
        Err(DaemonError::AuthenticationDenied)
    );
    assert_eq!(mismatch.session_state(1_000), DaemonSessionState::Closed);
    assert_eq!(
        mismatch.audit()[0].reason,
        DaemonAuditReason::AuthenticationDenied
    );
    assert!(!mismatch.audit()[0].authenticated);
}

#[test]
// @spec:AC-1458
fn bootstrap_requires_exact_binding_and_negotiated_protocol() {
    let project = project();
    let daemon = AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project));

    let mut wrong_node = handshake(project);
    wrong_node.node = NodeId::new("node-2").unwrap();
    assert_eq!(
        daemon.bootstrap(Some(credential()), wrong_node, 1_000),
        Err(DaemonError::AuthenticationDenied)
    );

    let wrong_project = ProjectId::from_str("proj-22222222-2222-4222-8222-222222222222").unwrap();
    assert_eq!(
        daemon.bootstrap(Some(credential()), handshake(wrong_project), 1_000),
        Err(DaemonError::AuthorizationDenied)
    );

    let mut unsupported_protocol = handshake(project);
    unsupported_protocol.protocol = ProtocolRevision {
        major: 99,
        minor: 0,
    };
    assert_eq!(
        daemon.bootstrap(Some(credential()), unsupported_protocol, 1_000),
        Err(DaemonError::ProtocolNegotiationDenied)
    );

    let ready = daemon
        .bootstrap(Some(credential()), handshake(project), 1_000)
        .unwrap();
    assert_eq!(ready.state, DaemonSessionState::Ready);
    assert_eq!(ready.expires_at_ms, 61_000);
}

#[test]
// @spec:AC-1459
fn expired_leases_reopen_safely_and_stale_cleanup_cannot_close_replacement() {
    let project = project();
    let daemon = AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project));
    let first = daemon
        .bootstrap(Some(credential()), handshake(project), 1_000)
        .unwrap();

    // Bootstrap atomically expires the old lease and admits a valid replacement.
    let second = daemon
        .bootstrap(Some(credential()), handshake(project), 61_000)
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(daemon.session_state(61_000), DaemonSessionState::Ready);

    // Stale cleanup for a superseded lease cannot close the replacement.
    assert_eq!(daemon.stop(first.id), Err(DaemonError::StaleLease));
    assert_eq!(daemon.session_state(61_000), DaemonSessionState::Ready);
    // Repeated cleanup of an already-closed lease stays Closed (AC-1459).
    assert_eq!(daemon.revoke(second.id), Ok(DaemonSessionState::Closed));
    assert_eq!(daemon.stop(second.id), Ok(DaemonSessionState::Closed));
    assert_eq!(
        daemon.expire(second.id, 61_000),
        Ok(DaemonSessionState::Closed)
    );
    assert_eq!(daemon.session_state(61_000), DaemonSessionState::Closed);
}

#[test]
// @spec:AC-1460
fn audit_is_bounded_redacted_and_records_rejected_attempts() {
    let project = project();
    let daemon = AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project));

    for now_ms in 0..=(MAX_AUDIT_EVENTS as u64) {
        let lease = daemon
            .bootstrap(Some(credential()), handshake(project), now_ms * 100_000)
            .unwrap();
        assert_eq!(daemon.stop(lease.id), Ok(DaemonSessionState::Closed));
    }
    assert_eq!(
        daemon.bootstrap(None, handshake(project), 9_999_999),
        Err(DaemonError::AuthenticationDenied)
    );

    let audit = daemon.audit();
    assert_eq!(audit.len(), MAX_AUDIT_EVENTS);
    assert_eq!(
        audit.last().unwrap().reason,
        DaemonAuditReason::AuthenticationDenied
    );
    assert!(!audit.last().unwrap().authenticated);
    assert_eq!(audit.last().unwrap().peer.as_ref().unwrap().0, "peer-a");
    assert_eq!(audit.last().unwrap().node.as_ref().unwrap().0, "node-1");
    assert_eq!(audit.last().unwrap().project, project);
    assert!(!format!("{audit:?}").contains("cred_remote_1"));
}
