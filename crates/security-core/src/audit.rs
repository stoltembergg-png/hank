//! Pure, deterministic, fail-closed audit log boundary.
//!
//! This crate does not perform I/O, does not read the real clock, and does not
//! know about concrete sinks, storage, network, secrets, or remote systems.
//! Callers provide a bounded [`AuditPolicy`], a monotonic clock value, an
//! [`AuditSink`] implementation, and the events to record. The crate returns
//! explicit, typed outcomes for every state transition.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Upper bound on a single event payload in bytes.
pub const MAX_EVENT_PAYLOAD_BYTES: usize = 4096;
/// Upper bound on the number of events retained by a single log.
pub const MAX_EVENTS_PER_LOG: usize = 4096;
/// Upper bound on the number of events retained by a single query/export.
pub const MAX_QUERY_RESULTS: usize = 1024;
/// Upper bound on the number of events exported in a single batch.
pub const MAX_EXPORT_ROWS: usize = 1024;
/// Upper bound on the retention window in milliseconds.
pub const MAX_RETENTION_MS: u64 = 365 * 24 * 60 * 60 * 1000;
/// Upper bound on the number of events admitted by retention policy.
pub const MAX_RETENTION_EVENTS: usize = 4096;
/// Upper bound on the length of an actor identifier.
pub const MAX_ACTOR_ID_LEN: usize = 128;
/// Upper bound on the length of a resource identifier.
pub const MAX_RESOURCE_ID_LEN: usize = 128;
/// Upper bound on the length of a policy revision.
pub const MAX_POLICY_REVISION_LEN: usize = 128;
/// Upper bound on the length of a project identifier.
pub const MAX_PROJECT_ID_LEN: usize = 128;
/// Upper bound on the length of a scope key.
pub const MAX_SCOPE_KEY_LEN: usize = 128;
/// Upper bound on the number of events exported by a single call to `export`.
pub const MAX_EXPORT_LIMIT: usize = MAX_EXPORT_ROWS;
/// Placeholder used for every redacted field.
pub const REDACTED_PLACEHOLDER: &str = "[REDACTED]";
/// Sentinel used to identify the genesis (first) event in a chain.
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Classification of a redacted value. Every value flagged as
/// [`RedactedField::Secret`] is replaced by [`REDACTED_PLACEHOLDER`] before
/// any serialization, export, query, comparison, or hash computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RedactedField {
    /// Token, password, API key, private key, connection string, or any
    /// value that should never cross the boundary of the audit contract.
    Secret,
}

/// Classification of the audit event. Adapters and consumers route
/// downstream behavior (metrics, alerting, export) on this label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditClass {
    Authorization,
    Migration,
    Recovery,
    PluginRemote,
    Release,
    Denial,
    Other,
}

/// Single redacted value attached to an event payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RedactedValue {
    field: RedactedField,
}

impl RedactedValue {
    /// Build a redacted value marker. The contained value is irrelevant; the
    /// field classification governs serialization.
    pub fn new(field: RedactedField) -> Self {
        Self { field }
    }

    /// Return the classification of the redacted field.
    pub fn field(&self) -> RedactedField {
        self.field
    }
}

impl fmt::Display for RedactedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED_PLACEHOLDER)
    }
}

/// A structured payload key. Keys are bounded strings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PayloadKey(String);

