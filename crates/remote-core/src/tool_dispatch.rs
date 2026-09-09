//! Bounded remote-tool dispatch contract.
//!
//! This module closes the gap between an authenticated remote lease and a
//! typed tool request without pretending to be a network implementation. A
//! concrete adapter supplies [`RemoteToolTransport`]; the core owns identity,
//! permission, payload, timeout, idempotency, cancellation and unknown-outcome
//! rules. Tests use an in-memory fixture and never open a socket or resolve a
//! real credential.

use crate::{AuthenticatedDaemon, DaemonError, DaemonLeaseContext, PeerAuthenticator};
use agent_protocol::ids::{OperationKey, ProjectId, TraceId};
use agent_protocol::remote_protocol::{NodeId, PeerId, ProtocolRevision, MAX_PAYLOAD};
use provider_core::CancellationToken;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tool_core::{
    PermissionDecision, PermissionEvaluator, PermissionRequest, ToolOutcome, ToolRequest,
    ToolResponse,
};

/// Absolute timeout ceiling for one remote tool request.
pub const MAX_REMOTE_TIMEOUT_MS: u64 = 5 * 60 * 1_000;
/// Maximum number of operation records retained by one dispatcher.
pub const MAX_REMOTE_OPERATIONS: usize = 256;
/// Terminal-operation retention window before capacity can be reclaimed.
pub const REMOTE_OPERATION_RETENTION_MS: u64 = 10 * 60 * 1_000;
/// Maximum identifier length admitted at this boundary.
pub const MAX_REMOTE_IDENTIFIER_BYTES: usize = 128;

/// Stable identity of a tool capability that a remote policy permits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RemoteToolIdentity {
    pub name: String,
    pub version: String,
    pub capability: String,
}

impl RemoteToolIdentity {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        capability: impl Into<String>,
    ) -> Result<Self, RemoteToolPolicyError> {
        let identity = Self {
            name: name.into(),
            version: version.into(),
            capability: capability.into(),
        };
        if !valid_identifier(&identity.name)
            || !valid_identifier(&identity.version)
            || !valid_identifier(&identity.capability)
        {
            return Err(RemoteToolPolicyError::InvalidToolIdentity);
        }
        Ok(identity)
    }
}

/// Explicit allowlist and resource bounds for remote tool dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteToolPolicy {
    allowed_tools: BTreeSet<RemoteToolIdentity>,
    default_timeout_ms: u64,
    max_timeout_ms: u64,
    max_request_bytes: usize,
    max_response_bytes: usize,
}

impl RemoteToolPolicy {
    pub fn bounded(
        default_timeout_ms: u64,
        max_timeout_ms: u64,
        max_request_bytes: usize,
        max_response_bytes: usize,
    ) -> Result<Self, RemoteToolPolicyError> {
        if default_timeout_ms == 0
            || max_timeout_ms == 0
            || default_timeout_ms > max_timeout_ms
            || max_timeout_ms > MAX_REMOTE_TIMEOUT_MS
            || max_request_bytes == 0
            || max_request_bytes > MAX_PAYLOAD
            || max_response_bytes == 0
            || max_response_bytes > MAX_PAYLOAD
        {
            return Err(RemoteToolPolicyError::InvalidPolicy);
        }
        Ok(Self {
            allowed_tools: BTreeSet::new(),
            default_timeout_ms,
            max_timeout_ms,
            max_request_bytes,
            max_response_bytes,
        })
    }

    pub fn allow_tool(
        mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        capability: impl Into<String>,
    ) -> Result<Self, RemoteToolPolicyError> {
        self.allowed_tools
            .insert(RemoteToolIdentity::new(name, version, capability)?);
        Ok(self)
    }

    fn allows(&self, request: &ToolRequest) -> bool {
        self.allowed_tools.contains(&RemoteToolIdentity {
            name: request.tool_name.clone(),
            version: request.tool_version.clone(),
            capability: request.context.capability.clone(),
        })
    }

