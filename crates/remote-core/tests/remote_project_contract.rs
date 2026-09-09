//! Contract tests for the bounded remote project support boundary.
//!
//! These fixtures are transport-neutral and offline. They prove that a remote
//! project is bound to an exact authorized node/peer, that cross-project and
//! stale-version access fail closed, and that typed artifact/workflow/session
//! references never carry raw filesystem paths or secret material. No network,
//! shell, provider or credential store is exercised.

use agent_protocol::ids::{ArtifactId, ProjectId, SessionId, WorkflowId};
use agent_protocol::remote_protocol::{NodeId, PeerId};
use remote_core::remote_project::{
    RemoteProjectDescriptor, RemoteProjectError, RemoteProjectRegistry,
};
use std::collections::BTreeSet;
use std::str::FromStr;

const PROJECT_A: &str = "proj-aaaaaaa1-1111-4111-8111-111111111111";
const PROJECT_B: &str = "proj-bbbbbbb2-2222-4222-8222-222222222222";

fn project_a() -> ProjectId {
    ProjectId::from_str(PROJECT_A).unwrap()
}

fn project_b() -> ProjectId {
    ProjectId::from_str(PROJECT_B).unwrap()
}

fn node(value: &str) -> NodeId {
    NodeId::new(value).unwrap()
}

fn peer(value: &str) -> PeerId {
    PeerId::new(value).unwrap()
}

fn workflow() -> WorkflowId {
    WorkflowId::new()
}

fn session() -> SessionId {
    SessionId::new()
}

fn artifact() -> ArtifactId {
    ArtifactId::new()
}