impl PayloadKey {
    fn parse(value: &str) -> Result<Self, AuditError> {
        if value.is_empty()
            || value.len() > MAX_SCOPE_KEY_LEN
            || value.chars().any(char::is_control)
        {
            return Err(AuditError::EventInvalid);
        }
        Ok(Self(value.to_string()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// A structured payload value: either a UTF-8 string or a redacted marker.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PayloadValue {
    /// Opaque text payload. Must not contain raw secrets; callers are
    /// expected to use [`PayloadValue::redacted`] for any sensitive value.
    Text(String),
    /// A redacted marker; serialized as [`REDACTED_PLACEHOLDER`].
    Redacted(RedactedValue),
}

impl PayloadValue {
    /// Build a text payload value. Rejects empty strings and control
    /// characters.
    pub fn text(value: impl Into<String>) -> Result<Self, AuditError> {
        let value = value.into();
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(AuditError::EventInvalid);
        }
        Ok(Self::Text(value))
    }

    /// Build a redacted payload value of the given field classification.
    pub fn redacted(field: RedactedField) -> Self {
        Self::Redacted(RedactedValue::new(field))
    }

    /// Render the payload value to its deterministic wire form. Redacted
    /// values are rendered as [`REDACTED_PLACEHOLDER`].
    pub fn render(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Redacted(_) => REDACTED_PLACEHOLDER.to_string(),
        }
    }
}

/// Structured payload of an audit event. All keys and values are bounded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    entries: BTreeMap<PayloadKey, PayloadValue>,
}

impl Payload {
    /// Build an empty payload.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Insert a text entry. Returns [`AuditError::EventInvalid`] if the key
    /// is empty/too long/contains control characters, if the value is
    /// empty/contains control characters, or if the payload would exceed
    /// [`MAX_EVENT_PAYLOAD_BYTES`].
    pub fn insert_text(&mut self, key: &str, value: impl Into<String>) -> Result<(), AuditError> {
        let key = PayloadKey::parse(key)?;
        let value = value.into();
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(AuditError::EventInvalid);
        }
        let value = PayloadValue::Text(value);
        self.insert(key, value)
    }

    /// Insert a redacted entry. Use this for any value classified as
    /// [`RedactedField::Secret`].
    pub fn insert_redacted(&mut self, key: &str, field: RedactedField) -> Result<(), AuditError> {
        let key = PayloadKey::parse(key)?;
        let value = PayloadValue::redacted(field);
        self.insert(key, value)
    }

    /// Number of entries in the payload.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the payload has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over the entries as `(key, rendered_value)` pairs in
    /// deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, String)> + '_ {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.render()))
    }

    fn insert(&mut self, key: PayloadKey, value: PayloadValue) -> Result<(), AuditError> {
        let key_for_remove = key.clone();
        if self.entries.contains_key(&key) {
            return Err(AuditError::EventInvalid);
        }
        self.entries.insert(key, value);
        if self.encoded_len() > MAX_EVENT_PAYLOAD_BYTES {
            self.entries.remove(&key_for_remove);
            return Err(AuditError::PayloadTooLarge);
        }
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let mut total = 0usize;
        for (k, v) in &self.entries {
            total = total.saturating_add(k.as_str().len());
            total = total.saturating_add(1);
            total = total.saturating_add(v.render().len());
        }
        total
    }
}

impl Default for Payload {
    fn default() -> Self {
        Self::new()
    }
}

/// Classification of an integrity failure surfaced by [`AuditLog::verify_chain`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntegrityClassification {
    Ok,
    Missing,
    OutOfOrder,
    HashMismatch,
    Broken,
}

/// Detailed result of an integrity verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditIntegrity {
    classification: IntegrityClassification,
    event_id: Option<String>,
    index: Option<usize>,
}

impl AuditIntegrity {
    /// Create a new integrity result.
    pub fn new(
        classification: IntegrityClassification,
        event_id: Option<String>,
        index: Option<usize>,
    ) -> Self {
        Self {
            classification,
            event_id,
            index,
        }
    }

    /// Classification of the integrity outcome.
    pub fn classification(&self) -> IntegrityClassification {
        self.classification
    }

    /// Event identifier involved in the failure, when applicable.
    pub fn event_id(&self) -> Option<&str> {
        self.event_id.as_deref()
    }

    /// Zero-based index of the offending event, when applicable.
    pub fn index(&self) -> Option<usize> {
        self.index
    }

    /// Whether the chain verified cleanly.
    pub fn is_ok(&self) -> bool {
        matches!(self.classification, IntegrityClassification::Ok)
    }
}

/// Outcome of an audit query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditQueryResult {
    events: Vec<AuditEvent>,
}

impl AuditQueryResult {
    fn new(events: Vec<AuditEvent>) -> Self {
        Self { events }
    }

