//! Contract tests for the bounded remote tool dispatch boundary.
//!
//! These fixtures are transport-neutral and offline. They prove the dispatch
//! policy without claiming a production WebSocket/TLS or tool-host runtime.

use agent_core::budget::BudgetLimits;
use agent_protocol::ids::{OperationKey, ProjectId, TraceId};
use agent_protocol::remote_protocol::{Handshake, NodeId, PeerId, ProtocolRevision};
use provider_core::{CancellationToken, CredentialRef};
use remote_core::tool_dispatch::{
    RemoteDispatchError, RemoteOperationStatus, RemoteToolDispatcher, RemoteToolFuture,
    RemoteToolPolicy, RemoteToolRequest, RemoteToolTransport, RemoteTransportError,
    MAX_REMOTE_OPERATIONS, REMOTE_OPERATION_RETENTION_MS,
};
use remote_core::{AuthenticatedDaemon, DaemonError, DaemonPolicy, PeerAuthenticator};
use serde_json::json;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::sync::Notify;
use tool_core::{
    PermissionEvaluator, PermissionRequest, PolicyDecision, ToolContext, ToolEffect, ToolOutcome,
    ToolRequest, ToolResponse,
};

const PROJECT: &str = "proj-11111111-1111-4111-8111-111111111111";

fn fixture_credential_value() -> String {
    ["cred", "_remote", "_fixture"].concat()
}

fn fixture_credential() -> CredentialRef {
    CredentialRef::parse(fixture_credential_value()).unwrap()
}

struct AcceptedAuthenticator;

impl PeerAuthenticator for AcceptedAuthenticator {
    fn authenticate(
        &self,
        credential: &CredentialRef,
    ) -> Result<remote_core::AuthenticatedPeer, DaemonError> {
        if credential.as_str() == fixture_credential_value() {
            Ok(remote_core::AuthenticatedPeer::new("peer-a", "node-1").unwrap())
        } else {
            Err(DaemonError::AuthenticationDenied)
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FixtureMode {
    Success,
    Unavailable,
    SensitiveResponse,
    MismatchedResponse,
}

struct FixtureTransport {
    calls: AtomicUsize,
    mode: Mutex<FixtureMode>,
    last_request: Mutex<Option<RemoteToolRequest>>,
}

impl FixtureTransport {
    fn new(mode: FixtureMode) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            mode: Mutex::new(mode),
            last_request: Mutex::new(None),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn last_request(&self) -> RemoteToolRequest {
        self.last_request
            .lock()
            .unwrap()
            .clone()
            .expect("fixture received a request")
    }
}

impl RemoteToolTransport for FixtureTransport {
    fn execute<'a>(
        &'a self,
        request: RemoteToolRequest,
        cancellation: CancellationToken,
    ) -> RemoteToolFuture<'a> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last_request.lock().unwrap() = Some(request.clone());
        let mode = *self.mode.lock().unwrap();
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(RemoteTransportError::Cancelled);
            }
            match mode {
                FixtureMode::Success => Ok(response_for(&request, json!({"ok": true}))),
                FixtureMode::Unavailable => Err(RemoteTransportError::Unavailable),
                FixtureMode::SensitiveResponse => Ok(response_for(
                    &request,
                    json!({"credential": fixture_credential_value()}),
                )),
                FixtureMode::MismatchedResponse => {
                    let mut response = response_for(&request, json!({"ok": true}));
                    response.operation_key = OperationKey::new();
                    Ok(response)
                }
            }
        })
    }

    fn cancel(
        &self,
        _target: &remote_core::tool_dispatch::RemoteToolTarget,
        _operation_key: OperationKey,
    ) -> Result<(), RemoteTransportError> {
        Ok(())
    }
}

struct BlockingTransport {
    started: Arc<Notify>,
    release: Arc<Notify>,
    calls: AtomicUsize,
}

impl BlockingTransport {
    fn new() -> Self {
        Self {
            started: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
            calls: AtomicUsize::new(0),
        }
    }
}

impl RemoteToolTransport for BlockingTransport {
    fn execute<'a>(
        &'a self,
        request: RemoteToolRequest,
        _cancellation: CancellationToken,
    ) -> RemoteToolFuture<'a> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let started = Arc::clone(&self.started);
        let release = Arc::clone(&self.release);
        Box::pin(async move {
            started.notify_one();
            release.notified().await;
            Ok(response_for(&request, json!({"ok": true})))
        })
    }

    fn cancel(
        &self,
        _target: &remote_core::tool_dispatch::RemoteToolTarget,
        _operation_key: OperationKey,
    ) -> Result<(), RemoteTransportError> {
        Ok(())
    }
}

