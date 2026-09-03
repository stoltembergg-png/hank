//! Authenticated remote daemon control-plane contracts.
//!
//! This crate is deliberately transport-neutral. It models only the fail-closed
//! bootstrap, binding, lease, revocation and redacted-audit boundaries needed
//! before a concrete daemon adapter may bind a socket. It does not open a
//! listener, handle raw secrets, or dispatch remote tools.

use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::{Handshake, NodeId, PeerId, ProtocolRevision};
use provider_core::CredentialRef;
use std::collections::{BTreeSet, VecDeque};
use std::sync::Mutex;
use thiserror::Error;

pub mod event_stream;

pub mod credential_broker;

/// Maximum retained redacted lifecycle events. Older events rotate out.
pub const MAX_AUDIT_EVENTS: usize = 256;

/// Authenticated peer identity produced by a concrete authentication adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPeer {
    pub peer: PeerId,
    pub node: NodeId,
}

impl AuthenticatedPeer {
    pub fn new(peer: &str, node: &str) -> Result<Self, DaemonError> {
        Ok(Self {
            peer: PeerId::new(peer).map_err(|_| DaemonError::AuthenticationDenied)?,
            node: NodeId::new(node).map_err(|_| DaemonError::AuthenticationDenied)?,
        })
    }
}

/// Injection seam for credential verification. It receives an opaque reference,
/// never secret material.
pub trait PeerAuthenticator: Send + Sync {
    fn authenticate(&self, credential: &CredentialRef) -> Result<AuthenticatedPeer, DaemonError>;
}

/// Exact identity binding, protocol support and bounded lease duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonPolicy {
    peer: PeerId,
    node: NodeId,
    project: ProjectId,
    lease_duration_ms: u64,
    supported_capabilities: BTreeSet<String>,
}

impl DaemonPolicy {
    pub fn exact(
        peer: &str,
        node: &str,
        project: ProjectId,
        lease_duration_ms: u64,
    ) -> Result<Self, DaemonError> {
        if lease_duration_ms == 0 {
            return Err(DaemonError::InvalidPolicy);
        }
        Ok(Self {
            peer: PeerId::new(peer).map_err(|_| DaemonError::InvalidPolicy)?,
            node: NodeId::new(node).map_err(|_| DaemonError::InvalidPolicy)?,
            project,
            lease_duration_ms,
            supported_capabilities: [String::from("observe")].into_iter().collect(),
        })
    }

    fn permits(&self, peer: &AuthenticatedPeer, handshake: &Handshake) -> bool {
        peer.peer == self.peer
            && peer.node == self.node
            && handshake.peer == self.peer
            && handshake.node == self.node
            && handshake.project == self.project
    }
}

/// Observable daemon-session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonSessionState {
    Ready,
    Closed,
}

/// Session lease returned only after authentication, protocol negotiation and
/// authorization succeed. Its ID binds later lifecycle cleanup to this lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonLease {
    pub id: u64,
    pub state: DaemonSessionState,
    pub expires_at_ms: u64,
}

/// Redacted audit reasons for daemon lifecycle and admission events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonAuditReason {
    Ready,
    AuthenticationDenied,
    AuthorizationDenied,
    ProtocolNegotiationDenied,
    SessionActive,
    Expired,
    Revoked,
    Stopped,
}

/// Redacted audit record. Peer/node are claimed identity for rejected requests
/// and authenticated identity only when `authenticated` is true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonAuditEvent {
    pub peer: Option<PeerId>,
    pub node: Option<NodeId>,
    pub project: ProjectId,
    pub revision: ProtocolRevision,
    pub reason: DaemonAuditReason,
    pub authenticated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DaemonError {
    #[error("remote daemon authentication denied")]
    AuthenticationDenied,
    #[error("remote daemon authorization denied")]
    AuthorizationDenied,
    #[error("remote daemon protocol negotiation denied")]
    ProtocolNegotiationDenied,
    #[error("remote daemon policy is invalid")]
    InvalidPolicy,
    #[error("remote daemon lease overflow")]
    LeaseOverflow,
    #[error("remote daemon session is already active")]
    SessionActive,
    #[error("remote daemon lease is stale")]
    StaleLease,
    #[error("remote daemon state lock unavailable")]
    StateUnavailable,
}