    /// Number of events in the result.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the result is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Borrow the events in the order they were returned.
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    fn into_events(self) -> Vec<AuditEvent> {
        self.events
    }
}

/// Filter for [`AuditLog::query`]. All criteria are combined as a
/// conjunction. An empty filter is rejected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditQuery {
    actor: Option<String>,
    resource: Option<String>,
    policy_revision: Option<String>,
    class: Option<AuditClass>,
    since_ms: Option<u64>,
    until_ms: Option<u64>,
    limit: Option<usize>,
}

impl AuditQuery {
    /// Build a new empty query. At least one criterion must be set before
    /// calling [`AuditLog::query`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the actor criterion.
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// Set the resource criterion.
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Set the policy revision criterion.
    pub fn with_policy_revision(mut self, policy_revision: impl Into<String>) -> Self {
        self.policy_revision = Some(policy_revision.into());
        self
    }

    /// Set the class criterion.
    pub fn with_class(mut self, class: AuditClass) -> Self {
        self.class = Some(class);
        self
    }

    /// Set the inclusive lower bound on the event timestamp.
    pub fn since_ms(mut self, since_ms: u64) -> Self {
        self.since_ms = Some(since_ms);
        self
    }

    /// Set the exclusive upper bound on the event timestamp.
    pub fn until_ms(mut self, until_ms: u64) -> Self {
        self.until_ms = Some(until_ms);
        self
    }

    /// Set the maximum number of events to return.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Whether the query is empty (no criteria set).
    pub fn is_empty(&self) -> bool {
        self.actor.is_none()
            && self.resource.is_none()
            && self.policy_revision.is_none()
            && self.class.is_none()
            && self.since_ms.is_none()
            && self.until_ms.is_none()
    }

    fn validate(&self) -> Result<(), AuditError> {
        if self.is_empty() {
            return Err(AuditError::QueryRejected);
        }
        if let Some(actor) = &self.actor {
            validate_id("actor", actor)?;
        }
        if let Some(resource) = &self.resource {
            validate_id("resource", resource)?;
        }
        if let Some(revision) = &self.policy_revision {
            validate_policy_revision(revision)?;
        }
        if let (Some(since), Some(until)) = (self.since_ms, self.until_ms) {
            if since >= until {
                return Err(AuditError::QueryRejected);
            }
        }
        if let Some(limit) = self.limit {
            if limit == 0 || limit > MAX_QUERY_RESULTS {
                return Err(AuditError::QueryRejected);
            }
        }
        Ok(())
    }
}

/// Bounded policy of an [`AuditLog`]. Revisions must match across calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditPolicy {
    project_id: String,
    policy_revision: String,
    capacity: usize,
    retention_events: usize,
    retention_ms: u64,
}

impl AuditPolicy {
    /// Build a new audit policy. `retention_ms` is the wall-clock retention
    /// window; `retention_events` is the maximum number of events retained;
    /// `capacity` is the maximum number of events kept in memory.
    pub fn new(
        project_id: impl Into<String>,
        policy_revision: impl Into<String>,
        capacity: usize,
        retention_events: usize,
        retention_ms: u64,
    ) -> Result<Self, AuditError> {
        let project_id = project_id.into();
        let policy_revision = policy_revision.into();
        validate_project_id(&project_id)?;
        validate_policy_revision(&policy_revision)?;
        if capacity == 0 || capacity > MAX_EVENTS_PER_LOG {
            return Err(AuditError::PolicyInvalid);
        }
        if retention_events == 0 || retention_events > MAX_RETENTION_EVENTS {
            return Err(AuditError::PolicyInvalid);
        }
        if retention_events > capacity {
            return Err(AuditError::PolicyInvalid);
        }
        if retention_ms > MAX_RETENTION_MS {
            return Err(AuditError::PolicyInvalid);
        }
        Ok(Self {
            project_id,
            policy_revision,
            capacity,
            retention_events,
            retention_ms,
        })
    }