    fn timeout_ms(&self, request: &ToolRequest) -> Result<u64, RemoteDispatchError> {
        let timeout_ms = match request.timeout_seconds {
            Some(seconds) if seconds > 0 => seconds
                .checked_mul(1_000)
                .ok_or(RemoteDispatchError::TimeoutExceeded)?,
            Some(_) => return Err(RemoteDispatchError::TimeoutExceeded),
            None => self.default_timeout_ms,
        };
        if timeout_ms > self.max_timeout_ms {
            return Err(RemoteDispatchError::TimeoutExceeded);
        }
        Ok(timeout_ms)
    }
}

/// Errors constructing a remote tool policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RemoteToolPolicyError {
    #[error("remote tool policy is invalid")]
    InvalidPolicy,
    #[error("remote tool identity is invalid")]
    InvalidToolIdentity,
}

/// Authenticated target carried to a concrete transport adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteToolTarget {
    pub lease_id: u64,
    pub peer: PeerId,
    pub node: NodeId,
    pub project: ProjectId,
    pub revision: ProtocolRevision,
}

impl From<DaemonLeaseContext> for RemoteToolTarget {
    fn from(context: DaemonLeaseContext) -> Self {
        Self {
            lease_id: context.lease.id,
            peer: context.peer,
            node: context.node,
            project: context.project,
            revision: context.revision,
        }
    }
}

/// Typed, bounded request handed to the concrete remote transport.
///
/// This DTO deliberately excludes `ToolContext`, policy decisions, budgets,
/// reservation handles and arbitrary metadata. The remote host receives only
/// the fields required to resolve and execute the already-authorized tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteToolRequest {
    pub target: RemoteToolTarget,
    pub operation_key: OperationKey,
    pub tool_name: String,
    pub tool_version: String,
    pub capability: String,
    pub input: serde_json::Value,
    pub trace_id: TraceId,
    pub deadline_ms: u64,
}

/// Future returned by an injected remote transport.
pub type RemoteToolFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ToolResponse, RemoteTransportError>> + Send + 'a>>;

/// Transport seam for a remote node. Implementations may use WebSocket/TLS,
/// but the core never opens a listener, shell, provider or credential store.
pub trait RemoteToolTransport: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: RemoteToolRequest,
        cancellation: CancellationToken,
    ) -> RemoteToolFuture<'a>;

    /// Cancels an in-flight operation. `Ok(())` means the adapter guarantees
    /// that the operation cannot complete after this point.
    fn cancel(
        &self,
        target: &RemoteToolTarget,
        operation_key: OperationKey,
    ) -> Result<(), RemoteTransportError>;
}

/// Errors reported by a concrete transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RemoteTransportError {
    #[error("remote transport rejected the request before execution")]
    Rejected,
    #[error("remote transport timed out after dispatch")]
    Timeout,
    #[error("remote transport cancelled after dispatch")]
    Cancelled,
    #[error("remote transport became unavailable after dispatch")]
    Unavailable,
    #[error("remote transport returned an invalid protocol result")]
    InvalidResponse,
}

/// Observable idempotency state of a remote operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteOperationStatus {
    InFlight,
    Completed,
    Cancelled,
    Rejected,
    Unknown,
}

