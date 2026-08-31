//! Provider-neutral binding from planning findings to Claim/Evidence facts.
//!
//! This module accepts only resolver-produced [`EvidenceRecord`] values. It
//! never treats a reviewer reference or reviewer prose as proof and it does
//! not access storage, tools, providers or authorization boundaries.

use crate::claim_evidence::{
    Claim, ClaimClass, ClaimEvidenceError, ClaimEvidenceKind, ClaimResolution, EvidenceRecord,
    EvidenceScope, EvidenceStatus as ClaimEvidenceStatus, FactState, MAX_EVIDENCE_RECORDS,
    MAX_REQUIRED_EVIDENCE,
};
use crate::planning_reconciliation::{
    EvidenceStatus as PlanningEvidenceStatus, ReconciliationError, ReviewerFinding,
};
use crate::{ProjectId, RunId, TraceId};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const PLANNING_EVIDENCE_BINDING_SCHEMA_VERSION: u32 = 1;
pub const MAX_BINDING_IDEMPOTENCY_KEY_LEN: usize = 128;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBindingMetrics {
    pub total_records: usize,
    pub verified: usize,
    pub unverified: usize,
    pub stale: usize,
    pub conflicting: usize,
    pub no_proof: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningEvidenceBindingRequest {
    schema_version: u32,
    idempotency_key: String,
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    finding: ReviewerFinding,
    expected_identity: EvidenceScope,
    required_evidence: Vec<ClaimEvidenceKind>,
    evidence_records: Vec<EvidenceRecord>,
    cancelled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningEvidenceBindingRequestWire {
    schema_version: u32,
    idempotency_key: String,
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    finding: ReviewerFinding,
    expected_identity: EvidenceScope,
    required_evidence: Vec<ClaimEvidenceKind>,
    evidence_records: Vec<EvidenceRecord>,
    cancelled: bool,
}

impl<'de> Deserialize<'de> for PlanningEvidenceBindingRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlanningEvidenceBindingRequestWire::deserialize(deserializer)?;
        let request = Self {
            schema_version: wire.schema_version,
            idempotency_key: wire.idempotency_key,
            project_id: wire.project_id,
            run_id: wire.run_id,
            trace_id: wire.trace_id,
            finding: wire.finding,
            expected_identity: wire.expected_identity,
            required_evidence: wire.required_evidence,
            evidence_records: wire.evidence_records,
            cancelled: wire.cancelled,
        };
        request.validate().map_err(D::Error::custom)?;
        Ok(request)
    }
}

impl PlanningEvidenceBindingRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
        idempotency_key: impl Into<String>,
        finding: ReviewerFinding,
        expected_identity: EvidenceScope,
        required_evidence: Vec<ClaimEvidenceKind>,
        evidence_records: Vec<EvidenceRecord>,
    ) -> Result<Self, PlanningEvidenceBindingError> {
        let request = Self {
            schema_version: PLANNING_EVIDENCE_BINDING_SCHEMA_VERSION,
            idempotency_key: idempotency_key.into(),
            project_id,
            run_id,
            trace_id,
            finding,
            expected_identity,
            required_evidence,
            evidence_records,
            cancelled: false,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_cancelled(mut self, cancelled: bool) -> Self {
        self.cancelled = cancelled;
        self
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    pub fn finding(&self) -> &ReviewerFinding {
        &self.finding
    }

    pub fn expected_identity(&self) -> &EvidenceScope {
        &self.expected_identity
    }

    pub fn required_evidence(&self) -> &[ClaimEvidenceKind] {
        &self.required_evidence
    }

    pub fn evidence_records(&self) -> &[EvidenceRecord] {
        &self.evidence_records
    }

    pub fn validate(&self) -> Result<(), PlanningEvidenceBindingError> {
        if self.schema_version != PLANNING_EVIDENCE_BINDING_SCHEMA_VERSION {
            return Err(PlanningEvidenceBindingError::UnsupportedSchemaVersion);
        }
        if !bounded_text(&self.idempotency_key, MAX_BINDING_IDEMPOTENCY_KEY_LEN) {
            return Err(PlanningEvidenceBindingError::InvalidIdentity);
        }
        self.expected_identity.validate()?;
        if self.project_id != self.finding.project_id
            || self.run_id != self.finding.run_id
            || self.trace_id != self.finding.trace_id
            || self.expected_identity.project_id != self.project_id
            || self.expected_identity.run_id != self.run_id
            || self.expected_identity.trace_id != self.trace_id
        {
            return Err(PlanningEvidenceBindingError::IdentityMismatch);
        }
        self.finding
            .validate_shape()
            .map_err(PlanningEvidenceBindingError::InvalidFinding)?;
        validate_required_evidence(&self.required_evidence)?;
        if self.evidence_records.len() > MAX_EVIDENCE_RECORDS {
            return Err(PlanningEvidenceBindingError::BoundsExceeded);
        }
        let mut evidence_ids = BTreeSet::new();
        for record in &self.evidence_records {
            record.validate()?;
            if !evidence_ids.insert(record.evidence_id()) {
                return Err(PlanningEvidenceBindingError::ClaimEvidence(
                    ClaimEvidenceError::DuplicateEvidence,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningEvidenceBinding {
    schema_version: u32,
    idempotency_key: String,
    claim: Claim,
    evidence_records: Vec<EvidenceRecord>,
    state: FactState,
    effective_disposition: Option<crate::planning_reconciliation::Disposition>,
    metrics: EvidenceBindingMetrics,
    fingerprint: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningEvidenceBindingWire {
    schema_version: u32,
    idempotency_key: String,
    claim: Claim,
    evidence_records: Vec<EvidenceRecord>,
    state: FactState,
    effective_disposition: Option<crate::planning_reconciliation::Disposition>,
    metrics: EvidenceBindingMetrics,
    fingerprint: String,
}

impl<'de> Deserialize<'de> for PlanningEvidenceBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlanningEvidenceBindingWire::deserialize(deserializer)?;
        let binding = Self {
            schema_version: wire.schema_version,
            idempotency_key: wire.idempotency_key,
            claim: wire.claim,
            evidence_records: wire.evidence_records,
            state: wire.state,
            effective_disposition: wire.effective_disposition,
            metrics: wire.metrics,
            fingerprint: wire.fingerprint,
        };
        binding.validate().map_err(D::Error::custom)?;
        Ok(binding)
    }
}

impl PlanningEvidenceBinding {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    pub fn claim(&self) -> &Claim {
        &self.claim
    }

    pub fn evidence_records(&self) -> &[EvidenceRecord] {
        &self.evidence_records
    }

    pub fn state(&self) -> FactState {
        self.state
    }

    pub fn effective_disposition(&self) -> Option<crate::planning_reconciliation::Disposition> {
        self.effective_disposition
    }

    pub fn metrics(&self) -> &EvidenceBindingMetrics {
        &self.metrics
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn mitigation_allowed(&self) -> bool {
        self.state == FactState::Verified
            && self.effective_disposition
                == Some(crate::planning_reconciliation::Disposition::Mitigate)
    }

    pub fn validate(&self) -> Result<(), PlanningEvidenceBindingError> {
        if self.schema_version != PLANNING_EVIDENCE_BINDING_SCHEMA_VERSION {
            return Err(PlanningEvidenceBindingError::UnsupportedSchemaVersion);
        }
        if !bounded_text(&self.idempotency_key, MAX_BINDING_IDEMPOTENCY_KEY_LEN)
            || !valid_fingerprint(&self.fingerprint)
        {
            return Err(PlanningEvidenceBindingError::InvalidIdentity);
        }
        self.claim.validate()?;
        if self.claim.state() != self.state {
            return Err(PlanningEvidenceBindingError::ClaimEvidence(
                ClaimEvidenceError::InvalidShape,
            ));
        }
        if self.evidence_records.len() > MAX_EVIDENCE_RECORDS {
            return Err(PlanningEvidenceBindingError::BoundsExceeded);
        }

        let mut evidence_ids = BTreeSet::new();
        for record in &self.evidence_records {
            record.validate()?;
            if !evidence_ids.insert(record.evidence_id()) {
                return Err(PlanningEvidenceBindingError::ClaimEvidence(
                    ClaimEvidenceError::DuplicateEvidence,
                ));
            }
            if record.claim_id() != self.claim.claim_id() {
                return Err(PlanningEvidenceBindingError::ClaimEvidence(
                    ClaimEvidenceError::ClaimMismatch,
                ));
            }
            if record.scope() != self.claim.expected_identity() {
                return Err(PlanningEvidenceBindingError::ClaimEvidence(
                    ClaimEvidenceError::IdentityMismatch,
                ));
            }
        }

        let mut claim_evidence_ids = self.claim.evidence_ids().to_vec();
        claim_evidence_ids.sort();
        let record_evidence_ids = evidence_ids.into_iter().collect::<Vec<_>>();
        if claim_evidence_ids != record_evidence_ids {
            return Err(PlanningEvidenceBindingError::ClaimEvidence(
                ClaimEvidenceError::InvalidShape,
            ));
        }

        let expected_state = if self.evidence_records.is_empty() {
            FactState::NoProof
        } else {
            state_for(&self.evidence_records)
        };
        if self.state != expected_state {
            return Err(PlanningEvidenceBindingError::ClaimEvidence(
                ClaimEvidenceError::InvalidShape,
            ));
        }

        let expected_metrics = if self.state == FactState::NoProof {
            EvidenceBindingMetrics {
                no_proof: 1,
                ..EvidenceBindingMetrics::default()
            }
        } else {
            metrics_for(&self.evidence_records)
        };
        if self.metrics != expected_metrics {
            return Err(PlanningEvidenceBindingError::ClaimEvidence(
                ClaimEvidenceError::InvalidShape,
            ));
        }
        if self.state != FactState::Verified
            && self.effective_disposition
                == Some(crate::planning_reconciliation::Disposition::Mitigate)
        {
            return Err(PlanningEvidenceBindingError::ClaimEvidence(
                ClaimEvidenceError::InvalidShape,
            ));
        }

        let mut claim = self.claim.clone();
        let resolution = ClaimResolution::new(self.state, self.claim.evidence_ids().to_vec());
        claim.apply_resolution(resolution, &self.evidence_records)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanningEvidenceBindingOutcome {
    Bound(Box<PlanningEvidenceBinding>),
    Cancelled { fingerprint: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanningEvidenceBindingError {
    #[error("planning evidence binding schema version is unsupported")]
    UnsupportedSchemaVersion,
    #[error("planning evidence binding identity is invalid")]
    InvalidIdentity,
    #[error("planning evidence binding identity does not match")]
    IdentityMismatch,
    #[error("planning finding is invalid")]
    InvalidFinding(#[source] ReconciliationError),
    #[error("planning evidence binding input exceeds bounds")]
    BoundsExceeded,
    #[error("finding evidence reference has no matching resolver record")]
    MissingEvidenceReference,
    #[error("resolver evidence record is not referenced by the finding")]
    UnexpectedEvidence,
    #[error("finding and resolver evidence do not carry the same digest")]
    EvidenceReferenceMismatch,
    #[error("finding and resolver evidence statuses do not match")]
    EvidenceStatusMismatch,
    #[error("claim/evidence contract rejected binding: {0}")]
    ClaimEvidence(#[from] ClaimEvidenceError),
}

pub struct PlanningEvidenceAdapter;

impl PlanningEvidenceAdapter {
    pub fn bind(
        request: &PlanningEvidenceBindingRequest,
    ) -> Result<PlanningEvidenceBindingOutcome, PlanningEvidenceBindingError> {
        request.validate()?;
        let request_fingerprint = request_fingerprint(request);
        if request.cancelled {
            return Ok(PlanningEvidenceBindingOutcome::Cancelled {
                fingerprint: request_fingerprint,
            });
        }

        let mut claim = Claim::new(
            request.project_id,
            request.run_id,
            request.trace_id,
            request.finding.finding_id.clone(),
            ClaimClass::PlanFinding,
            request.finding.claim_digest.clone(),
            request.required_evidence.clone(),
            request.expected_identity.clone(),
        )?;

        let mapped = map_finding_evidence(request)?;
        let resolution =
            resolution_for(mapped.state, mapped.evidence_ids).with_reason(mapped.reason)?;
        let _outcome = claim.apply_resolution(resolution, &mapped.records)?;
        let effective_disposition = effective_disposition(&request.finding, mapped.state);
        let fingerprint = binding_fingerprint(
            request,
            mapped.state,
            &mapped.records,
            effective_disposition,
        );

        Ok(PlanningEvidenceBindingOutcome::Bound(Box::new(
            PlanningEvidenceBinding {
                schema_version: PLANNING_EVIDENCE_BINDING_SCHEMA_VERSION,
                idempotency_key: request.idempotency_key.clone(),
                claim,
                evidence_records: mapped.records,
                state: mapped.state,
                effective_disposition,
                metrics: mapped.metrics,
                fingerprint,
            },
        )))
    }
}

struct MappedEvidence {
    records: Vec<EvidenceRecord>,
    state: FactState,
    evidence_ids: Vec<String>,
    metrics: EvidenceBindingMetrics,
    reason: &'static str,
}

fn map_finding_evidence(
    request: &PlanningEvidenceBindingRequest,
) -> Result<MappedEvidence, PlanningEvidenceBindingError> {
    let refs = &request.finding.evidence;
    if refs.is_empty()
        || refs
            .iter()
            .any(|item| item.status == PlanningEvidenceStatus::Missing)
    {
        if !request.evidence_records.is_empty() {
            return Err(PlanningEvidenceBindingError::UnexpectedEvidence);
        }
        return Ok(MappedEvidence {
            records: Vec::new(),
            state: FactState::NoProof,
            evidence_ids: Vec::new(),
            metrics: EvidenceBindingMetrics {
                no_proof: 1,
                ..EvidenceBindingMetrics::default()
            },
            reason: if refs.is_empty() {
                "finding has no resolver evidence"
            } else {
                "finding includes missing resolver evidence"
            },
        });
    }

    let mut reference_ids = BTreeSet::<String>::new();
    for evidence_ref in refs {
        if !reference_ids.insert(evidence_ref.evidence_id.clone()) {
            return Err(PlanningEvidenceBindingError::ClaimEvidence(
                ClaimEvidenceError::DuplicateEvidence,
            ));
        }
    }
    if request
        .evidence_records
        .iter()
        .any(|record| !reference_ids.contains(record.evidence_id()))
    {
        return Err(PlanningEvidenceBindingError::UnexpectedEvidence);
    }

    let mut records = Vec::with_capacity(refs.len());
    for evidence_ref in refs {
        let record = request
            .evidence_records
            .iter()
            .find(|item| item.evidence_id() == evidence_ref.evidence_id)
            .ok_or(PlanningEvidenceBindingError::MissingEvidenceReference)?;
        if record.evidence_digest() != evidence_ref.digest {
            return Err(PlanningEvidenceBindingError::EvidenceReferenceMismatch);
        }
        if record.status() != claim_status(evidence_ref.status)? {
            return Err(PlanningEvidenceBindingError::EvidenceStatusMismatch);
        }
        records.push(record.clone());
    }
    records.sort_by(|left, right| left.evidence_id().cmp(right.evidence_id()));

    let mut evidence_ids = records
        .iter()
        .map(|record| record.evidence_id().to_owned())
        .collect::<Vec<_>>();
    evidence_ids.sort();
    let state = state_for(&records);
    let metrics = metrics_for(&records);
    let reason = match state {
        FactState::Verified => "all referenced resolver evidence is verified",
        FactState::Unverified => "resolver evidence is insufficient",
        FactState::Stale => "resolver evidence is stale",
        FactState::Conflicting => "resolver evidence conflicts",
        FactState::NoProof => unreachable!(),
    };
    Ok(MappedEvidence {
        records,
        state,
        evidence_ids,
        metrics,
        reason,
    })
}

fn resolution_for(state: FactState, evidence_ids: Vec<String>) -> ClaimResolution {
    match state {
        FactState::Verified => ClaimResolution::verified(evidence_ids),
        FactState::Unverified => ClaimResolution::unverified(evidence_ids),
        FactState::Stale => ClaimResolution::stale(evidence_ids),
        FactState::Conflicting => ClaimResolution::conflicting(evidence_ids),
        FactState::NoProof => ClaimResolution::no_proof(),
    }
}

fn claim_status(
    status: PlanningEvidenceStatus,
) -> Result<ClaimEvidenceStatus, PlanningEvidenceBindingError> {
    match status {
        PlanningEvidenceStatus::Verified => Ok(ClaimEvidenceStatus::Verified),
        PlanningEvidenceStatus::Unverified => Ok(ClaimEvidenceStatus::Unverified),
        PlanningEvidenceStatus::Conflicting => Ok(ClaimEvidenceStatus::Conflicting),
        PlanningEvidenceStatus::Stale => Ok(ClaimEvidenceStatus::Stale),
        PlanningEvidenceStatus::Missing => Err(PlanningEvidenceBindingError::InvalidFinding(
            ReconciliationError::InvalidEvidence,
        )),
    }
}

fn state_for(records: &[EvidenceRecord]) -> FactState {
    if records
        .iter()
        .any(|record| record.status() == ClaimEvidenceStatus::Conflicting)
    {
        FactState::Conflicting
    } else if records
        .iter()
        .any(|record| record.status() == ClaimEvidenceStatus::Stale)
    {
        FactState::Stale
    } else if records.iter().any(|record| {
        matches!(
            record.status(),
            ClaimEvidenceStatus::Unverified | ClaimEvidenceStatus::NoProof
        )
    }) {
        FactState::Unverified
    } else {
        FactState::Verified
    }
}

fn metrics_for(records: &[EvidenceRecord]) -> EvidenceBindingMetrics {
    let mut metrics = EvidenceBindingMetrics {
        total_records: records.len(),
        ..EvidenceBindingMetrics::default()
    };
    for record in records {
        match record.status() {
            ClaimEvidenceStatus::Verified => metrics.verified += 1,
            ClaimEvidenceStatus::Unverified => metrics.unverified += 1,
            ClaimEvidenceStatus::Stale => metrics.stale += 1,
            ClaimEvidenceStatus::Conflicting => metrics.conflicting += 1,
            ClaimEvidenceStatus::NoProof => metrics.no_proof += 1,
        }
    }
    metrics
}

fn effective_disposition(
    finding: &ReviewerFinding,
    state: FactState,
) -> Option<crate::planning_reconciliation::Disposition> {
    if finding.suggested_disposition == Some(crate::planning_reconciliation::Disposition::Mitigate)
        && state != FactState::Verified
    {
        Some(crate::planning_reconciliation::Disposition::HumanRequired)
    } else {
        finding.suggested_disposition
    }
}

fn validate_required_evidence(
    required_evidence: &[ClaimEvidenceKind],
) -> Result<(), PlanningEvidenceBindingError> {
    if required_evidence.is_empty() || required_evidence.len() > MAX_REQUIRED_EVIDENCE {
        return Err(PlanningEvidenceBindingError::BoundsExceeded);
    }
    let mut seen = BTreeSet::new();
    if required_evidence.iter().any(|kind| !seen.insert(*kind)) {
        return Err(PlanningEvidenceBindingError::ClaimEvidence(
            ClaimEvidenceError::DuplicateEvidence,
        ));
    }
    Ok(())
}

fn bounded_text(value: &str, max_len: usize) -> bool {
    !value.is_empty() && value.len() <= max_len && !value.chars().any(char::is_control)
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 16 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn request_fingerprint(request: &PlanningEvidenceBindingRequest) -> String {
    let mut value = format!(
        "{}|{}|{}|{}|{}|{}|{}|{:?}",
        request.idempotency_key,
        request.project_id,
        request.run_id,
        request.trace_id,
        request.finding.finding_id,
        request.finding.claim_digest,
        request.expected_identity.identity_digest(),
        request.required_evidence,
    );
    for evidence_ref in &request.finding.evidence {
        value.push_str(&format!(
            "|ref:{}:{}:{:?}",
            evidence_ref.evidence_id, evidence_ref.digest, evidence_ref.status
        ));
    }
    stable_digest(&value)
}

fn binding_fingerprint(
    request: &PlanningEvidenceBindingRequest,
    state: FactState,
    records: &[EvidenceRecord],
    disposition: Option<crate::planning_reconciliation::Disposition>,
) -> String {
    let mut value = format!(
        "{}|{:?}|{:?}",
        request_fingerprint(request),
        state,
        disposition
    );
    for record in records {
        value.push_str(&format!(
            "|record:{}:{}:{:?}:{:?}:{:?}",
            record.evidence_id(),
            record.evidence_digest(),
            record.kind(),
            record.status(),
            record.scope(),
        ));
    }
    stable_digest(&value)
}

fn stable_digest(value: &str) -> String {
    let mut state = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}

impl From<ReconciliationError> for PlanningEvidenceBindingError {
    fn from(error: ReconciliationError) -> Self {
        Self::InvalidFinding(error)
    }
}