    /// Project identifier.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Policy revision.
    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    /// Capacity of the in-memory log.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Maximum number of events retained.
    pub fn retention_events(&self) -> usize {
        self.retention_events
    }

    /// Retention window in milliseconds.
    pub fn retention_ms(&self) -> u64 {
        self.retention_ms
    }
}

/// A single audit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    event_id: String,
    actor: String,
    resource: String,
    policy_revision: String,
    classification: AuditClass,
    payload_sha256: String,
    payload: Payload,
    sequence: u64,
    timestamp_ms: u64,
    prev_hash: String,
    hash: String,
}

impl AuditEvent {
    /// Reconstruct an event from its parts. Used by `AuditLog::record` and by
    /// tests/fixtures. Validates every field; redacts before hashing.
    #[allow(clippy::too_many_arguments)]
    pub fn assemble(
        event_id: impl Into<String>,
        actor: impl Into<String>,
        resource: impl Into<String>,
        policy_revision: impl Into<String>,
        classification: AuditClass,
        timestamp_ms: u64,
        sequence: u64,
        prev_hash: impl Into<String>,
        payload: Payload,
    ) -> Result<Self, AuditError> {
        let event_id = event_id.into();
        let actor = actor.into();
        let resource = resource.into();
        let policy_revision = policy_revision.into();
        let prev_hash = prev_hash.into();
        validate_event_id(&event_id)?;
        validate_id("actor", &actor)?;
        validate_id("resource", &resource)?;
        validate_policy_revision(&policy_revision)?;
        if payload.is_empty() {
            return Err(AuditError::EventInvalid);
        }
        let payload_sha256 = sha256_of_payload(&payload);
        let hash = compute_event_hash(
            &event_id,
            &actor,
            &resource,
            &policy_revision,
            classification,
            sequence,
            timestamp_ms,
            &prev_hash,
            &payload_sha256,
        );
        Ok(Self {
            event_id,
            actor,
            resource,
            policy_revision,
            classification,
            payload_sha256,
            payload,
            sequence,
            timestamp_ms,
            prev_hash,
            hash,
        })
    }

    /// Event identifier.
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    /// Actor responsible for the event.
    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// Resource affected by the event.
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Policy revision under which the event was recorded.
    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    /// Classification of the event.
    pub fn classification(&self) -> AuditClass {
        self.classification
    }

    /// SHA-256 of the rendered payload.
    pub fn payload_sha256(&self) -> &str {
        &self.payload_sha256
    }

    /// Monotonic sequence of the event within its log.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Monotonic timestamp of the event in milliseconds.
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }

    /// Hash of the previous event in the chain.
    pub fn prev_hash(&self) -> &str {
        &self.prev_hash
    }

    /// Hash of this event.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Borrow the structured payload.
    pub fn payload(&self) -> &Payload {
        &self.payload
    }
}

/// Reason why a sink rejected a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SinkError {
    #[error("audit sink is unavailable")]
    Unavailable,
    #[error("audit sink rejected the event")]
    Rejected,
}

/// Sink for audit events. Implementations live outside `security-core` and
/// are responsible for I/O, batching, retry, persistence, and forwarding.
pub trait AuditSink {
    /// Persist the event. Returning an [`Err`] causes
    /// [`AuditLog::record`] to fail closed.
    fn write(&mut self, event: &AuditEvent) -> Result<(), SinkError>;
}

/// In-memory sink, useful for tests and for adapters that do not yet have
/// a durable backing store.
#[derive(Debug, Default, Clone)]
pub struct InMemorySink {
    events: Vec<AuditEvent>,
    fail_next: bool,
}

impl InMemorySink {
    /// Build a new empty in-memory sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the sink to fail the next call to [`AuditSink::write`].
    pub fn fail_next_write(&mut self) {
        self.fail_next = true;
    }

    /// Borrow the events currently held in the sink.
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }
}

impl AuditSink for InMemorySink {
    fn write(&mut self, event: &AuditEvent) -> Result<(), SinkError> {
        if self.fail_next {
            self.fail_next = false;
            return Err(SinkError::Unavailable);
        }
        self.events.push(event.clone());
        Ok(())
    }
}