#[derive(Clone)]
struct ActiveSession {
    id: u64,
    peer: PeerId,
    node: NodeId,
    project: ProjectId,
    revision: ProtocolRevision,
    expires_at_ms: u64,
}

struct DaemonState {
    active: Option<ActiveSession>,
    next_lease_id: u64,
    last_closed_lease_id: Option<u64>,
    audit: VecDeque<DaemonAuditEvent>,
}

/// Bounded, in-memory authenticated daemon control plane.
pub struct AuthenticatedDaemon<A> {
    authenticator: A,
    policy: DaemonPolicy,
    state: Mutex<DaemonState>,
}

impl<A: PeerAuthenticator> AuthenticatedDaemon<A> {
    pub fn new(authenticator: A, policy: DaemonPolicy) -> Self {
        Self {
            authenticator,
            policy,
            state: Mutex::new(DaemonState {
                active: None,
                next_lease_id: 1,
                last_closed_lease_id: None,
                audit: VecDeque::with_capacity(MAX_AUDIT_EVENTS),
            }),
        }
    }

    /// Negotiates protocol, authenticates the opaque credential reference and
    /// authorizes the exact binding before making a daemon session Ready.
    pub fn bootstrap(
        &self,
        credential: Option<CredentialRef>,
        handshake: Handshake,
        now_ms: u64,
    ) -> Result<DaemonLease, DaemonError> {
        if handshake
            .clone()
            .negotiate(
                ProtocolRevision::V1_0,
                ProtocolRevision::V1_0,
                &self.policy.supported_capabilities,
            )
            .is_err()
        {
            self.record_attempt(
                &handshake,
                DaemonAuditReason::ProtocolNegotiationDenied,
                false,
            )?;
            return Err(DaemonError::ProtocolNegotiationDenied);
        }
        let credential = match credential {
            Some(credential) => credential,
            None => {
                self.record_attempt(&handshake, DaemonAuditReason::AuthenticationDenied, false)?;
                return Err(DaemonError::AuthenticationDenied);
            }
        };
        let peer = match self.authenticator.authenticate(&credential) {
            Ok(peer) => peer,
            Err(_) => {
                self.record_attempt(&handshake, DaemonAuditReason::AuthenticationDenied, false)?;
                return Err(DaemonError::AuthenticationDenied);
            }
        };
        if peer.peer != handshake.peer || peer.node != handshake.node {
            self.record_attempt(&handshake, DaemonAuditReason::AuthenticationDenied, false)?;
            return Err(DaemonError::AuthenticationDenied);
        }
        if !self.policy.permits(&peer, &handshake) {
            self.record_attempt(&handshake, DaemonAuditReason::AuthorizationDenied, true)?;
            return Err(DaemonError::AuthorizationDenied);
        }

        let expires_at_ms = now_ms
            .checked_add(self.policy.lease_duration_ms)
            .ok_or(DaemonError::LeaseOverflow)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DaemonError::StateUnavailable)?;
        close_expired(&mut state, now_ms);
        if state.active.is_some() {
            push_audit(
                &mut state,
                event_from_handshake(&handshake, DaemonAuditReason::SessionActive, true),
            );
            return Err(DaemonError::SessionActive);
        }
        let id = state.next_lease_id;
        state.next_lease_id = state
            .next_lease_id
            .checked_add(1)
            .ok_or(DaemonError::LeaseOverflow)?;
        let active = ActiveSession {
            id,
            peer: handshake.peer.clone(),
            node: handshake.node.clone(),
            project: handshake.project,
            revision: handshake.protocol,
            expires_at_ms,
        };
        push_audit(
            &mut state,
            event_from_active(&active, DaemonAuditReason::Ready),
        );
        state.active = Some(active);
        Ok(DaemonLease {
            id,
            state: DaemonSessionState::Ready,
            expires_at_ms,
        })
    }

    /// Reads state after atomically closing an expired active lease.
    pub fn session_state(&self, now_ms: u64) -> DaemonSessionState {
        let Ok(mut state) = self.state.lock() else {
            return DaemonSessionState::Closed;
        };
        close_expired(&mut state, now_ms);
        state_from(&state)
    }

    /// Closes the exact lease only when its deadline has been reached.
    pub fn expire(&self, lease_id: u64, now_ms: u64) -> Result<DaemonSessionState, DaemonError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DaemonError::StateUnavailable)?;
        let Some(active) = state.active.as_ref() else {
            // Idempotent only for the lease that was actually closed.
            if state.last_closed_lease_id == Some(lease_id) {
                return Ok(DaemonSessionState::Closed);
            }
            return Err(DaemonError::StaleLease);
        };
        if active.id != lease_id {
            return Err(DaemonError::StaleLease);
        }
        if now_ms < active.expires_at_ms {
            return Ok(DaemonSessionState::Ready);
        }
        close_active(&mut state, DaemonAuditReason::Expired);
        Ok(DaemonSessionState::Closed)
    }

    /// Revokes the exact active lease. Stale cleanup cannot close a replacement.
    pub fn revoke(&self, lease_id: u64) -> Result<DaemonSessionState, DaemonError> {
        self.close_lease(lease_id, DaemonAuditReason::Revoked)
    }

    /// Stops the exact active lease. Stale cleanup cannot close a replacement.
    pub fn stop(&self, lease_id: u64) -> Result<DaemonSessionState, DaemonError> {
        self.close_lease(lease_id, DaemonAuditReason::Stopped)
    }

    /// Returns bounded, redacted audit events in oldest-to-newest order.
    pub fn audit(&self) -> Vec<DaemonAuditEvent> {
        self.state
            .lock()
            .map(|state| state.audit.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn close_lease(
        &self,
        lease_id: u64,
        reason: DaemonAuditReason,
    ) -> Result<DaemonSessionState, DaemonError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DaemonError::StateUnavailable)?;
        let Some(active) = state.active.as_ref() else {
            // Idempotent only for the lease that was actually closed.
            if state.last_closed_lease_id == Some(lease_id) {
                return Ok(DaemonSessionState::Closed);
            }
            return Err(DaemonError::StaleLease);
        };
        if active.id != lease_id {
            return Err(DaemonError::StaleLease);
        }
        close_active(&mut state, reason);
        Ok(DaemonSessionState::Closed)
    }

    fn record_attempt(
        &self,
        handshake: &Handshake,
        reason: DaemonAuditReason,
        authenticated: bool,
    ) -> Result<(), DaemonError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DaemonError::StateUnavailable)?;
        push_audit(
            &mut state,
            event_from_handshake(handshake, reason, authenticated),
        );
        Ok(())
    }
}

