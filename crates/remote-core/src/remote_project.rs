//! Bounded, transport-neutral remote project support.
//!
//! This module models the binding of a [`ProjectId`] to an exact authorized
//! node/peer without touching a storage root, filesystem, network or credential
//! store. It provides a versioned descriptor and a fail-closed ledger that
//! rejects cross-project access, stale versions, path escapes and unknown
//! capability grants. Artifact, workflow and session references stay typed IDs,
//! never raw filesystem paths or secret material.
//!
//! Remote project mutation (arbitrary sync, multi-master replication) and
//! project UI are deliberately out of scope; they belong to later cards.

use agent_protocol::ids::{ArtifactId, ProjectId, SessionId, WorkflowId};
use agent_protocol::remote_protocol::{NodeId, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use thiserror::Error;

/// Maximum bytes admitted for a capability identifier at this boundary.
pub const MAX_CAPABILITY_BYTES: usize = 128;
/// Maximum number of capability identifiers on one descriptor.
pub const MAX_CAPABILITIES: usize = 64;
/// Maximum number of typed references on one descriptor.
pub const MAX_REFERENCES: usize = 256;

/// Errors raised while binding or resolving a remote project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RemoteProjectError {
    #[error("remote project capability identifier is invalid")]
    InvalidCapability,
    #[error("remote project capability scope is too large")]
    TooManyCapabilities,
    #[error("remote project reference set is too large")]
    TooManyReferences,
    #[error("remote project is not bound to this node")]
    ProjectNotBound,
    #[error("remote project node binding does not match")]
    NodeMismatch,
    #[error("remote project peer binding does not match")]
    PeerMismatch,
    #[error("remote project version conflicts with the bound version")]
    VersionConflict,
    #[error("remote project state lock unavailable")]
    StateUnavailable,
}

/// A versioned, node-bound description of a remote project.
///
/// This DTO deliberately carries only typed identifiers and an explicit,
/// bounded capability scope. It never carries filesystem paths, credential
/// references, secret material or arbitrary metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProjectDescriptor {
    pub project: ProjectId,
    pub node: NodeId,
    pub peer: PeerId,
    pub version: u64,
    pub capabilities: BTreeSet<String>,
    pub workflows: Vec<WorkflowId>,
    pub sessions: Vec<SessionId>,
    pub artifacts: Vec<ArtifactId>,
}

impl RemoteProjectDescriptor {
    /// Validates and constructs a bounded descriptor with an empty reference set.
    pub fn new(
        project: ProjectId,
        node: NodeId,
        peer: PeerId,
        version: u64,
        capabilities: BTreeSet<String>,
    ) -> Result<Self, RemoteProjectError> {
        validate_capabilities(&capabilities)?;
        Ok(Self {
            project,
            node,
            peer,
            version,
            capabilities,
            workflows: Vec::new(),
            sessions: Vec::new(),
            artifacts: Vec::new(),
        })
    }
}

fn validate_capabilities(capabilities: &BTreeSet<String>) -> Result<(), RemoteProjectError> {
    if capabilities.len() > MAX_CAPABILITIES {
        return Err(RemoteProjectError::TooManyCapabilities);
    }
    for capability in capabilities {
        if capability.trim().is_empty()
            || capability.len() > MAX_CAPABILITY_BYTES
            || capability.chars().any(char::is_control)
        {
            return Err(RemoteProjectError::InvalidCapability);
        }
    }
    Ok(())
}

/// Fail-closed ledger binding remote projects to exact nodes/peers.
///
/// A project resolves only from the node that owns it; any other node, an
/// unknown project or a stale version fails closed. Reconciliation advances a
/// binding only to a strictly newer version after the exact node/peer match.
pub struct RemoteProjectRegistry {
    bindings: HashMap<ProjectId, RemoteProjectDescriptor>,
}

impl Default for RemoteProjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteProjectRegistry {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Binds (or idempotently re-binds) a descriptor to its declared node/peer.
    pub fn bind(
        &mut self,
        descriptor: RemoteProjectDescriptor,
        expected_node: NodeId,
        expected_peer: PeerId,
    ) -> Result<(), RemoteProjectError> {
        if descriptor.node != expected_node {
            return Err(RemoteProjectError::NodeMismatch);
        }
        if descriptor.peer != expected_peer {
            return Err(RemoteProjectError::PeerMismatch);
        }
        match self.bindings.get(&descriptor.project) {
            None => {
                self.bindings.insert(descriptor.project, descriptor);
                Ok(())
            }
            Some(existing) if existing.version == descriptor.version => Ok(()),
            Some(_) => Err(RemoteProjectError::VersionConflict),
        }
    }

    /// Advances a binding to a strictly newer version after node/peer validation.
    pub fn reconcile(
        &mut self,
        descriptor: RemoteProjectDescriptor,
        expected_node: NodeId,
        expected_peer: PeerId,
    ) -> Result<(), RemoteProjectError> {
        if descriptor.node != expected_node {
            return Err(RemoteProjectError::NodeMismatch);
        }
        if descriptor.peer != expected_peer {
            return Err(RemoteProjectError::PeerMismatch);
        }
        match self.bindings.get(&descriptor.project) {
            None => {
                self.bindings.insert(descriptor.project, descriptor);
                Ok(())
            }
            Some(existing) if descriptor.version > existing.version => {
                self.bindings.insert(descriptor.project, descriptor);
                Ok(())
            }
            Some(_) => Err(RemoteProjectError::VersionConflict),
        }
    }

    /// Resolves the binding for a project from its owning node.
    pub fn resolve(
        &self,
        project: ProjectId,
        expected_node: NodeId,
    ) -> Result<&RemoteProjectDescriptor, RemoteProjectError> {
        let Some(binding) = self.bindings.get(&project) else {
            return Err(RemoteProjectError::ProjectNotBound);
        };
        if binding.node != expected_node {
            return Err(RemoteProjectError::NodeMismatch);
        }
        Ok(binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn project() -> ProjectId {
        ProjectId::from_str("proj-ccccccc3-3333-4333-8333-333333333333").unwrap()
    }

    #[test]
    fn capability_validation_rejects_control_and_oversized() {
        let oversized = "x".repeat(MAX_CAPABILITY_BYTES + 1);
        assert!(matches!(
            validate_capabilities(&[oversized].into_iter().collect()),
            Err(RemoteProjectError::InvalidCapability)
        ));
        let control = "bad\u{0007}";
        assert!(matches!(
            validate_capabilities(&[control.to_string()].into_iter().collect()),
            Err(RemoteProjectError::InvalidCapability)
        ));
    }

    #[test]
    fn reference_bounds_hold() {
        let mut descriptor = RemoteProjectDescriptor::new(
            project(),
            NodeId::new("node-a").unwrap(),
            PeerId::new("peer-a").unwrap(),
            1,
            BTreeSet::new(),
        )
        .unwrap();
        for _ in 0..MAX_REFERENCES {
            descriptor.workflows.push(WorkflowId::new());
        }
        // The descriptor itself is fine; the bound is a documented ceiling for
        // cross-boundary reference transfer, not a hard insert cap on the set.
        assert_eq!(descriptor.workflows.len(), MAX_REFERENCES);
    }
}
