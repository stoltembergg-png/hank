//! Authenticated remote daemon control-plane contracts.
//!
//! This crate is deliberately transport-neutral. It models only the fail-closed
//! bootstrap, binding, lease, revocation and redacted-audit boundaries needed
//! before a concrete daemon adapter may bind a socket. It does not open a
//! listener, handle raw secrets, or dispatch remote tools.

use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::{Handshake, NodeId, PeerId, ProtocolRevision};
use provider_core::CredentialRef;
use std::sync::Mutex;
use thiserror::Error;

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

/// Exact identity binding and bounded lease duration for a daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonPolicy {
    peer: PeerId,
    node: NodeId,
    project: ProjectId,
    lease_duration_ms: u64,
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

/// Session lease returned only after authentication and authorization succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonLease {
    pub state: DaemonSessionState,
    pub expires_at_ms: u64,
}

/// Redacted audit reasons for daemon lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonAuditReason {
    Ready,
    Expired,
    Revoked,
    Stopped,
}

/// Audit record that intentionally has no credential or token field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonAuditEvent {
    pub peer: PeerId,
    pub node: NodeId,
    pub project: ProjectId,
    pub revision: ProtocolRevision,
    pub reason: DaemonAuditReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DaemonError {
    #[error("remote daemon authentication denied")]
    AuthenticationDenied,
    #[error("remote daemon authorization denied")]
    AuthorizationDenied,
    #[error("remote daemon policy is invalid")]
    InvalidPolicy,
    #[error("remote daemon lease overflow")]
    LeaseOverflow,
    #[error("remote daemon session is already active")]
    SessionActive,
    #[error("remote daemon state lock unavailable")]
    StateUnavailable,
}

#[derive(Clone)]
struct ActiveSession {
    peer: PeerId,
    node: NodeId,
    project: ProjectId,
    revision: ProtocolRevision,
    expires_at_ms: u64,
}

struct DaemonState {
    active: Option<ActiveSession>,
    audit: Vec<DaemonAuditEvent>,
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
                audit: Vec::new(),
            }),
        }
    }

    /// Authenticates a credential reference and authorizes the exact binding
    /// before making a daemon session Ready.
    pub fn bootstrap(
        &self,
        credential: Option<CredentialRef>,
        handshake: Handshake,
        now_ms: u64,
    ) -> Result<DaemonLease, DaemonError> {
        let credential = credential.ok_or(DaemonError::AuthenticationDenied)?;
        let peer = self.authenticator.authenticate(&credential)?;
        if !self.policy.permits(&peer, &handshake) {
            return Err(DaemonError::AuthorizationDenied);
        }
        let expires_at_ms = now_ms
            .checked_add(self.policy.lease_duration_ms)
            .ok_or(DaemonError::LeaseOverflow)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DaemonError::StateUnavailable)?;
        if state.active.is_some() {
            return Err(DaemonError::SessionActive);
        }
        let active = ActiveSession {
            peer: handshake.peer.clone(),
            node: handshake.node.clone(),
            project: handshake.project,
            revision: handshake.protocol,
            expires_at_ms,
        };
        state.audit.push(DaemonAuditEvent {
            peer: active.peer.clone(),
            node: active.node.clone(),
            project: active.project,
            revision: active.revision,
            reason: DaemonAuditReason::Ready,
        });
        state.active = Some(active);
        Ok(DaemonLease {
            state: DaemonSessionState::Ready,
            expires_at_ms,
        })
    }

    pub fn session_state(&self) -> DaemonSessionState {
        self.state
            .lock()
            .map(|state| {
                if state.active.is_some() {
                    DaemonSessionState::Ready
                } else {
                    DaemonSessionState::Closed
                }
            })
            .unwrap_or(DaemonSessionState::Closed)
    }

    /// Closes the current lease when its deadline has been reached.
    pub fn expire(&self, now_ms: u64) -> DaemonSessionState {
        self.close_if(
            |active| now_ms >= active.expires_at_ms,
            DaemonAuditReason::Expired,
        )
    }

    /// Revokes the active lease. Repeated calls remain Closed.
    pub fn revoke(&self) -> DaemonSessionState {
        self.close_if(|_| true, DaemonAuditReason::Revoked)
    }

    /// Stops the active lease. Repeated calls remain Closed.
    pub fn stop(&self) -> DaemonSessionState {
        self.close_if(|_| true, DaemonAuditReason::Stopped)
    }

    pub fn audit(&self) -> Vec<DaemonAuditEvent> {
        self.state
            .lock()
            .map(|state| state.audit.clone())
            .unwrap_or_default()
    }

    fn close_if(
        &self,
        should_close: impl FnOnce(&ActiveSession) -> bool,
        reason: DaemonAuditReason,
    ) -> DaemonSessionState {
        let Ok(mut state) = self.state.lock() else {
            return DaemonSessionState::Closed;
        };
        let Some(active) = state.active.as_ref() else {
            return DaemonSessionState::Closed;
        };
        if !should_close(active) {
            return DaemonSessionState::Ready;
        }
        let active = state.active.take().expect("active session checked");
        state.audit.push(DaemonAuditEvent {
            peer: active.peer,
            node: active.node,
            project: active.project,
            revision: active.revision,
            reason,
        });
        DaemonSessionState::Closed
    }
}
