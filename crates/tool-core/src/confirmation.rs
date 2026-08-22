//! Bounded, fail-closed approval artifacts for sensitive tool effects.

use crate::permission::ToolEffect;
use agent_core::budget::ReservationId;
use agent_core::ids::{AgentId, ProjectId};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use uuid::Uuid;

pub const MAX_APPROVALS: usize = 1024;
pub const MAX_APPROVAL_TEXT_BYTES: usize = 128;
pub const MAX_APPROVAL_PAYLOAD_BYTES: usize = 64 * 1024;
pub const MAX_APPROVAL_LIFETIME_MS: u64 = 24 * 60 * 60 * 1000;

/// Confirmation mode selected by the permission boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationPolicy {
    AlwaysAllow,
    AskOnce,
    AskEveryTime,
    Deny,
}

/// A pending approval contains only hashes of schema and arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: Uuid,
    pub project_id: ProjectId,
    pub agent_id: Option<AgentId>,
    pub tool_name: String,
    pub tool_version: String,
    pub schema_hash: String,
    pub args_hash: String,
    pub effect: ToolEffect,
    pub budget_ref: Option<ReservationId>,
    pub trace_id: TraceId,
    pub actor_id: String,
    pub policy: ConfirmationPolicy,
    pub created_at_ms: u64,
    pub expires_at_ms: u64,
}

impl ApprovalRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        agent_id: Option<AgentId>,
        tool_name: impl Into<String>,
        tool_version: impl Into<String>,
        schema: &Value,
        args: &Value,
        effect: ToolEffect,
        budget_ref: Option<ReservationId>,
        trace_id: TraceId,
        actor_id: impl Into<String>,
        policy: ConfirmationPolicy,
        created_at_ms: u64,
        expires_at_ms: u64,
    ) -> Result<Self, ConfirmationError> {
        let tool_name = tool_name.into();
        let tool_version = tool_version.into();
        let actor_id = actor_id.into();
        validate_text(&tool_name, "tool_name")?;
        validate_text(&tool_version, "tool_version")?;
        validate_text(&actor_id, "actor_id")?;
        if expires_at_ms <= created_at_ms
            || expires_at_ms - created_at_ms > MAX_APPROVAL_LIFETIME_MS
        {
            return Err(ConfirmationError::InvalidRequest);
        }

        Ok(Self {
            request_id: Uuid::new_v4(),
            project_id,
            agent_id,
            tool_name,
            tool_version,
            schema_hash: Self::hash_payload(schema)?,
            args_hash: Self::hash_payload(args)?,
            effect,
            budget_ref,
            trace_id,
            actor_id,
            policy,
            created_at_ms,
            expires_at_ms,
        })
    }

    /// Hashes canonical JSON and never retains the supplied payload.
    pub fn hash_payload(payload: &Value) -> Result<String, ConfirmationError> {
        let canonical = canonical_json(payload);
        let bytes =
            serde_json::to_vec(&canonical).map_err(|_| ConfirmationError::InvalidPayload)?;
        if bytes.len() > MAX_APPROVAL_PAYLOAD_BYTES {
            return Err(ConfirmationError::InvalidPayload);
        }
        let digest = Sha256::digest(bytes);
        Ok(format!("{digest:x}"))
    }

    fn request_fingerprint(&self) -> Result<String, ConfirmationError> {
        fingerprint(&RequestBinding::from(self, true))
    }

    fn scope_fingerprint(&self) -> Result<String, ConfirmationError> {
        fingerprint(&RequestBinding::from(self, false))
    }
}

/// Short-lived approval artifact returned to the application/UI boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalGrant {
    pub grant_id: Uuid,
    pub request_id: Uuid,
    pub actor_id: String,
    request_fingerprint: String,
    scope_fingerprint: String,
    expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfirmationError {
    #[error("approval request is invalid")]
    InvalidRequest,
    #[error("approval payload is invalid or exceeds the bound")]
    InvalidPayload,
    #[error("approval actor is invalid")]
    InvalidActor,
    #[error("policy denies the effect")]
    PolicyDenied,
    #[error("confirmation is not required for this policy")]
    ApprovalNotRequired,
    #[error("approval request was not found")]
    NotFound,
    #[error("approval has expired")]
    Expired,
    #[error("approval was revoked")]
    Revoked,
    #[error("approval grant was already consumed")]
    Replay,
    #[error("approval does not match the presented request")]
    RequestMismatch,
    #[error("approval actor does not match")]
    ActorMismatch,
    #[error("approval ledger is full")]
    CapacityFull,
    #[error("approval ledger lock is unavailable")]
    LedgerUnavailable,
}