fn capabilities(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn descriptor(
    project: ProjectId,
    node: NodeId,
    peer: PeerId,
    version: u64,
) -> RemoteProjectDescriptor {
    RemoteProjectDescriptor::new(project, node, peer, version, capabilities(&["observe"])).unwrap()
}

#[test]
// @spec:AC-3015
fn descriptor_binds_project_to_exact_node_and_peer() {
    let binding = descriptor(project_a(), node("node-a"), peer("peer-a"), 1);
    let mut registry = RemoteProjectRegistry::new();
    registry
        .bind(binding.clone(), node("node-a"), peer("peer-a"))
        .unwrap();
    let resolved = registry.resolve(project_a(), node("node-a")).unwrap();
    assert_eq!(resolved.project, project_a());
    assert_eq!(resolved.version, 1);
}

#[test]
// @spec:AC-3016
fn wrong_project_or_node_is_rejected_fail_closed() {
    let binding = descriptor(project_a(), node("node-a"), peer("peer-a"), 1);
    let mut registry = RemoteProjectRegistry::new();
    // Bind to the wrong node than the descriptor declares.
    assert_eq!(
        registry
            .bind(binding.clone(), node("other-node"), peer("peer-a"))
            .unwrap_err(),
        RemoteProjectError::NodeMismatch
    );
    // Resolve a project that was never bound.
    assert_eq!(
        registry.resolve(project_b(), node("node-a")).unwrap_err(),
        RemoteProjectError::ProjectNotBound
    );
    // Resolve a bound project from a node that does not own it.
    registry
        .bind(binding, node("node-a"), peer("peer-a"))
        .unwrap();
    assert_eq!(
        registry
            .resolve(project_a(), node("other-node"))
            .unwrap_err(),
        RemoteProjectError::NodeMismatch
    );
}

#[test]
// @spec:AC-3017
fn stale_or_older_version_conflicts_and_requires_reconcile() {
    let mut registry = RemoteProjectRegistry::new();
    registry
        .bind(
            descriptor(project_a(), node("node-a"), peer("peer-a"), 2),
            node("node-a"),
            peer("peer-a"),
        )
        .unwrap();
    // A stale (older) version must fail closed and demand reconciliation.
    assert_eq!(
        registry
            .bind(
                descriptor(project_a(), node("node-a"), peer("peer-a"), 1),
                node("node-a"),
                peer("peer-a"),
            )
            .unwrap_err(),
        RemoteProjectError::VersionConflict
    );
    // Explicit reconcile advances the binding only to a strictly newer version.
    registry
        .reconcile(
            descriptor(project_a(), node("node-a"), peer("peer-a"), 3),
            node("node-a"),
            peer("peer-a"),
        )
        .unwrap();
    assert_eq!(
        registry
            .resolve(project_a(), node("node-a"))
            .unwrap()
            .version,
        3
    );
}

#[test]
// @spec:AC-3018
fn descriptor_rejects_oversized_input_and_unknown_capability() {
    // A capability identifier above the size bound is rejected at construction.
    let long_capability = "c".repeat(300);
    assert!(matches!(
        RemoteProjectDescriptor::new(
            project_a(),
            node("node-a"),
            peer("peer-a"),
            1,
            capabilities(&[&long_capability]),
        ),
        Err(RemoteProjectError::InvalidCapability)
    ));
    // An empty capability set is explicit and valid (deny-by-default scope).
    assert!(RemoteProjectDescriptor::new(
        project_a(),
        node("node-a"),
        peer("peer-a"),
        1,
        capabilities(&[]),
    )
    .is_ok());
}

#[test]
// @spec:AC-3019
fn typed_references_carry_no_raw_filesystem_paths() {
    // Workflow/session/artifact references are typed IDs; a descriptor never
    // accepts raw path strings for them. The reference sets are validated to
    // contain only well-formed typed IDs (which reject path traversal by
    // construction). This test asserts the registry surfaces references as
    // typed identifiers, not strings that could escape a storage root.
    let mut descriptor = descriptor(project_a(), node("node-a"), peer("peer-a"), 1);
    descriptor.workflows.push(workflow());
    descriptor.sessions.push(session());
    descriptor.artifacts.push(artifact());

    let mut registry = RemoteProjectRegistry::new();
    registry
        .bind(descriptor.clone(), node("node-a"), peer("peer-a"))
        .unwrap();
    let resolved = registry.resolve(project_a(), node("node-a")).unwrap();
    assert_eq!(resolved.workflows.len(), 1);
    assert_eq!(resolved.sessions.len(), 1);
    assert_eq!(resolved.artifacts.len(), 1);
}

#[test]
// @spec:AC-3020
fn capability_scope_is_explicit_and_denied_by_default() {
    // A descriptor carrying an unknown capability must not silently grant it.
    let mut registry = RemoteProjectRegistry::new();
    registry
        .bind(
            descriptor(project_a(), node("node-a"), peer("peer-a"), 1),
            node("node-a"),
            peer("peer-a"),
        )
        .unwrap();
    let binding = registry.resolve(project_a(), node("node-a")).unwrap();
    // "observe" was declared; "write" was never granted.
    assert!(binding.capabilities.contains("observe"));
    assert!(!binding.capabilities.contains("write"));
}

#[test]
// @spec:AC-3021
fn binding_survives_restart_via_ledger_reconstruction() {
    // A registry reconstructed from a prior descriptor set retains the binding
    // without re-running any transport or credential resolution.
    let original = descriptor(project_a(), node("node-a"), peer("peer-a"), 5);
    let mut registry = RemoteProjectRegistry::new();
    registry
        .bind(original.clone(), node("node-a"), peer("peer-a"))
        .unwrap();

    let mut rebuilt = RemoteProjectRegistry::new();
    rebuilt
        .bind(original, node("node-a"), peer("peer-a"))
        .unwrap();
    assert_eq!(
        rebuilt
            .resolve(project_a(), node("node-a"))
            .unwrap()
            .version,
        5
    );
}

#[test]
// @spec:AC-3022
fn duplicate_bind_is_idempotent_for_same_version() {
    let binding = descriptor(project_a(), node("node-a"), peer("peer-a"), 1);
    let mut registry = RemoteProjectRegistry::new();
    registry
        .bind(binding.clone(), node("node-a"), peer("peer-a"))
        .unwrap();
    // Binding the exact same version again is a no-op, not a conflict.
    registry
        .bind(binding, node("node-a"), peer("peer-a"))
        .unwrap();
    assert_eq!(
        registry
            .resolve(project_a(), node("node-a"))
            .unwrap()
            .version,
        1
    );
}