struct HangingTransport {
    calls: AtomicUsize,
}

impl HangingTransport {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl RemoteToolTransport for HangingTransport {
    fn execute<'a>(
        &'a self,
        request: RemoteToolRequest,
        _cancellation: CancellationToken,
    ) -> RemoteToolFuture<'a> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(response_for(&request, json!({"ok": true})))
        })
    }

    fn cancel(
        &self,
        _target: &remote_core::tool_dispatch::RemoteToolTarget,
        _operation_key: OperationKey,
    ) -> Result<(), RemoteTransportError> {
        Ok(())
    }
}

fn project() -> ProjectId {
    ProjectId::from_str(PROJECT).unwrap()
}

fn handshake() -> Handshake {
    Handshake {
        protocol: ProtocolRevision::V1_0,
        api: ProtocolRevision::V1_0,
        peer: PeerId::new("peer-a").unwrap(),
        node: NodeId::new("node-1").unwrap(),
        project: project(),
        capabilities: [String::from("observe")].into_iter().collect(),
    }
}

fn dispatcher<T: RemoteToolTransport + 'static>(
    transport: Arc<T>,
) -> RemoteToolDispatcher<AcceptedAuthenticator, T> {
    dispatcher_with_lease_duration(transport, 60_000)
}

fn dispatcher_with_lease_duration<T: RemoteToolTransport + 'static>(
    transport: Arc<T>,
    lease_duration_ms: u64,
) -> RemoteToolDispatcher<AcceptedAuthenticator, T> {
    let daemon = Arc::new(AuthenticatedDaemon::new(
        AcceptedAuthenticator,
        DaemonPolicy::exact("peer-a", "node-1", project(), lease_duration_ms).unwrap(),
    ));
    let policy = RemoteToolPolicy::bounded(1_000, 5_000, 4_096, 4_096)
        .unwrap()
        .allow_tool("fixture.echo", "1.0.0", "observe")
        .unwrap();
    RemoteToolDispatcher::new(
        daemon,
        transport,
        Arc::new(PermissionEvaluator::new()),
        policy,
    )
}

fn request() -> ToolRequest {
    ToolRequest {
        operation_key: OperationKey::new(),
        tool_name: "fixture.echo".into(),
        tool_version: "1.0.0".into(),
        input: json!({"value": "bounded"}),
        context: ToolContext {
            project_id: project(),
            agent_id: None,
            session_id: None,
            task_id: None,
            workflow_id: None,
            capability: "observe".into(),
            policy_decision: PolicyDecision::Allow,
            budget_limits: BudgetLimits::default(),
            reservation_id: None,
            trace_id: TraceId::new(),
            metadata: BTreeMap::new(),
        },
        timeout_seconds: Some(1),
        metadata: BTreeMap::new(),
    }
}

fn permission(request: &ToolRequest) -> PermissionRequest {
    PermissionRequest {
        project_id: Some(request.context.project_id),
        tool_name: request.tool_name.clone(),
        tool_version: request.tool_version.clone(),
        capability: request.context.capability.clone(),
        effect: ToolEffect::Read,
        policy: PolicyDecision::Allow,
        budget_available: true,
        confirmation_approved: false,
    }
}