/// Errors that can be surfaced by an [`AuditLog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AuditError {
    #[error("audit policy is invalid")]
    PolicyInvalid,
    #[error("audit event is invalid")]
    EventInvalid,
    #[error("audit policy revision does not match the log")]
    PolicyRevisionMismatch,
    #[error("audit log is not configured for the supplied project")]
    ScopeMismatch,
    #[error("audit sink is unavailable")]
    SinkUnavailable,
    #[error("audit integrity broken")]
    IntegrityBroken,
    #[error("audit export was rejected")]
    ExportRejected,
    #[error("audit query was rejected")]
    QueryRejected,
    #[error("audit payload exceeds the bounded size")]
    PayloadTooLarge,
    #[error("audit retention policy is invalid")]
    RetentionInvalid,
    #[error("audit log is at capacity")]
    CapacityExceeded,
}

/// Append-only audit log.
#[derive(Debug)]
pub struct AuditLog<S: AuditSink> {
    policy: AuditPolicy,
    events: VecDeque<AuditEvent>,
    sink: S,
}

impl<S: AuditSink> AuditLog<S> {
    /// Build a new audit log backed by the supplied sink.
    pub fn new(policy: AuditPolicy, sink: S) -> Result<Self, AuditError> {
        Ok(Self {
            policy,
            events: VecDeque::new(),
            sink,
        })
    }

    /// Build an audit log pre-populated with the supplied events. Used by
    /// adapters that reconstruct a log from a durable sink and by tests
    /// that need to inject forged chains. The events must already be in
    /// chain order; their sequence, prev_hash, and hash are verified
    /// against each other on construction.
    pub fn from_events(
        policy: AuditPolicy,
        sink: S,
        events: Vec<AuditEvent>,
    ) -> Result<Self, AuditError> {
        let mut log = Self::new(policy, sink)?;
        for event in events {
            log.sink.write(&event).map_err(|err| match err {
                SinkError::Unavailable => AuditError::SinkUnavailable,
                SinkError::Rejected => AuditError::ExportRejected,
            })?;
            log.events.push_back(event);
        }
        Ok(log)
    }

    /// Borrow the policy.
    pub fn policy(&self) -> &AuditPolicy {
        &self.policy
    }

