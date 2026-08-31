//! Versioned Claim/Evidence facts with a fail-closed state machine.
//!
//! This module is intentionally provider-neutral. It stores only bounded
//! claim digests and resolver-produced evidence records; claim text is not a
//! fact and there is no execution, approval, persistence, or provider path.

use crate::{ProjectId, RunId, TraceId};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const CLAIM_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const MAX_CLAIM_ID_LEN: usize = 128;
pub const MAX_RESOLVER_ID_LEN: usize = 128;
pub const MAX_REVISION_LEN: usize = 128;
pub const MAX_REASON_LEN: usize = 256;
pub const MAX_REQUIRED_EVIDENCE: usize = 8;
pub const MAX_CLAIM_EVIDENCE_REFERENCES: usize = 16;
pub const MAX_EVIDENCE_RECORDS: usize = 128;

const DIGEST_LEN: usize = 64;
const MAX_REASONABLE_SHA_LEN: usize = 64;
const MIN_SHA_LEN: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimClass {
    PlanFinding,
    RepositoryState,
    CodeChange,
    TestResult,
    ExternalFact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimEvidenceKind {
    Commit,
    Tree,
    Policy,
    Schema,
    Test,
    Artifact,
}

/// Fact states are data-only. `Verified` is reachable only through a
/// resolution carrying matching resolver evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactState {
    Verified,
    Unverified,
    Conflicting,
    Stale,
    NoProof,
}

/// Compatibility name used by consumers that refer to the state as a claim
/// state rather than a fact state.
pub type ClaimState = FactState;

impl FactState {
    fn default_reason(self) -> &'static str {
        match self {
            Self::Verified => "resolver evidence matched claim",
            Self::Unverified => "resolver evidence is insufficient",
            Self::Conflicting => "resolver evidence conflicts",
            Self::Stale => "resolver evidence is stale",
            Self::NoProof => "no resolver evidence is available",
        }
    }

    /// Returns whether the bounded lifecycle permits a transition.
    ///
    /// Replaying the same state is allowed for idempotency. A verified fact
    /// cannot silently be downgraded to merely unverified; it must first be
    /// marked stale, conflicting, or without proof.
    pub fn can_transition_to(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (
                    Self::NoProof,
                    Self::Unverified | Self::Verified | Self::Stale | Self::Conflicting
                ) | (
                    Self::Unverified,
                    Self::Verified | Self::Stale | Self::Conflicting | Self::NoProof
                ) | (
                    Self::Verified,
                    Self::Stale | Self::Conflicting | Self::NoProof
                ) | (
                    Self::Stale,
                    Self::Unverified | Self::Verified | Self::Conflicting | Self::NoProof
                ) | (
                    Self::Conflicting,
                    Self::Unverified | Self::Verified | Self::Stale | Self::NoProof
                )
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClaimEvidenceError {
    #[error("claim/evidence schema version is unsupported")]
    UnsupportedSchemaVersion,
    #[error("claim or evidence field is outside its bound")]
    BoundsExceeded,
    #[error("claim or evidence shape is invalid")]
    InvalidShape,
    #[error("claim or evidence digest is invalid")]
    InvalidDigest,
    #[error("sensitive value is not allowed in claim/evidence metadata")]
    SensitiveValue,
    #[error("claim identity does not match its evidence")]
    IdentityMismatch,
    #[error("evidence belongs to a different claim")]
    ClaimMismatch,
    #[error("evidence identifier or kind is duplicated")]
    DuplicateEvidence,
    #[error("required evidence is missing")]
    MissingEvidence,
    #[error("a required evidence kind is missing")]
    MissingRequiredEvidence,
    #[error("evidence for the requested state is missing")]
    MissingStateEvidence,
    #[error("evidence status is incompatible with the requested claim state")]
    InvalidEvidenceStatus,
    #[error("evidence is not valid for the requested claim state")]
    InvalidEvidence,
    #[error("claim state transition is invalid: {from:?} -> {to:?}")]
    InvalidTransition { from: FactState, to: FactState },
}

/// Short compatibility alias for callers that use the domain-wide error
/// naming convention.
pub type ClaimError = ClaimEvidenceError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceScope {
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub identity_digest: String,
    pub head_sha: Option<String>,
    pub tree_sha: Option<String>,
    pub policy_revision: Option<String>,
    pub schema_revision: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceScopeWire {
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    identity_digest: String,
    head_sha: Option<String>,
    tree_sha: Option<String>,
    policy_revision: Option<String>,
    schema_revision: Option<String>,
}

impl<'de> Deserialize<'de> for EvidenceScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EvidenceScopeWire::deserialize(deserializer)?;
        Self::new(
            wire.project_id,
            wire.run_id,
            wire.trace_id,
            wire.identity_digest,
            wire.head_sha,
            wire.tree_sha,
            wire.policy_revision,
            wire.schema_revision,
        )
        .map_err(D::Error::custom)
    }
}