#[derive(Debug)]
struct Entry {
    request_fingerprint: String,
    scope_fingerprint: String,
    policy: ConfirmationPolicy,
    expires_at_ms: u64,
    grant: Option<ApprovalGrant>,
    revoked: bool,
    consumed: bool,
}

#[derive(Debug, Default)]
struct LedgerState {
    entries: BTreeMap<Uuid, Entry>,
    ask_once_scopes: BTreeMap<String, Uuid>,
    revoked: BTreeSet<Uuid>,
}

/// Thread-safe in-memory approval ledger with bounded retention.
#[derive(Debug, Default)]
pub struct ConfirmationLedger {
    state: Mutex<LedgerState>,
}

impl ConfirmationLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, request: ApprovalRequest) -> Result<(), ConfirmationError> {
        let request_fingerprint = request.request_fingerprint()?;
        let scope_fingerprint = request.scope_fingerprint()?;
        if request.policy == ConfirmationPolicy::Deny {
            return Err(ConfirmationError::PolicyDenied);
        }
        if request.policy == ConfirmationPolicy::AlwaysAllow {
            return Err(ConfirmationError::ApprovalNotRequired);
        }
        let mut state = self.lock()?;
        if state.entries.len() >= MAX_APPROVALS || state.entries.contains_key(&request.request_id) {
            return Err(ConfirmationError::CapacityFull);
        }
        state.entries.insert(
            request.request_id,
            Entry {
                request_fingerprint,
                scope_fingerprint,
                policy: request.policy,
                expires_at_ms: request.expires_at_ms,
                grant: None,
                revoked: false,
                consumed: false,
            },
        );
        Ok(())
    }

    pub fn approve(
        &self,
        request_id: Uuid,
        actor_id: &str,
        now_ms: u64,
    ) -> Result<ApprovalGrant, ConfirmationError> {
        validate_text(actor_id, "actor_id")?;
        let mut state = self.lock()?;
        let (
            expires_at_ms,
            revoked,
            existing_grant,
            request_fingerprint,
            scope_fingerprint,
            policy,
        ) = {
            let entry = state
                .entries
                .get(&request_id)
                .ok_or(ConfirmationError::NotFound)?;
            (
                entry.expires_at_ms,
                entry.revoked,
                entry.grant.clone(),
                entry.request_fingerprint.clone(),
                entry.scope_fingerprint.clone(),
                entry.policy,
            )
        };
        if now_ms >= expires_at_ms {
            return Err(ConfirmationError::Expired);
        }
        if revoked {
            return Err(ConfirmationError::Revoked);
        }
        if let Some(grant) = existing_grant {
            if grant.actor_id == actor_id {
                return Ok(grant);
            }
            return Err(ConfirmationError::ActorMismatch);
        }
        let grant = ApprovalGrant {
            grant_id: Uuid::new_v4(),
            request_id,
            actor_id: actor_id.to_string(),
            request_fingerprint,
            scope_fingerprint: scope_fingerprint.clone(),
            expires_at_ms,
        };
        if policy == ConfirmationPolicy::AskOnce {
            state.ask_once_scopes.insert(scope_fingerprint, request_id);
        }
        state
            .entries
            .get_mut(&request_id)
            .ok_or(ConfirmationError::NotFound)?
            .grant = Some(grant.clone());
        Ok(grant)
    }

    pub fn authorize(
        &self,
        request: &ApprovalRequest,
        grant: &ApprovalGrant,
        actor_id: &str,
        now_ms: u64,
    ) -> Result<(), ConfirmationError> {
        validate_text(actor_id, "actor_id")?;
        if request.policy == ConfirmationPolicy::Deny {
            return Err(ConfirmationError::PolicyDenied);
        }
        if request.policy == ConfirmationPolicy::AlwaysAllow {
            return Err(ConfirmationError::ApprovalNotRequired);
        }
        if now_ms >= request.expires_at_ms {
            return Err(ConfirmationError::Expired);
        }
        let request_fingerprint = request.request_fingerprint()?;
        let scope_fingerprint = request.scope_fingerprint()?;
        let mut state = self.lock()?;
        let entry_id = if state.entries.contains_key(&request.request_id) {
            request.request_id
        } else if request.policy == ConfirmationPolicy::AskOnce {
            *state
                .ask_once_scopes
                .get(&scope_fingerprint)
                .ok_or(ConfirmationError::NotFound)?
        } else {
            return Err(ConfirmationError::NotFound);
        };
        if state.revoked.contains(&entry_id) {
            return Err(ConfirmationError::Revoked);
        }
        let entry = state
            .entries
            .get_mut(&entry_id)
            .ok_or(ConfirmationError::NotFound)?;
        if entry.revoked {
            return Err(ConfirmationError::Revoked);
        }
        if now_ms >= entry.expires_at_ms || now_ms >= grant.expires_at_ms {
            return Err(ConfirmationError::Expired);
        }
        if grant.actor_id != actor_id {
            return Err(ConfirmationError::ActorMismatch);
        }
        if grant.request_id != entry_id
            || grant.grant_id
                != entry
                    .grant
                    .as_ref()
                    .map(|value| value.grant_id)
                    .unwrap_or_default()
        {
            return Err(ConfirmationError::RequestMismatch);
        }
        if request.policy != entry.policy || grant.scope_fingerprint != scope_fingerprint {
            return Err(ConfirmationError::RequestMismatch);
        }
        if request.policy == ConfirmationPolicy::AskEveryTime {
            if entry.consumed || grant.request_fingerprint != request_fingerprint {
                return if entry.consumed {
                    Err(ConfirmationError::Replay)
                } else {
                    Err(ConfirmationError::RequestMismatch)
                };
            }
            entry.consumed = true;
        } else if grant.scope_fingerprint != scope_fingerprint {
            return Err(ConfirmationError::RequestMismatch);
        }
        Ok(())
    }

    pub fn revoke(&self, request: &ApprovalRequest) -> Result<(), ConfirmationError> {
        let scope_fingerprint = request.scope_fingerprint()?;
        let mut state = self.lock()?;
        let entry_id = state
            .entries
            .get(&request.request_id)
            .map(|_| request.request_id)
            .or_else(|| state.ask_once_scopes.get(&scope_fingerprint).copied())
            .ok_or(ConfirmationError::NotFound)?;
        if let Some(entry) = state.entries.get_mut(&entry_id) {
            entry.revoked = true;
        }
        state.revoked.insert(entry_id);
        state.ask_once_scopes.remove(&scope_fingerprint);
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, LedgerState>, ConfirmationError> {
        self.state
            .lock()
            .map_err(|_| ConfirmationError::LedgerUnavailable)
    }
}