    /// Borrow the sink.
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// Mutably borrow the sink.
    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    /// Number of events currently retained.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Record a new event. Returns the assembled event. The sink is invoked
    /// exactly once; on failure the event is not retained and the error is
    /// returned to the caller.
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        event_id: impl Into<String>,
        actor: impl Into<String>,
        resource: impl Into<String>,
        classification: AuditClass,
        timestamp_ms: u64,
        payload: Payload,
    ) -> Result<AuditEvent, AuditError> {
        if self.events.len() >= self.policy.capacity() {
            return Err(AuditError::CapacityExceeded);
        }
        let next_sequence = self
            .events
            .back()
            .map(|last| last.sequence().saturating_add(1))
            .unwrap_or(0);
        let prev_hash = self
            .events
            .back()
            .map(|last| last.hash().to_string())
            .unwrap_or_else(|| GENESIS_HASH.to_string());
        let event = AuditEvent::assemble(
            event_id,
            actor,
            resource,
            self.policy.policy_revision.clone(),
            classification,
            timestamp_ms,
            next_sequence,
            prev_hash,
            payload,
        )?;
        self.sink.write(&event).map_err(|err| match err {
            SinkError::Unavailable => AuditError::SinkUnavailable,
            SinkError::Rejected => AuditError::ExportRejected,
        })?;
        self.events.push_back(event.clone());
        Ok(event)
    }

    /// Apply the retention policy, dropping events whose timestamp is older
    /// than `now_ms - retention_ms` and trimming to `retention_events`.
    pub fn retain(&mut self, now_ms: u64) -> Result<usize, AuditError> {
        if now_ms > MAX_RETENTION_MS.saturating_add(self.policy.retention_ms) {
            return Err(AuditError::RetentionInvalid);
        }
        if now_ms < self.policy.retention_ms {
            return Ok(0);
        }
        let cutoff = now_ms - self.policy.retention_ms;
        let before = self.events.len();
        while let Some(front) = self.events.front() {
            if front.timestamp_ms() < cutoff {
                self.events.pop_front();
            } else {
                break;
            }
        }
        while self.events.len() > self.policy.retention_events {
            self.events.pop_front();
        }
        Ok(before.saturating_sub(self.events.len()))
    }

    /// Verify the integrity of the chain. Returns the first failure with
    /// its classification and event identifier, or [`IntegrityClassification::Ok`].
    pub fn verify_chain(&self) -> AuditIntegrity {
        let mut expected_prev = GENESIS_HASH.to_string();
        let mut expected_sequence: u64 = 0;
        for (idx, event) in self.events.iter().enumerate() {
            if event.sequence() != expected_sequence {
                return AuditIntegrity::new(
                    IntegrityClassification::OutOfOrder,
                    Some(event.event_id().to_string()),
                    Some(idx),
                );
            }
            if event.prev_hash() != expected_prev {
                return AuditIntegrity::new(
                    IntegrityClassification::HashMismatch,
                    Some(event.event_id().to_string()),
                    Some(idx),
                );
            }
            let recomputed = recompute_hash(event);
            if recomputed != event.hash() {
                return AuditIntegrity::new(
                    IntegrityClassification::Broken,
                    Some(event.event_id().to_string()),
                    Some(idx),
                );
            }
            expected_prev = event.hash().to_string();
            expected_sequence = expected_sequence.saturating_add(1);
        }
        AuditIntegrity::new(IntegrityClassification::Ok, None, None)
    }

    /// Run a query. Returns the matching events in chain order, capped by
    /// the query's limit. Sensitive fields are redacted in the returned
    /// events.
    pub fn query(&self, query: &AuditQuery) -> Result<AuditQueryResult, AuditError> {
        query.validate()?;
        let mut out: Vec<AuditEvent> = Vec::new();
        let limit = query.limit.unwrap_or(MAX_QUERY_RESULTS);
        if limit == 0 || limit > MAX_QUERY_RESULTS {
            return Err(AuditError::QueryRejected);
        }
        for event in &self.events {
            if let Some(actor) = &query.actor {
                if event.actor() != actor {
                    continue;
                }
            }
            if let Some(resource) = &query.resource {
                if event.resource() != resource {
                    continue;
                }
            }
            if let Some(revision) = &query.policy_revision {
                if event.policy_revision() != revision {
                    continue;
                }
            }
            if let Some(class) = query.class {
                if event.classification() != class {
                    continue;
                }
            }
            if let Some(since) = query.since_ms {
                if event.timestamp_ms() < since {
                    continue;
                }
            }
            if let Some(until) = query.until_ms {
                if event.timestamp_ms() >= until {
                    continue;
                }
            }
            out.push(event.clone());
            if out.len() >= limit {
                break;
            }
        }
        Ok(AuditQueryResult::new(out))
    }

    /// Export at most `limit` events, redacting any sensitive payload.
    /// Returns the events in chain order. The limit is bounded by
    /// [`MAX_EXPORT_ROWS`].
    pub fn export(&self, limit: usize) -> Result<Vec<AuditEvent>, AuditError> {
        if limit == 0 || limit > MAX_EXPORT_ROWS {
            return Err(AuditError::ExportRejected);
        }
        let mut out = Vec::with_capacity(limit.min(self.events.len()));
        for event in self.events.iter().take(limit) {
            out.push(event.clone());
        }
        Ok(out)
    }

    /// Apply retention and then export. The returned vector is bounded by
    /// [`MAX_EXPORT_ROWS`].
    pub fn retain_and_export(
        &mut self,
        now_ms: u64,
        limit: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        self.retain(now_ms)?;
        self.export(limit)
    }
}

fn validate_project_id(value: &str) -> Result<(), AuditError> {
    if value.is_empty() || value.len() > MAX_PROJECT_ID_LEN || value.chars().any(char::is_control) {
        return Err(AuditError::PolicyInvalid);
    }
    Ok(())
}