impl EvidenceScope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
        identity_digest: String,
        head_sha: Option<String>,
        tree_sha: Option<String>,
        policy_revision: Option<String>,
        schema_revision: Option<String>,
    ) -> Result<Self, ClaimEvidenceError> {
        let scope = Self {
            project_id,
            run_id,
            trace_id,
            identity_digest,
            head_sha,
            tree_sha,
            policy_revision,
            schema_revision,
        };
        scope.validate()?;
        Ok(scope)
    }

    pub fn head_sha(&self) -> Option<&str> {
        self.head_sha.as_deref()
    }

    pub fn identity_digest(&self) -> &str {
        &self.identity_digest
    }

    pub fn tree_sha(&self) -> Option<&str> {
        self.tree_sha.as_deref()
    }

    pub fn policy_revision(&self) -> Option<&str> {
        self.policy_revision.as_deref()
    }

    pub fn schema_revision(&self) -> Option<&str> {
        self.schema_revision.as_deref()
    }

    pub fn validate(&self) -> Result<(), ClaimEvidenceError> {
        validate_digest(&self.identity_digest)?;
        validate_optional_sha(self.head_sha.as_deref())?;
        validate_optional_sha(self.tree_sha.as_deref())?;
        validate_optional_text(self.policy_revision.as_deref(), MAX_REVISION_LEN)?;
        validate_optional_text(self.schema_revision.as_deref(), MAX_REVISION_LEN)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    schema_version: u32,
    evidence_id: String,
    claim_id: String,
    kind: ClaimEvidenceKind,
    scope: EvidenceScope,
    evidence_digest: String,
    resolver_id: String,
    status: EvidenceStatus,
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceRecordWire {
    schema_version: u32,
    evidence_id: String,
    claim_id: String,
    kind: ClaimEvidenceKind,
    scope: EvidenceScope,
    evidence_digest: String,
    resolver_id: String,
    status: EvidenceStatus,
    reason: String,
}

impl<'de> Deserialize<'de> for EvidenceRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EvidenceRecordWire::deserialize(deserializer)?;
        let record = Self {
            schema_version: wire.schema_version,
            evidence_id: wire.evidence_id,
            claim_id: wire.claim_id,
            kind: wire.kind,
            scope: wire.scope,
            evidence_digest: wire.evidence_digest,
            resolver_id: wire.resolver_id,
            status: wire.status,
            reason: wire.reason,
        };
        record.validate().map_err(D::Error::custom)?;
        Ok(record)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Verified,
    Unverified,
    Conflicting,
    Stale,
    NoProof,
}

impl EvidenceStatus {
    fn default_reason(self) -> &'static str {
        match self {
            Self::Verified => "resolver evidence verified",
            Self::Unverified => "resolver evidence did not verify",
            Self::Conflicting => "resolver evidence conflicts",
            Self::Stale => "resolver evidence is stale",
            Self::NoProof => "resolver returned no proof",
        }
    }
}

