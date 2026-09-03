//! Scoped, redacted, transport-neutral remote credential broker.
//!
//! This module issues opaque remote references for credentials that already
//! exist locally. It never stores, serializes or transmits secret material;
//! it only binds an opaque handle to an exact node/project/actor scope with a
//! bounded lease. Every handle also embeds a per-broker seed and a monotonic
//! generation counter, so two brokers that issue the same `(scope, ref)`
//! produce different handles and a previously purged or revoked handle can
//! never replay.
//!
//! Time and entropy come from a [`BrokerClock`] / [`BrokerEntropy`] trait
//! object that the caller injects. The default implementations are
//! [`SystemClock`] and [`OsEntropy`]; tests can pass deterministic stubs.
//! All scope inputs (node, project, actor) are bounded and revalidated at
//! every broker boundary so direct field construction cannot bypass the
//! invariants. Concrete OS keychain / Stronghold adapters and secret
//! migration belong to later cards.

use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::NodeId;
use provider_core::CredentialRef;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Maximum active scoped credential leases.
pub const MAX_CREDENTIAL_LEASES: usize = 256;
/// Maximum actor identifier length.
pub const MAX_ACTOR_LEN: usize = 64;
/// Maximum NodeId identifier length accepted at broker boundaries.
pub const MAX_NODE_ID_LEN: usize = 128;
/// Maximum retained redacted audit events.
pub const MAX_CREDENTIAL_AUDIT_EVENTS: usize = 256;
/// Maximum retained revoked tombstones (capacity observability only).
pub const MAX_REVOKED_TOMBSTONES: usize = 256;
/// Maximum allowed lease duration in milliseconds (24h). Anything beyond
/// this would let a single lease occupy a slot indefinitely and bypass
/// the bounded-expiry/rotation guarantee.
pub const MAX_LEASE_DURATION_MS: u64 = 24 * 60 * 60 * 1_000;
/// Maximum allowed length of a credential reference label held by the
/// broker. Mirrors the provider-core invariant to keep defence in depth
/// even when the caller hands the broker a value that was not produced
/// by `CredentialRef::parse` (e.g. via direct construction or deserialise).
pub const MAX_CREDENTIAL_REF_LEN: usize = 128;

/// Source of monotonic time for the broker. Production code uses
/// [`SystemClock`]; tests inject a deterministic clock.
pub trait BrokerClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Source of per-broker entropy used to seed handle generation so a fresh
/// broker never produces the same handle for the same `(scope, ref)` that a
/// previous broker instance did.
pub trait BrokerEntropy: Send + Sync {
    fn next_seed(&self) -> [u8; 16];
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl BrokerClock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OsEntropy;

impl BrokerEntropy for OsEntropy {
    fn next_seed(&self) -> [u8; 16] {
        // Mix the system clock with a process-local atomic counter so two
        // calls in the same millisecond still differ.
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let now = SystemClock.now_ms();
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut seed = [0u8; 16];
        seed[0..8].copy_from_slice(&now.to_le_bytes());
        seed[8..16].copy_from_slice(&count.to_le_bytes());
        seed
    }
}

/// Exact scope a credential reference is bound to. All fields are private;
/// callers must construct the scope through [`CredentialScope::new`] so the
/// invariants are revalidated at every broker boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialScope {
    node: NodeId,
    project: ProjectId,
    actor: String,
}

impl CredentialScope {
    pub fn new(
        node: NodeId,
        project: ProjectId,
        actor: &str,
    ) -> Result<Self, CredentialBrokerError> {
        Self::validate_node(&node)?;
        Self::validate_actor(actor)?;
        Ok(Self {
            node,
            project,
            actor: actor.into(),
        })
    }

    /// Re-validates a scope at every broker boundary. The `node` field
    /// already lives inside a `NodeId`, but a caller could deserialize or
    /// directly build a `NodeId` whose inner string is empty, oversized or
    /// control-character-filled. This check keeps the fail-closed
    /// invariant regardless of how the value was produced.
    pub fn revalidate(&self) -> Result<(), CredentialBrokerError> {
        Self::validate_node(&self.node)?;
        Self::validate_actor(&self.actor)
    }

    fn validate_actor(actor: &str) -> Result<(), CredentialBrokerError> {
        if actor.trim().is_empty()
            || actor.len() > MAX_ACTOR_LEN
            || actor.chars().any(char::is_control)
        {
            return Err(CredentialBrokerError::InvalidScope);
        }
        Ok(())
    }

    fn validate_node(node: &NodeId) -> Result<(), CredentialBrokerError> {
        if node.0.is_empty()
            || node.0.len() > MAX_NODE_ID_LEN
            || node.0.chars().any(char::is_control)
        {
            return Err(CredentialBrokerError::InvalidScope);
        }
        Ok(())
    }