fn state_from(state: &DaemonState) -> DaemonSessionState {
    if state.active.is_some() {
        DaemonSessionState::Ready
    } else {
        DaemonSessionState::Closed
    }
}

fn close_expired(state: &mut DaemonState, now_ms: u64) {
    if state
        .active
        .as_ref()
        .is_some_and(|active| now_ms >= active.expires_at_ms)
    {
        close_active(state, DaemonAuditReason::Expired);
    }
}

fn close_active(state: &mut DaemonState, reason: DaemonAuditReason) {
    if let Some(active) = state.active.take() {
        state.last_closed_lease_id = Some(active.id);
        push_audit(state, event_from_active(&active, reason));
    }
}

fn event_from_active(active: &ActiveSession, reason: DaemonAuditReason) -> DaemonAuditEvent {
    DaemonAuditEvent {
        peer: Some(active.peer.clone()),
        node: Some(active.node.clone()),
        project: active.project,
        revision: active.revision,
        reason,
        authenticated: true,
    }
}

fn event_from_handshake(
    handshake: &Handshake,
    reason: DaemonAuditReason,
    authenticated: bool,
) -> DaemonAuditEvent {
    DaemonAuditEvent {
        peer: PeerId::new(&handshake.peer.0).ok(),
        node: NodeId::new(&handshake.node.0).ok(),
        project: handshake.project,
        revision: handshake.protocol,
        reason,
        authenticated,
    }
}

fn push_audit(state: &mut DaemonState, event: DaemonAuditEvent) {
    if state.audit.len() == MAX_AUDIT_EVENTS {
        state.audit.pop_front();
    }
    state.audit.push_back(event);
}
