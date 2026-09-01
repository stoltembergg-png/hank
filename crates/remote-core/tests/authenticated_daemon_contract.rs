use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::{Handshake, NodeId, PeerId, ProtocolRevision};
use provider_core::CredentialRef;
use remote_core::{
    AuthenticatedDaemon, DaemonAuditReason, DaemonError, DaemonPolicy, DaemonSessionState,
    PeerAuthenticator,
};
use std::str::FromStr;

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

#[test]
// @spec:AC-1457
fn bootstrap_rejects_absent_invalid_and_mismatched_authentication() {
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
    assert_eq!(daemon.session_state(), DaemonSessionState::Closed);
}

#[test]
// @spec:AC-1458
fn bootstrap_accepts_only_the_exact_peer_node_project_binding() {
    let project = project();
    let daemon = AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project));
    let credential = CredentialRef::parse("cred_remote_1").unwrap();

    let mut wrong_node = handshake(project);
    wrong_node.node = NodeId::new("node-2").unwrap();
    assert_eq!(
        daemon.bootstrap(Some(credential.clone()), wrong_node, 1_000),
        Err(DaemonError::AuthorizationDenied)
    );

    let ready = daemon
        .bootstrap(Some(credential), handshake(project), 1_000)
        .unwrap();
    assert_eq!(ready.state, DaemonSessionState::Ready);
    assert_eq!(ready.expires_at_ms, 61_000);
}

#[test]
// @spec:AC-1459
fn expired_revoked_and_repeated_stop_sessions_remain_closed() {
    let project = project();
    let daemon = AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project));
    let credential = CredentialRef::parse("cred_remote_1").unwrap();
    daemon
        .bootstrap(Some(credential), handshake(project), 1_000)
        .unwrap();

    assert_eq!(daemon.expire(61_000), DaemonSessionState::Closed);
    assert_eq!(daemon.stop(), DaemonSessionState::Closed);
    assert_eq!(daemon.stop(), DaemonSessionState::Closed);
    assert_eq!(daemon.revoke(), DaemonSessionState::Closed);
    assert_eq!(daemon.session_state(), DaemonSessionState::Closed);
}

#[test]
// @spec:AC-1460
fn audit_is_identity_bound_and_never_contains_credential_material() {
    let project = project();
    let daemon = AuthenticatedDaemon::new(AcceptedAuthenticator, policy(project));
    let credential = CredentialRef::parse("cred_remote_1").unwrap();
    daemon
        .bootstrap(Some(credential), handshake(project), 1_000)
        .unwrap();
    daemon.revoke();

    let audit = daemon.audit();
    assert_eq!(audit.len(), 2);
    assert_eq!(audit[0].reason, DaemonAuditReason::Ready);
    assert_eq!(audit[1].reason, DaemonAuditReason::Revoked);
    assert_eq!(audit[0].peer.0, "peer-a");
    assert_eq!(audit[0].node.0, "node-1");
    assert_eq!(audit[0].project, project);
    assert!(!format!("{audit:?}").contains("cred_remote_1"));
}