    pub fn node(&self) -> &NodeId {
        &self.node
    }

    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    pub fn actor(&self) -> &str {
        &self.actor
    }
}

/// Opaque handle for a scoped credential reference. Never carries secret
/// material; it is a SHA-256 digest of
/// `(broker_seed, scope, reference, generation)`. The seed is per-broker
/// random, the generation is per-broker monotonic, so two brokers that
/// issue the same `(scope, ref)` produce different handles and a
/// previously purged handle can never replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopedCredentialRef([u8; 32]);

impl ScopedCredentialRef {
    fn from_parts(
        seed: &[u8; 16],
        scope: &CredentialScope,
        reference: &CredentialRef,
        generation: u64,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(b"\0");
        hasher.update(scope.node.0.as_bytes());
        hasher.update(b"\0");
        hasher.update(scope.project.to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(scope.actor.as_bytes());
        hasher.update(b"\0");
        hasher.update(reference.as_str().as_bytes());
        hasher.update(b"\0");
        hasher.update(generation.to_le_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        Self(digest)
    }

    pub fn as_hex(&self) -> String {
        let mut out = String::with_capacity(70);
        out.push_str("scoped_");
        for byte in self.0 {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }

    /// Validated input conversion. Accepts only the canonical
    /// `scoped_<64-hex>` form produced by [`ScopedCredentialRef::as_hex`].
    /// Transport adapters that receive a handle from a peer must use
    /// this constructor to rebuild a handle before calling
    /// [`CredentialBroker::resolve`] or [`CredentialBroker::revoke`].
    pub fn parse(value: &str) -> Result<Self, CredentialBrokerError> {
        let prefix = "scoped_";
        let expected = prefix.len() + 64;
        if value.len() != expected || !value.starts_with(prefix) {
            return Err(CredentialBrokerError::InvalidScope);
        }
        let hex = &value[prefix.len()..];
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(CredentialBrokerError::InvalidScope);
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let hi = (chunk[0] as char)
                .to_digit(16)
                .ok_or(CredentialBrokerError::InvalidScope)?;
            let lo = (chunk[1] as char)
                .to_digit(16)
                .ok_or(CredentialBrokerError::InvalidScope)?;
            bytes[i] = ((hi << 4) | lo) as u8;
        }
        Ok(Self(bytes))
    }
}

/// Bounded lease for a scoped credential reference. The handle is the
/// only value that needs to be transported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialLease {
    pub handle: ScopedCredentialRef,
    pub scope: CredentialScope,
    pub expires_at_ms: u64,
}

/// Redacted audit reasons for broker operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialAuditReason {
    Issued,
    Resolved,
    ScopeDenied,
    Expired,
    Revoked,
    CapacityDenied,
    NotFound,
}

/// Redacted audit event. Contains only the scope and reason; never a
/// credential reference, secret or token value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialAuditEvent {
    pub scope: CredentialScope,
    pub reason: CredentialAuditReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CredentialBrokerError {
    #[error("remote credential scope is invalid")]
    InvalidScope,
    #[error("remote credential lease is not found")]
    NotFound,
    #[error("remote credential scope mismatch")]
    ScopeMismatch,
    #[error("remote credential lease is expired")]
    Expired,
    #[error("remote credential lease is revoked")]
    Revoked,
    #[error("remote credential broker capacity exhausted")]
    CapacityExhausted,
    #[error("remote credential broker state lock unavailable")]
    StateUnavailable,
}

struct LeaseRecord {
    handle: ScopedCredentialRef,
    scope: CredentialScope,
    reference: CredentialRef,
    expires_at_ms: u64,
}

struct BrokerState {
    seed: [u8; 16],
    next_generation: u64,
    leases: BTreeMap<ScopedCredentialRef, LeaseRecord>,
    revoked_tombstones: VecDeque<ScopedCredentialRef>,
    audit: VecDeque<CredentialAuditEvent>,
}

/// Bounded, redacted remote credential broker. Secret material never
/// enters. Time is taken from the injected `clock`; entropy from the
/// injected `entropy` is consumed once at construction to seed the
/// handle-generation space.
pub struct CredentialBroker {
    clock: Arc<dyn BrokerClock>,
    #[allow(dead_code)]
    entropy: Arc<dyn BrokerEntropy>,
    state: Mutex<BrokerState>,
}

impl CredentialBroker {
    pub fn new() -> Self {
        Self::with_clock_and_entropy(Arc::new(SystemClock), Arc::new(OsEntropy))
    }

    pub fn with_clock(clock: Arc<dyn BrokerClock>) -> Self {
        Self::with_clock_and_entropy(clock, Arc::new(OsEntropy))
    }