/// Fail-closed dispatch errors. `UnknownOutcome` is terminal for the operation:
/// callers must reconcile with the node instead of retrying blindly.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RemoteDispatchError {
    #[error("remote tool request is invalid")]
    InvalidRequest,
    #[error("remote tool permission denied")]
    PermissionDenied,
    #[error("remote tool is not allowlisted")]
    ToolNotAllowed,
    #[error("remote tool target does not match the authenticated lease")]
    TargetMismatch,
    #[error("remote tool lease is invalid: {0}")]
    Lease(#[from] DaemonError),
    #[error("remote tool timeout is outside the bounded policy")]
    TimeoutExceeded,
    #[error("remote tool payload exceeds the bounded policy")]
    PayloadTooLarge,
    #[error("remote tool payload contains sensitive material")]
    SensitiveMaterial,
    #[error("remote tool operation is already in flight")]
    DuplicateOperation,
    #[error("remote tool operation key conflicts with a different request")]
    OperationConflict,
    #[error("remote tool operation was cancelled")]
    Cancelled,
    #[error("remote tool operation has an unknown outcome")]
    UnknownOutcome,
    #[error("remote tool operation was rejected before execution")]
    TransportRejected,
    #[error("remote tool operation is not known")]
    OperationNotFound,
    #[error("remote tool operation already completed")]
    AlreadyCompleted,
    #[error("remote tool operation ledger is full")]
    Capacity,
    #[error("remote tool state lock unavailable")]
    StateUnavailable,
}

enum OperationState {
    InFlight,
    Completed(ToolResponse),
    Cancelled,
    Rejected,
    Unknown,
}

struct OperationRecord {
    state: OperationState,
    fingerprint: [u8; 32],
    target: RemoteToolTarget,
    updated_at_ms: u64,
}

/// Authenticated, policy-checked remote tool dispatcher.
pub struct RemoteToolDispatcher<A, T> {
    daemon: Arc<AuthenticatedDaemon<A>>,
    transport: Arc<T>,
    permissions: Arc<PermissionEvaluator>,
    policy: RemoteToolPolicy,
    operations: Mutex<HashMap<OperationKey, OperationRecord>>,
}

impl<A: PeerAuthenticator, T: RemoteToolTransport> RemoteToolDispatcher<A, T> {
    pub fn new(
        daemon: Arc<AuthenticatedDaemon<A>>,
        transport: Arc<T>,
        permissions: Arc<PermissionEvaluator>,
        policy: RemoteToolPolicy,
    ) -> Self {
        Self {
            daemon,
            transport,
            permissions,
            policy,
            operations: Mutex::new(HashMap::new()),
        }
    }

    /// Convenience bootstrap retained in the composition boundary; it does not
    /// bypass the daemon's authentication or exact identity checks.
    pub fn bootstrap(
        &self,
        credential: Option<provider_core::CredentialRef>,
        handshake: agent_protocol::remote_protocol::Handshake,
        now_ms: u64,
    ) -> Result<crate::DaemonLease, DaemonError> {
        self.daemon.bootstrap(credential, handshake, now_ms)
    }

    pub fn revoke(&self, lease_id: u64) -> Result<crate::DaemonSessionState, DaemonError> {
        self.daemon.revoke(lease_id)
    }