impl EvidenceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        claim_id: impl Into<String>,
        evidence_id: impl Into<String>,
        kind: ClaimEvidenceKind,
        scope: EvidenceScope,
        evidence_digest: impl Into<String>,
        resolver_id: impl Into<String>,
        status: EvidenceStatus,
    ) -> Result<Self, ClaimEvidenceError> {
        let record = Self {
            schema_version: CLAIM_EVIDENCE_SCHEMA_VERSION,
            evidence_id: evidence_id.into(),
            claim_id: claim_id.into(),
            kind,
            scope,
            evidence_digest: evidence_digest.into(),
            resolver_id: resolver_id.into(),
            status,
            reason: status.default_reason().into(),
        };
        record.validate()?;
        Ok(record)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn kind(&self) -> ClaimEvidenceKind {
        self.kind
    }

    pub fn scope(&self) -> &EvidenceScope {
        &self.scope
    }

    pub fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }

    pub fn resolver_id(&self) -> &str {
        &self.resolver_id
    }

    pub fn status(&self) -> EvidenceStatus {
        self.status
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Result<Self, ClaimEvidenceError> {
        self.reason = reason.into();
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), ClaimEvidenceError> {
        if self.schema_version != CLAIM_EVIDENCE_SCHEMA_VERSION {
            return Err(ClaimEvidenceError::UnsupportedSchemaVersion);
        }
        validate_text(&self.evidence_id, MAX_CLAIM_ID_LEN)?;
        validate_text(&self.claim_id, MAX_CLAIM_ID_LEN)?;
        validate_digest(&self.evidence_digest)?;
        validate_text(&self.resolver_id, MAX_RESOLVER_ID_LEN)?;
        validate_text(&self.reason, MAX_REASON_LEN)?;
        self.scope.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimResolution {
    state: FactState,
    evidence_ids: Vec<String>,
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimResolutionWire {
    state: FactState,
    evidence_ids: Vec<String>,
    reason: String,
}

impl<'de> Deserialize<'de> for ClaimResolution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ClaimResolutionWire::deserialize(deserializer)?;
        let resolution = Self {
            state: wire.state,
            evidence_ids: wire.evidence_ids,
            reason: wire.reason,
        };
        resolution.validate().map_err(D::Error::custom)?;
        Ok(resolution)
    }
}

impl ClaimResolution {
    pub fn new(state: FactState, evidence_ids: Vec<String>) -> Self {
        Self {
            state,
            evidence_ids,
            reason: state.default_reason().into(),
        }
    }

    pub fn verified(evidence_ids: Vec<String>) -> Self {
        Self::new(FactState::Verified, evidence_ids)
    }

    pub fn unverified(evidence_ids: Vec<String>) -> Self {
        Self::new(FactState::Unverified, evidence_ids)
    }

    pub fn conflicting(evidence_ids: Vec<String>) -> Self {
        Self::new(FactState::Conflicting, evidence_ids)
    }

    pub fn stale(evidence_ids: Vec<String>) -> Self {
        Self::new(FactState::Stale, evidence_ids)
    }

    pub fn no_proof() -> Self {
        Self::new(FactState::NoProof, Vec::new())
    }

    pub fn state(&self) -> FactState {
        self.state
    }

    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Result<Self, ClaimEvidenceError> {
        self.reason = reason.into();
        self.validate()?;
        Ok(self)
    }

    fn validate(&self) -> Result<(), ClaimEvidenceError> {
        validate_evidence_ids(&self.evidence_ids)?;
        validate_text(&self.reason, MAX_REASON_LEN)?;
        if self.state == FactState::Verified && self.evidence_ids.is_empty() {
            return Err(ClaimEvidenceError::MissingEvidence);
        }
        if self.state == FactState::NoProof && !self.evidence_ids.is_empty() {
            return Err(ClaimEvidenceError::InvalidEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionOutcome {
    Applied,
    Idempotent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    schema_version: u32,
    claim_id: String,
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    claim_class: ClaimClass,
    claim_digest: String,
    required_evidence: Vec<ClaimEvidenceKind>,
    expected_identity: EvidenceScope,
    state: FactState,
    evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimWire {
    schema_version: u32,
    claim_id: String,
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    claim_class: ClaimClass,
    claim_digest: String,
    required_evidence: Vec<ClaimEvidenceKind>,
    expected_identity: EvidenceScope,
    state: FactState,
    evidence_ids: Vec<String>,
}

impl<'de> Deserialize<'de> for Claim {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ClaimWire::deserialize(deserializer)?;
        let claim = Self {
            schema_version: wire.schema_version,
            claim_id: wire.claim_id,
            project_id: wire.project_id,
            run_id: wire.run_id,
            trace_id: wire.trace_id,
            claim_class: wire.claim_class,
            claim_digest: wire.claim_digest,
            required_evidence: wire.required_evidence,
            expected_identity: wire.expected_identity,
            state: wire.state,
            evidence_ids: wire.evidence_ids,
        };
        claim.validate().map_err(D::Error::custom)?;
        Ok(claim)
    }
}

impl Claim {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
        claim_id: impl Into<String>,
        claim_class: ClaimClass,
        claim_digest: impl Into<String>,
        required_evidence: Vec<ClaimEvidenceKind>,
        expected_identity: EvidenceScope,
    ) -> Result<Self, ClaimEvidenceError> {
        let claim = Self {
            schema_version: CLAIM_EVIDENCE_SCHEMA_VERSION,
            claim_id: claim_id.into(),
            project_id,
            run_id,
            trace_id,
            claim_class,
            claim_digest: claim_digest.into(),
            required_evidence,
            expected_identity,
            state: FactState::NoProof,
            evidence_ids: Vec::new(),
        };
        claim.validate()?;
        Ok(claim)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn run_id(&self) -> RunId {
        self.run_id
    }

    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    pub fn claim_class(&self) -> ClaimClass {
        self.claim_class
    }

    pub fn claim_digest(&self) -> &str {
        &self.claim_digest
    }

    pub fn required_evidence(&self) -> &[ClaimEvidenceKind] {
        &self.required_evidence
    }

    pub fn expected_identity(&self) -> &EvidenceScope {
        &self.expected_identity
    }

    pub fn state(&self) -> FactState {
        self.state
    }

    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }

    /// Applies a resolver-produced state decision after validating every
    /// selected evidence record against the claim identity and requirements.
    pub fn apply_resolution(
        &mut self,
        resolution: ClaimResolution,
        records: &[EvidenceRecord],
    ) -> Result<ResolutionOutcome, ClaimEvidenceError> {
        self.validate()?;
        resolution.validate()?;

        if !self.state.can_transition_to(resolution.state) {
            return Err(ClaimEvidenceError::InvalidTransition {
                from: self.state,
                to: resolution.state,
            });
        }
        if records.len() > MAX_EVIDENCE_RECORDS {
            return Err(ClaimEvidenceError::BoundsExceeded);
        }

        let selected = self.select_evidence(resolution.evidence_ids(), records)?;
        validate_resolution_evidence(self, resolution.state, resolution.evidence_ids(), &selected)?;

        if self.state == resolution.state && self.evidence_ids == resolution.evidence_ids {
            return Ok(ResolutionOutcome::Idempotent);
        }

        self.state = resolution.state;
        self.evidence_ids = resolution.evidence_ids;
        Ok(ResolutionOutcome::Applied)
    }

    /// Claims are not authority. Execution belongs to an outer authorization
    /// boundary and cannot be inferred from a fact state.
    pub fn can_execute(&self) -> bool {
        false
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn can_merge(&self) -> bool {
        false
    }

    pub fn validate(&self) -> Result<(), ClaimEvidenceError> {
        if self.schema_version != CLAIM_EVIDENCE_SCHEMA_VERSION {
            return Err(ClaimEvidenceError::UnsupportedSchemaVersion);
        }
        validate_text(&self.claim_id, MAX_CLAIM_ID_LEN)?;
        validate_digest(&self.claim_digest)?;
        self.expected_identity.validate()?;
        if self.expected_identity.project_id != self.project_id
            || self.expected_identity.run_id != self.run_id
            || self.expected_identity.trace_id != self.trace_id
        {
            return Err(ClaimEvidenceError::IdentityMismatch);
        }
        validate_required_evidence(&self.required_evidence)?;
        validate_evidence_ids(&self.evidence_ids)?;
        if self.state == FactState::Verified && self.evidence_ids.is_empty() {
            return Err(ClaimEvidenceError::MissingEvidence);
        }
        if self.state == FactState::NoProof && !self.evidence_ids.is_empty() {
            return Err(ClaimEvidenceError::InvalidEvidence);
        }
        Ok(())
    }

    fn select_evidence<'a>(
        &self,
        evidence_ids: &[String],
        records: &'a [EvidenceRecord],
    ) -> Result<Vec<&'a EvidenceRecord>, ClaimEvidenceError> {
        let mut selected = Vec::with_capacity(evidence_ids.len());
        for evidence_id in evidence_ids {
            let mut matches = records
                .iter()
                .filter(|record| record.evidence_id() == evidence_id);
            let record = matches.next().ok_or(ClaimEvidenceError::MissingEvidence)?;
            if matches.next().is_some() {
                return Err(ClaimEvidenceError::DuplicateEvidence);
            }
            record.validate()?;
            if record.claim_id() != self.claim_id {
                return Err(ClaimEvidenceError::ClaimMismatch);
            }
            if record.scope() != &self.expected_identity {
                return Err(ClaimEvidenceError::IdentityMismatch);
            }
            selected.push(record);
        }
        Ok(selected)
    }
}

fn validate_resolution_evidence(
    claim: &Claim,
    state: FactState,
    evidence_ids: &[String],
    selected: &[&EvidenceRecord],
) -> Result<(), ClaimEvidenceError> {
    if state == FactState::NoProof {
        if !evidence_ids.is_empty() {
            return Err(ClaimEvidenceError::InvalidEvidence);
        }
        return Ok(());
    }

    match state {
        FactState::Verified => {
            if selected.is_empty() {
                return Err(ClaimEvidenceError::MissingEvidence);
            }
            if selected
                .iter()
                .any(|record| record.status() != EvidenceStatus::Verified)
            {
                return Err(ClaimEvidenceError::InvalidEvidenceStatus);
            }
            if claim
                .required_evidence
                .iter()
                .any(|required| !selected.iter().any(|record| record.kind() == *required))
            {
                return Err(ClaimEvidenceError::MissingRequiredEvidence);
            }
        }
        FactState::Unverified => {
            if selected.iter().any(|record| {
                !matches!(
                    record.status(),
                    EvidenceStatus::Unverified | EvidenceStatus::NoProof
                )
            }) {
                return Err(ClaimEvidenceError::InvalidEvidenceStatus);
            }
        }
        FactState::Stale => {
            if !selected
                .iter()
                .any(|record| record.status() == EvidenceStatus::Stale)
            {
                return Err(ClaimEvidenceError::MissingStateEvidence);
            }
        }
        FactState::Conflicting => {
            if !selected
                .iter()
                .any(|record| record.status() == EvidenceStatus::Conflicting)
            {
                return Err(ClaimEvidenceError::MissingStateEvidence);
            }
        }
        FactState::NoProof => unreachable!(),
    }
    Ok(())
}

fn validate_required_evidence(required: &[ClaimEvidenceKind]) -> Result<(), ClaimEvidenceError> {
    if required.is_empty() || required.len() > MAX_REQUIRED_EVIDENCE {
        return Err(ClaimEvidenceError::BoundsExceeded);
    }
    let mut seen = BTreeSet::new();
    if required.iter().any(|kind| !seen.insert(*kind)) {
        return Err(ClaimEvidenceError::DuplicateEvidence);
    }
    Ok(())
}

fn validate_evidence_ids(ids: &[String]) -> Result<(), ClaimEvidenceError> {
    if ids.len() > MAX_CLAIM_EVIDENCE_REFERENCES {
        return Err(ClaimEvidenceError::BoundsExceeded);
    }
    let mut seen = BTreeSet::new();
    for id in ids {
        validate_text(id, MAX_CLAIM_ID_LEN)?;
        if !seen.insert(id) {
            return Err(ClaimEvidenceError::DuplicateEvidence);
        }
    }
    Ok(())
}

fn validate_text(value: &str, max_len: usize) -> Result<(), ClaimEvidenceError> {
    if value.is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(ClaimEvidenceError::BoundsExceeded);
    }
    if contains_sensitive_marker(value) {
        return Err(ClaimEvidenceError::SensitiveValue);
    }
    Ok(())
}

fn validate_optional_text(value: Option<&str>, max_len: usize) -> Result<(), ClaimEvidenceError> {
    if let Some(value) = value {
        validate_text(value, max_len)?;
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ClaimEvidenceError> {
    if value.len() != DIGEST_LEN || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ClaimEvidenceError::InvalidDigest);
    }
    Ok(())
}

fn validate_optional_sha(value: Option<&str>) -> Result<(), ClaimEvidenceError> {
    if let Some(value) = value {
        if !(MIN_SHA_LEN..=MAX_REASONABLE_SHA_LEN).contains(&value.len())
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ClaimEvidenceError::InvalidDigest);
        }
    }
    Ok(())
}

fn contains_sensitive_marker(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "token=",
        "secret=",
        "api_key=",
        "password=",
        "authorization:",
        "bearer ",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}