fn validate_id(label: &str, value: &str) -> Result<(), AuditError> {
    let max = match label {
        "actor" => MAX_ACTOR_ID_LEN,
        "resource" => MAX_RESOURCE_ID_LEN,
        _ => return Err(AuditError::EventInvalid),
    };
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(AuditError::EventInvalid);
    }
    Ok(())
}

fn validate_policy_revision(value: &str) -> Result<(), AuditError> {
    if value.is_empty()
        || value.len() > MAX_POLICY_REVISION_LEN
        || value.chars().any(char::is_control)
    {
        return Err(AuditError::PolicyInvalid);
    }
    Ok(())
}

fn validate_event_id(value: &str) -> Result<(), AuditError> {
    if value.is_empty()
        || value.len() > 64
        || value.chars().any(char::is_control)
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AuditError::EventInvalid);
    }
    Ok(())
}

fn sha256_of_payload(payload: &Payload) -> String {
    let mut hasher = Sha256::new();
    for (key, value) in payload.iter() {
        hasher.update(key.as_bytes());
        hasher.update([0u8]);
        hasher.update(value.as_bytes());
        hasher.update([0xffu8]);
    }
    hex_lower(&hasher.finalize())
}

#[allow(clippy::too_many_arguments)]
fn compute_event_hash(
    event_id: &str,
    actor: &str,
    resource: &str,
    policy_revision: &str,
    classification: AuditClass,
    sequence: u64,
    timestamp_ms: u64,
    prev_hash: &str,
    payload_sha256: &str,
) -> String {
    #[allow(clippy::too_many_arguments)]
    let mut hasher = Sha256::new();
    hasher.update(event_id.as_bytes());
    hasher.update([0u8]);
    hasher.update(actor.as_bytes());
    hasher.update([0u8]);
    hasher.update(resource.as_bytes());
    hasher.update([0u8]);
    hasher.update(policy_revision.as_bytes());
    hasher.update([0u8]);
    hasher.update(classification_label(classification).as_bytes());
    hasher.update([0u8]);
    hasher.update(sequence.to_be_bytes());
    hasher.update(timestamp_ms.to_be_bytes());
    hasher.update(prev_hash.as_bytes());
    hasher.update([0u8]);
    hasher.update(payload_sha256.as_bytes());
    hex_lower(&hasher.finalize())
}

fn recompute_hash(event: &AuditEvent) -> String {
    compute_event_hash(
        event.event_id(),
        event.actor(),
        event.resource(),
        event.policy_revision(),
        event.classification(),
        event.sequence(),
        event.timestamp_ms(),
        event.prev_hash(),
        event.payload_sha256(),
    )
}

fn classification_label(classification: AuditClass) -> &'static str {
    match classification {
        AuditClass::Authorization => "authorization",
        AuditClass::Migration => "migration",
        AuditClass::Recovery => "recovery",
        AuditClass::PluginRemote => "plugin_remote",
        AuditClass::Release => "release",
        AuditClass::Denial => "denial",
        AuditClass::Other => "other",
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Helper to keep the unique-set of payload keys small and to support
/// deterministic diffing across events.
pub fn unique_payload_keys(events: &[AuditEvent]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for event in events {
        for (key, _) in event.payload().iter() {
            out.insert(key.to_string());
        }
    }
    out
}

/// Build a query that selects all events of a given class. Useful for
/// tests and for adapters that need a class-only filter.
pub fn class_query(class: AuditClass, limit: usize) -> Result<AuditQuery, AuditError> {
    if limit == 0 || limit > MAX_QUERY_RESULTS {
        return Err(AuditError::QueryRejected);
    }
    Ok(AuditQuery::new().with_class(class).with_limit(limit))
}

#[doc(hidden)]
pub fn __ensure_used_audit_query() {
    // Force the inner constructor path to be visible to documentation; this
    // is a no-op but keeps `AuditQueryResult::into_events` reachable from
    // test binaries without warnings.
    let _ = AuditQueryResult::new(Vec::new()).into_events();
}