fn response_for(request: &RemoteToolRequest, payload: serde_json::Value) -> ToolResponse {
    ToolResponse {
        operation_key: request.operation_key,
        tool_name: request.tool_name.clone(),
        tool_version: request.tool_version.clone(),
        outcome: ToolOutcome::Success,
        payload,
        trace_id: request.trace_id,
        duration_ms: 1,
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
// @spec:AC-3007
async fn permitted_tool_executes_on_exact_node_and_replays_cached_result() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let request = request();
    let permission = permission(&request);

    let first = dispatcher
        .dispatch(
            lease.id,
            1_000,
            NodeId::new("node-1").unwrap(),
            request.clone(),
            permission.clone(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    let second = dispatcher
        .dispatch(
            lease.id,
            2_000,
            NodeId::new("node-1").unwrap(),
            request,
            permission,
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(first.operation_key, second.operation_key);
    assert_eq!(transport.calls(), 1);
    assert_eq!(
        transport.last_request().target.node,
        NodeId::new("node-1").unwrap()
    );
    assert_eq!(
        dispatcher.status(first.operation_key),
        Some(RemoteOperationStatus::Completed)
    );
}

#[tokio::test]
// @spec:AC-3007
async fn permitted_tool_rejects_conflicting_operation_key_without_reuse() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let first_request = request();
    let operation_key = first_request.operation_key;
    let first_permission = permission(&first_request);
    dispatcher
        .dispatch(
            lease.id,
            1_000,
            NodeId::new("node-1").unwrap(),
            first_request.clone(),
            first_permission,
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let mut conflicting_request = first_request;
    conflicting_request.input = json!({"value": "different"});
    let conflicting_permission = permission(&conflicting_request);
    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                conflicting_request,
                conflicting_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::OperationConflict)
    );
    assert_eq!(transport.calls(), 1);
    assert_eq!(
        dispatcher.status(operation_key),
        Some(RemoteOperationStatus::Completed)
    );
}

#[tokio::test]
// @spec:AC-3008
async fn wrong_node_or_permission_is_rejected_before_transport() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let request = request();
    let base_permission = permission(&request);

    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-2").unwrap(),
                request.clone(),
                base_permission.clone(),
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::TargetMismatch)
    );
    let mut denied = base_permission;
    denied.policy = PolicyDecision::Deny;
    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                request.clone(),
                denied,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::PermissionDenied)
    );

    let mut project_request = request.clone();
    project_request.context.project_id =
        ProjectId::from_str("proj-22222222-2222-4222-8222-222222222222").unwrap();
    let project_permission = permission(&project_request);
    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                project_request,
                project_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::TargetMismatch)
    );

    let mut capability_request = request;
    capability_request.context.capability = "write".into();
    let capability_permission = permission(&capability_request);
    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                capability_request,
                capability_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::ToolNotAllowed)
    );
    assert_eq!(transport.calls(), 0);
}

#[tokio::test]
// @spec:AC-3009
async fn payload_and_sensitive_material_are_rejected_at_boundary() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let mut sensitive_request = request();
    sensitive_request
        .metadata
        .insert("credential".into(), "forbidden".into());
    let sensitive_permission = permission(&sensitive_request);

    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                sensitive_request,
                sensitive_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::SensitiveMaterial)
    );

    let mut oversized = request();
    oversized
        .metadata
        .insert("padding".into(), "x".repeat(8_000));
    let oversized_permission = permission(&oversized);
    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                oversized,
                oversized_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::PayloadTooLarge)
    );
    assert_eq!(transport.calls(), 0);
}

#[tokio::test]
// @spec:AC-3009
async fn payload_boundary_excludes_internal_tool_context() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let request = request();
    let permission = permission(&request);
    dispatcher
        .dispatch(
            lease.id,
            1_000,
            NodeId::new("node-1").unwrap(),
            request,
            permission,
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let encoded = serde_json::to_value(transport.last_request()).unwrap();
    assert!(encoded.get("tool").is_none());
    assert!(encoded.get("context").is_none());
    assert!(encoded.get("policy_decision").is_none());
    assert!(encoded.get("budget_limits").is_none());
    assert!(encoded.get("reservation_id").is_none());
    assert_eq!(encoded["capability"], "observe");
    assert!(encoded.get("input").is_some());
}

#[tokio::test]
// @spec:AC-3007
async fn permitted_tool_terminal_records_expire_before_capacity_is_permanent() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let dispatcher =
        dispatcher_with_lease_duration(transport.clone(), REMOTE_OPERATION_RETENTION_MS + 60_000);
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();

    for _ in 0..MAX_REMOTE_OPERATIONS {
        let request = request();
        let operation_permission = permission(&request);
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                request,
                operation_permission,
                CancellationToken::new(),
            )
            .await
            .unwrap();
    }

    let full_request = request();
    let full_permission = permission(&full_request);
    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                full_request,
                full_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::Capacity)
    );

    let retained_request = request();
    let retained_permission = permission(&retained_request);
    assert!(dispatcher
        .dispatch(
            lease.id,
            1_000 + REMOTE_OPERATION_RETENTION_MS + 1,
            NodeId::new("node-1").unwrap(),
            retained_request,
            retained_permission,
            CancellationToken::new(),
        )
        .await
        .is_ok());
}

#[tokio::test]
// @spec:AC-3010
async fn timeout_or_transport_loss_becomes_unknown_and_is_not_retried() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Unavailable));
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let request = request();
    let operation_key = request.operation_key;
    let permission = permission(&request);

    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                request.clone(),
                permission.clone(),
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::UnknownOutcome)
    );
    assert_eq!(
        dispatcher.status(operation_key),
        Some(RemoteOperationStatus::Unknown)
    );
    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                request,
                permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::UnknownOutcome)
    );
    assert_eq!(transport.calls(), 1);
}