#[derive(Debug, Serialize)]
struct RequestBinding<'a> {
    project_id: ProjectId,
    agent_id: Option<AgentId>,
    tool_name: &'a str,
    tool_version: &'a str,
    schema_hash: &'a str,
    args_hash: &'a str,
    effect: ToolEffect,
    budget_ref: Option<ReservationId>,
    trace_id: TraceId,
    actor_id: &'a str,
    policy: ConfirmationPolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<Uuid>,
}

impl<'a> RequestBinding<'a> {
    fn from(request: &'a ApprovalRequest, include_request_id: bool) -> Self {
        Self {
            project_id: request.project_id,
            agent_id: request.agent_id,
            tool_name: &request.tool_name,
            tool_version: &request.tool_version,
            schema_hash: &request.schema_hash,
            args_hash: &request.args_hash,
            effect: request.effect,
            budget_ref: request.budget_ref,
            trace_id: request.trace_id,
            actor_id: &request.actor_id,
            policy: request.policy,
            request_id: include_request_id.then_some(request.request_id),
        }
    }
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, ConfirmationError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ConfirmationError::InvalidRequest)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{digest:x}"))
}

fn validate_text(value: &str, _field: &str) -> Result<(), ConfirmationError> {
    if value.is_empty()
        || value.len() > MAX_APPROVAL_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(if value.is_empty() && _field == "actor_id" {
            ConfirmationError::InvalidActor
        } else {
            ConfirmationError::InvalidRequest
        });
    }
    Ok(())
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sorted = Map::new();
            let mut entries: Vec<_> = object.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            for (key, value) in entries {
                sorted.insert(key.clone(), canonical_json(value));
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}