    /// Dispatches one typed request to the exact authenticated node.
    pub async fn dispatch(
        &self,
        lease_id: u64,
        now_ms: u64,
        target_node: NodeId,
        request: ToolRequest,
        permission: PermissionRequest,
        cancellation: CancellationToken,
    ) -> Result<ToolResponse, RemoteDispatchError> {
        if cancellation.is_cancelled() {
            return Err(RemoteDispatchError::Cancelled);
        }
        request
            .validate()
            .map_err(|_| RemoteDispatchError::InvalidRequest)?;
        validate_local_request(&request, self.policy.max_request_bytes)?;
        permission
            .validate()
            .map_err(|_| RemoteDispatchError::PermissionDenied)?;
        if permission.project_id != Some(request.context.project_id)
            || request.tool_name != permission.tool_name
            || request.tool_version != permission.tool_version
            || request.context.capability != permission.capability
            || request.context.policy_decision != permission.policy
        {
            return Err(RemoteDispatchError::PermissionDenied);
        }
        if !self.policy.allows(&request) {
            return Err(RemoteDispatchError::ToolNotAllowed);
        }
        if !matches!(
            self.permissions.evaluate(&permission),
            PermissionDecision::Allowed { .. }
        ) {
            return Err(RemoteDispatchError::PermissionDenied);
        }

        let lease = self.daemon.lease_context(lease_id, now_ms)?;
        if lease.node != target_node || lease.project != request.context.project_id {
            return Err(RemoteDispatchError::TargetMismatch);
        }
        let timeout_ms = self.policy.timeout_ms(&request)?;
        let deadline_ms = now_ms
            .checked_add(timeout_ms)
            .ok_or(RemoteDispatchError::TimeoutExceeded)?;
        let target = RemoteToolTarget::from(lease);
        let outbound = RemoteToolRequest {
            target,
            operation_key: request.operation_key,
            tool_name: request.tool_name.clone(),
            tool_version: request.tool_version.clone(),
            capability: request.context.capability.clone(),
            input: request.input.clone(),
            trace_id: request.context.trace_id,
            deadline_ms,
        };
        self.validate_request_payload(&outbound)?;
        let fingerprint = request_fingerprint(&outbound)?;

        let operation_key = request.operation_key;
        {
            let mut operations = self
                .operations
                .lock()
                .map_err(|_| RemoteDispatchError::StateUnavailable)?;
            purge_expired_terminal_operations(&mut operations, now_ms);
            if let Some(record) = operations.get(&operation_key) {
                if record.fingerprint != fingerprint {
                    return Err(RemoteDispatchError::OperationConflict);
                }
                return match &record.state {
                    OperationState::InFlight => Err(RemoteDispatchError::DuplicateOperation),
                    OperationState::Completed(response) => Ok(response.clone()),
                    OperationState::Cancelled => Err(RemoteDispatchError::Cancelled),
                    OperationState::Rejected => Err(RemoteDispatchError::TransportRejected),
                    OperationState::Unknown => Err(RemoteDispatchError::UnknownOutcome),
                };
            }
            if operations.len() >= MAX_REMOTE_OPERATIONS {
                return Err(RemoteDispatchError::Capacity);
            }
            operations.insert(
                operation_key,
                OperationRecord {
                    state: OperationState::InFlight,
                    fingerprint,
                    target: outbound.target.clone(),
                    updated_at_ms: now_ms,
                },
            );
        }

        let result = match tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            self.transport.execute(outbound, cancellation.clone()),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                self.mark_unknown(operation_key, now_ms)?;
                return Err(RemoteDispatchError::UnknownOutcome);
            }
        };
        if cancellation.is_cancelled() {
            self.mark_unknown(operation_key, now_ms)?;
            return Err(RemoteDispatchError::UnknownOutcome);
        }
        match result {
            Ok(response) => {
                if self.validate_response(&request, &response).is_err()
                    || !safe_terminal_outcome(response.outcome)
                {
                    self.mark_unknown(operation_key, now_ms)?;
                    return Err(RemoteDispatchError::UnknownOutcome);
                }
                let mut operations = self
                    .operations
                    .lock()
                    .map_err(|_| RemoteDispatchError::StateUnavailable)?;
                let Some(record) = operations.get_mut(&operation_key) else {
                    return Err(RemoteDispatchError::OperationNotFound);
                };
                match &record.state {
                    OperationState::InFlight => {
                        record.state = OperationState::Completed(response.clone());
                        record.updated_at_ms = now_ms;
                        Ok(response)
                    }
                    OperationState::Completed(cached) => Ok(cached.clone()),
                    OperationState::Cancelled | OperationState::Rejected => {
                        record.state = OperationState::Unknown;
                        record.updated_at_ms = now_ms;
                        Err(RemoteDispatchError::UnknownOutcome)
                    }
                    OperationState::Unknown => Err(RemoteDispatchError::UnknownOutcome),
                }
            }
            Err(RemoteTransportError::Rejected) => {
                self.set_state(operation_key, OperationState::Rejected, now_ms)?;
                Err(RemoteDispatchError::TransportRejected)
            }
            Err(
                RemoteTransportError::Timeout
                | RemoteTransportError::Cancelled
                | RemoteTransportError::Unavailable
                | RemoteTransportError::InvalidResponse,
            ) => {
                self.mark_unknown(operation_key, now_ms)?;
                Err(RemoteDispatchError::UnknownOutcome)
            }
        }
    }

    /// Requests cancellation of an in-flight operation. A successful adapter
    /// cancellation is terminal; an adapter failure becomes unknown.
    pub fn cancel(
        &self,
        lease_id: u64,
        now_ms: u64,
        operation_key: OperationKey,
    ) -> Result<(), RemoteDispatchError> {
        let lease = self.daemon.lease_context(lease_id, now_ms)?;
        let target = {
            let operations = self
                .operations
                .lock()
                .map_err(|_| RemoteDispatchError::StateUnavailable)?;
            let Some(record) = operations.get(&operation_key) else {
                return Err(RemoteDispatchError::OperationNotFound);
            };
            match record.state {
                OperationState::InFlight => record.target.clone(),
                OperationState::Cancelled => return Ok(()),
                OperationState::Completed(_) => return Err(RemoteDispatchError::AlreadyCompleted),
                OperationState::Rejected => return Err(RemoteDispatchError::TransportRejected),
                OperationState::Unknown => return Err(RemoteDispatchError::UnknownOutcome),
            }
        };
        if target.lease_id != lease.lease.id
            || target.node != lease.node
            || target.project != lease.project
        {
            return Err(RemoteDispatchError::TargetMismatch);
        }
        match self.transport.cancel(&target, operation_key) {
            Ok(()) => {
                self.set_state(operation_key, OperationState::Cancelled, now_ms)?;
                Ok(())
            }
            Err(_) => {
                self.mark_unknown(operation_key, now_ms)?;
                Err(RemoteDispatchError::UnknownOutcome)
            }
        }
    }

    pub fn status(&self, operation_key: OperationKey) -> Option<RemoteOperationStatus> {
        let operations = self.operations.lock().ok()?;
        operations
            .get(&operation_key)
            .map(|record| match record.state {
                OperationState::InFlight => RemoteOperationStatus::InFlight,
                OperationState::Completed(_) => RemoteOperationStatus::Completed,
                OperationState::Cancelled => RemoteOperationStatus::Cancelled,
                OperationState::Rejected => RemoteOperationStatus::Rejected,
                OperationState::Unknown => RemoteOperationStatus::Unknown,
            })
    }

    fn validate_request_payload(
        &self,
        request: &RemoteToolRequest,
    ) -> Result<(), RemoteDispatchError> {
        let encoded =
            serde_json::to_vec(request).map_err(|_| RemoteDispatchError::InvalidRequest)?;
        if encoded.len() > self.policy.max_request_bytes {
            return Err(RemoteDispatchError::PayloadTooLarge);
        }
        if contains_sensitive_material(&encoded) {
            return Err(RemoteDispatchError::SensitiveMaterial);
        }
        Ok(())
    }

    fn validate_response(
        &self,
        request: &ToolRequest,
        response: &ToolResponse,
    ) -> Result<(), RemoteDispatchError> {
        if response.operation_key != request.operation_key
            || response.tool_name != request.tool_name
            || response.tool_version != request.tool_version
            || response.trace_id != request.context.trace_id
        {
            return Err(RemoteDispatchError::InvalidRequest);
        }
        let encoded =
            serde_json::to_vec(response).map_err(|_| RemoteDispatchError::InvalidRequest)?;
        if encoded.len() > self.policy.max_response_bytes {
            return Err(RemoteDispatchError::PayloadTooLarge);
        }
        if contains_sensitive_material(&encoded) {
            return Err(RemoteDispatchError::SensitiveMaterial);
        }
        Ok(())
    }

    fn set_state(
        &self,
        operation_key: OperationKey,
        state: OperationState,
        updated_at_ms: u64,
    ) -> Result<(), RemoteDispatchError> {
        let mut operations = self
            .operations
            .lock()
            .map_err(|_| RemoteDispatchError::StateUnavailable)?;
        let Some(record) = operations.get_mut(&operation_key) else {
            return Err(RemoteDispatchError::OperationNotFound);
        };
        record.state = state;
        record.updated_at_ms = updated_at_ms;
        Ok(())
    }

    fn mark_unknown(
        &self,
        operation_key: OperationKey,
        updated_at_ms: u64,
    ) -> Result<(), RemoteDispatchError> {
        self.set_state(operation_key, OperationState::Unknown, updated_at_ms)
    }
}