    pub fn with_clock_and_entropy(
        clock: Arc<dyn BrokerClock>,
        entropy: Arc<dyn BrokerEntropy>,
    ) -> Self {
        let seed = entropy.next_seed();
        Self {
            clock,
            entropy,
            state: Mutex::new(BrokerState {
                seed,
                next_generation: 1,
                leases: BTreeMap::new(),
                revoked_tombstones: VecDeque::with_capacity(MAX_REVOKED_TOMBSTONES),
                audit: VecDeque::with_capacity(MAX_CREDENTIAL_AUDIT_EVENTS),
            }),
        }
    }

    /// Issues an opaque scoped reference for an existing local credential.
    /// Time is taken from the injected clock; the caller cannot choose the
    /// start instant. The handle embeds the per-broker seed and a
    /// monotonic generation, so re-issued leases for the same `(scope, ref)`
    /// never alias a previous one.
    pub fn issue(
        &self,
        scope: CredentialScope,
        reference: CredentialRef,
        lease_duration_ms: u64,
    ) -> Result<CredentialLease, CredentialBrokerError> {
        scope.revalidate()?;
        Self::validate_credential_ref(&reference)?;
        if lease_duration_ms == 0 {
            return Err(CredentialBrokerError::InvalidScope);
        }
        if lease_duration_ms > MAX_LEASE_DURATION_MS {
            return Err(CredentialBrokerError::InvalidScope);
        }
        let now_ms = self.clock.now_ms();
        let expires_at_ms = now_ms
            .checked_add(lease_duration_ms)
            .ok_or(CredentialBrokerError::InvalidScope)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| CredentialBrokerError::StateUnavailable)?;
        self.purge_expired(&mut state, now_ms);
        // Count only live, non-revoked leases against capacity; revoked
        // tombstones are kept in a separate bounded ring and do not block
        // new issuance.
        if state.leases.len() >= MAX_CREDENTIAL_LEASES {
            self.push_audit(
                &mut state,
                scope.clone(),
                CredentialAuditReason::CapacityDenied,
            );
            return Err(CredentialBrokerError::CapacityExhausted);
        }
        let generation = state.next_generation;
        state.next_generation = state
            .next_generation
            .checked_add(1)
            .ok_or(CredentialBrokerError::StateUnavailable)?;
        let handle = ScopedCredentialRef::from_parts(&state.seed, &scope, &reference, generation);
        state.leases.insert(
            handle,
            LeaseRecord {
                handle,
                scope: scope.clone(),
                reference,
                expires_at_ms,
            },
        );
        self.push_audit(&mut state, scope.clone(), CredentialAuditReason::Issued);
        Ok(CredentialLease {
            handle,
            scope,
            expires_at_ms,
        })
    }

    /// Resolves a broker-issued lease to the local credential reference
    /// only when the lease is still active. The `lease` is the
    /// broker-issued access context: the caller must present the
    /// original `CredentialLease` returned by `issue`; the broker will
    /// not accept a caller-supplied scope or handle on its own. Time is
    /// taken from the injected clock; the caller cannot bypass expiry
    /// by backdating the timestamp.
    pub fn resolve(&self, lease: &CredentialLease) -> Result<CredentialRef, CredentialBrokerError> {
        lease.scope.revalidate()?;
        Self::validate_credential_ref_handle(&lease.handle)?;
        let now_ms = self.clock.now_ms();
        let mut state = self
            .state
            .lock()
            .map_err(|_| CredentialBrokerError::StateUnavailable)?;
        // Evaluate the requested handle against the current state without
        // mutating, so we can emit a precise audit reason and return the
        // proper error variant (Expired vs NotFound vs ScopeMismatch).
        let mut target: Option<Result<CredentialRef, CredentialBrokerError>> = None;
        for record in state.leases.values() {
            if record.handle != lease.handle {
                continue;
            }
            if record.scope != lease.scope {
                target = Some(Err(CredentialBrokerError::ScopeMismatch));
                break;
            }
            if now_ms >= record.expires_at_ms {
                target = Some(Err(CredentialBrokerError::Expired));
                break;
            }
            target = Some(Ok(record.reference.clone()));
            break;
        }
        // Now purge expired entries; revoked tombstones are kept separately
        // for bounded observability.
        self.purge_expired(&mut state, now_ms);
        match target {
            Some(Ok(reference)) => {
                self.push_audit(
                    &mut state,
                    lease.scope.clone(),
                    CredentialAuditReason::Resolved,
                );
                Ok(reference)
            }
            Some(Err(err)) => {
                let reason = match err {
                    CredentialBrokerError::Revoked => CredentialAuditReason::Revoked,
                    CredentialBrokerError::Expired => CredentialAuditReason::Expired,
                    CredentialBrokerError::ScopeMismatch => CredentialAuditReason::ScopeDenied,
                    _ => CredentialAuditReason::ScopeDenied,
                };
                self.push_audit(&mut state, lease.scope.clone(), reason);
                Err(err)
            }
            None => {
                // Unknown handle: still recorded with the caller's scope so
                // the audit trail captures probing attempts.
                self.push_audit(
                    &mut state,
                    lease.scope.clone(),
                    CredentialAuditReason::NotFound,
                );
                Err(CredentialBrokerError::NotFound)
            }
        }
    }

    /// Revokes a lease by presenting the broker-issued lease token. The
    /// caller must present the exact `CredentialLease` returned by
    /// `issue`; the broker will not accept a caller-supplied scope or
    /// handle on its own. The handle is the only revocation token and
    /// the scope is bound to the issuer, so a malicious caller cannot
    /// enumerate or forge revocations for other agents' leases.
    pub fn revoke(&self, lease: &CredentialLease) -> Result<(), CredentialBrokerError> {
        lease.scope.revalidate()?;
        Self::validate_credential_ref_handle(&lease.handle)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| CredentialBrokerError::StateUnavailable)?;
        let removed = state.leases.remove(&lease.handle);
        match removed {
            Some(record) => {
                if record.scope != lease.scope {
                    // Return the lease so capacity is not silently freed by
                    // a forged scope, and audit the attempt.
                    state.leases.insert(record.handle, record);
                    self.push_audit(
                        &mut state,
                        lease.scope.clone(),
                        CredentialAuditReason::ScopeDenied,
                    );
                    return Err(CredentialBrokerError::ScopeMismatch);
                }
                if state.revoked_tombstones.len() == MAX_REVOKED_TOMBSTONES {
                    state.revoked_tombstones.pop_front();
                }
                state.revoked_tombstones.push_back(record.handle);
                self.push_audit(
                    &mut state,
                    lease.scope.clone(),
                    CredentialAuditReason::Revoked,
                );
                Ok(())
            }
            None => {
                self.push_audit(
                    &mut state,
                    lease.scope.clone(),
                    CredentialAuditReason::NotFound,
                );
                Err(CredentialBrokerError::NotFound)
            }
        }
    }

    /// Defensive check on a `ScopedCredentialRef` handle at the broker
    /// boundary. The handle is opaque, so this only enforces a sanity
    /// bound on its serialized length (it is a 32-byte digest and its
    /// hex form has a fixed length).
    fn validate_credential_ref_handle(
        handle: &ScopedCredentialRef,
    ) -> Result<(), CredentialBrokerError> {
        let hex = handle.as_hex();
        if hex.len() != "scoped_".len() + 64 {
            return Err(CredentialBrokerError::InvalidScope);
        }
        Ok(())
    }

    /// Returns bounded, redacted audit events in oldest-to-newest order.
    pub fn audit(&self) -> Vec<CredentialAuditEvent> {
        self.state
            .lock()
            .map(|state| state.audit.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Number of active leases (observability).
    pub fn active_leases(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.leases.len())
            .unwrap_or_default()
    }

    /// Number of currently retained revoked tombstones (observability).
    pub fn revoked_tombstones(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.revoked_tombstones.len())
            .unwrap_or_default()
    }

    fn purge_expired(&self, state: &mut BrokerState, now_ms: u64) {
        state
            .leases
            .retain(|_, record| now_ms < record.expires_at_ms);
    }

    /// Defensive check on the inner string of a `CredentialRef`. The
    /// provider-core parser already enforces this when the value is
    /// built via `CredentialRef::parse`, but a caller that hands the
    /// broker a value produced by direct construction or by a custom
    /// `Deserialize` impl could otherwise bypass that invariant. The
    /// broker itself never deserialises — this check is the safety net
    /// at the broker boundary.
    fn validate_credential_ref(reference: &CredentialRef) -> Result<(), CredentialBrokerError> {
        let value = reference.as_str();
        if value.is_empty() || value.len() > MAX_CREDENTIAL_REF_LEN {
            return Err(CredentialBrokerError::InvalidScope);
        }
        let normalized = value.to_ascii_lowercase();
        for marker in [
            "api_key", "apikey", "api-key", "secret", "token", "password", "bearer",
        ] {
            if normalized.contains(marker) {
                return Err(CredentialBrokerError::InvalidScope);
            }
        }
        Ok(())
    }

    fn push_audit(
        &self,
        state: &mut BrokerState,
        scope: CredentialScope,
        reason: CredentialAuditReason,
    ) {
        if state.audit.len() == MAX_CREDENTIAL_AUDIT_EVENTS {
            state.audit.pop_front();
        }
        state
            .audit
            .push_back(CredentialAuditEvent { scope, reason });
    }
}

impl Default for CredentialBroker {
    fn default() -> Self {
        Self::new()
    }
}