#[tokio::test]
// @spec:AC-3010
async fn timeout_or_transport_loss_transport_deadline_is_enforced() {
    let transport = Arc::new(HangingTransport::new());
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let request = request();
    let operation_key = request.operation_key;
    let permission = permission(&request);

    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                request,
                permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::UnknownOutcome)
    );
    assert_eq!(
        dispatcher.status(operation_key),
        Some(RemoteOperationStatus::Unknown)
    );
    assert_eq!(transport.calls(), 1);
}

#[tokio::test]
// @spec:AC-3011
async fn cancellation_before_dispatch_has_no_remote_effect() {
    let transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let dispatcher = dispatcher(transport.clone());
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let request = request();
    let permission = permission(&request);
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    assert_eq!(
        dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                request,
                permission,
                cancellation,
            )
            .await,
        Err(RemoteDispatchError::Cancelled)
    );
    assert_eq!(transport.calls(), 0);
}

#[tokio::test]
// @spec:AC-3012
async fn invalid_or_sensitive_response_is_unknown() {
    for mode in [
        FixtureMode::SensitiveResponse,
        FixtureMode::MismatchedResponse,
    ] {
        let transport = Arc::new(FixtureTransport::new(mode));
        let dispatcher = dispatcher(transport);
        let lease = dispatcher
            .bootstrap(Some(fixture_credential()), handshake(), 1_000)
            .unwrap();
        let request = request();
        let permission = permission(&request);
        assert_eq!(
            dispatcher
                .dispatch(
                    lease.id,
                    1_000,
                    NodeId::new("node-1").unwrap(),
                    request,
                    permission,
                    CancellationToken::new(),
                )
                .await,
            Err(RemoteDispatchError::UnknownOutcome)
        );
    }
}

#[tokio::test]
// @spec:AC-3013
async fn revoked_or_expired_lease_cannot_dispatch() {
    let revoked_transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let revoked_dispatcher = dispatcher(revoked_transport.clone());
    let revoked_lease = revoked_dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let revoked_request = request();
    let revoked_permission = permission(&revoked_request);
    revoked_dispatcher.revoke(revoked_lease.id).unwrap();
    assert!(matches!(
        revoked_dispatcher
            .dispatch(
                revoked_lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                revoked_request,
                revoked_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::Lease(DaemonError::StaleLease))
    ));
    assert_eq!(revoked_transport.calls(), 0);

    let expired_transport = Arc::new(FixtureTransport::new(FixtureMode::Success));
    let expired_dispatcher = dispatcher(expired_transport.clone());
    let expired_lease = expired_dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let expired_request = request();
    let expired_permission = permission(&expired_request);
    assert!(matches!(
        expired_dispatcher
            .dispatch(
                expired_lease.id,
                expired_lease.expires_at_ms + 1,
                NodeId::new("node-1").unwrap(),
                expired_request,
                expired_permission,
                CancellationToken::new(),
            )
            .await,
        Err(RemoteDispatchError::Lease(DaemonError::StaleLease))
    ));
    assert_eq!(expired_transport.calls(), 0);
}

#[tokio::test]
// @spec:AC-3014
async fn in_flight_cancellation_is_terminal_and_late_result_becomes_unknown() {
    let transport = Arc::new(BlockingTransport::new());
    let dispatcher = Arc::new(dispatcher(transport.clone()));
    let lease = dispatcher
        .bootstrap(Some(fixture_credential()), handshake(), 1_000)
        .unwrap();
    let request = request();
    let operation_key = request.operation_key;
    let permission = permission(&request);
    let task_dispatcher = Arc::clone(&dispatcher);
    let task = tokio::spawn(async move {
        task_dispatcher
            .dispatch(
                lease.id,
                1_000,
                NodeId::new("node-1").unwrap(),
                request,
                permission,
                CancellationToken::new(),
            )
            .await
    });

    transport.started.notified().await;
    assert_eq!(dispatcher.cancel(lease.id, 1_000, operation_key), Ok(()));
    transport.release.notify_one();

    assert_eq!(
        task.await.unwrap(),
        Err(RemoteDispatchError::UnknownOutcome)
    );
    assert_eq!(
        dispatcher.status(operation_key),
        Some(RemoteOperationStatus::Unknown)
    );
    assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
}