fn validate_local_request(
    request: &ToolRequest,
    max_request_bytes: usize,
) -> Result<(), RemoteDispatchError> {
    let encoded = serde_json::to_vec(request).map_err(|_| RemoteDispatchError::InvalidRequest)?;
    if encoded.len() > max_request_bytes {
        return Err(RemoteDispatchError::PayloadTooLarge);
    }
    if contains_sensitive_material(&encoded) {
        return Err(RemoteDispatchError::SensitiveMaterial);
    }
    Ok(())
}

fn purge_expired_terminal_operations(
    operations: &mut HashMap<OperationKey, OperationRecord>,
    now_ms: u64,
) {
    operations.retain(|_, record| {
        matches!(&record.state, OperationState::InFlight)
            || now_ms.saturating_sub(record.updated_at_ms) <= REMOTE_OPERATION_RETENTION_MS
    });
}

#[derive(Serialize)]
struct RemoteToolFingerprint<'a> {
    target: &'a RemoteToolTarget,
    operation_key: OperationKey,
    tool_name: &'a str,
    tool_version: &'a str,
    capability: &'a str,
    input: &'a serde_json::Value,
    trace_id: TraceId,
}

fn request_fingerprint(request: &RemoteToolRequest) -> Result<[u8; 32], RemoteDispatchError> {
    let canonical = RemoteToolFingerprint {
        target: &request.target,
        operation_key: request.operation_key,
        tool_name: &request.tool_name,
        tool_version: &request.tool_version,
        capability: &request.capability,
        input: &request.input,
        trace_id: request.trace_id,
    };
    let encoded =
        serde_json::to_vec(&canonical).map_err(|_| RemoteDispatchError::InvalidRequest)?;
    let digest = Sha256::digest(encoded);
    Ok(digest.into())
}

fn safe_terminal_outcome(outcome: ToolOutcome) -> bool {
    matches!(
        outcome,
        ToolOutcome::Success
            | ToolOutcome::PermissionDenied
            | ToolOutcome::SchemaValidationError
            | ToolOutcome::BudgetExhausted
            | ToolOutcome::NotFound
            | ToolOutcome::CapabilityMismatch
    )
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_REMOTE_IDENTIFIER_BYTES
        && !value.chars().any(char::is_control)
}

fn contains_sensitive_material(payload: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) else {
        // An undecodable payload is not safe to forward or retain.
        return true;
    };
    contains_sensitive_value(&value)
}

fn contains_sensitive_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            false
        }
        serde_json::Value::String(text) => sensitive_text(text),
        serde_json::Value::Array(values) => values.iter().any(contains_sensitive_value),
        serde_json::Value::Object(object) => object
            .iter()
            .any(|(key, value)| sensitive_key(key) || contains_sensitive_value(value)),
    }
}

fn sensitive_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "credential" | "secret" | "password" | "api_key" | "apikey" | "authorization" | "token"
    )
}

fn sensitive_text(text: &str) -> bool {
    const MARKERS: &[&str] = &[
        "cred_",
        "credential:",
        "bearer ",
        "api_key=",
        "apikey=",
        "secret=",
        "password=",
        "token=",
    ];
    let lower = text.to_ascii_lowercase();
    MARKERS.iter().any(|marker| lower.contains(marker))
}
