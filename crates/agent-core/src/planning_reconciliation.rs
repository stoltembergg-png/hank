//! Bounded, deterministic planning reconciliation.
//!
//! This module is a pure contract used by the Application layer. It merges
//! reviewer observations into an auditable [`FinalPlan`] without executing a
//! plan, granting capabilities, or treating reviewer text as authority.

use crate::{ProjectId, RunId, TraceId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const PLANNING_RECONCILIATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_RECONCILIATION_ROUNDS: u8 = 2;
pub const MAX_RECONCILIATION_FINDINGS: usize = 128;
pub const MAX_RECONCILIATION_EVIDENCE: usize = 8;
pub const MAX_RECONCILIATION_TEXT: usize = 512;
pub const MAX_RECONCILIATION_REVIEWERS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerKind {
    Architecture,
    Security,
    Test,
    Simplicity,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Accept,
    Reject,
    Mitigate,
    Defer,
    Split,
    HumanRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    PolicyProduct,
    ReviewerDisagreement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Verified,
    Unverified,
    Missing,
    Stale,
}

impl EvidenceStatus {
    fn backs_disposition(self) -> bool {
        self == Self::Verified
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub evidence_id: String,
    pub digest: String,
    pub status: EvidenceStatus,
}

impl EvidenceRef {
    pub fn new(
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
        evidence_id: impl Into<String>,
        digest: impl Into<String>,
        status: EvidenceStatus,
    ) -> Result<Self, ReconciliationError> {
        let evidence = Self {
            project_id,
            run_id,
            trace_id,
            evidence_id: evidence_id.into(),
            digest: digest.into(),
            status,
        };
        evidence.validate_shape()?;
        Ok(evidence)
    }

    fn validate_shape(&self) -> Result<(), ReconciliationError> {
        if !bounded_text(&self.evidence_id)
            || self.digest.len() > MAX_RECONCILIATION_TEXT
            || (!self.digest.is_empty() && !valid_digest(&self.digest))
            || (self.status.backs_disposition() && !valid_digest(&self.digest))
        {
            return Err(ReconciliationError::InvalidEvidence);
        }
        Ok(())
    }

    fn validate_scope(
        &self,
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
    ) -> Result<(), ReconciliationError> {
        self.validate_shape()?;
        if self.project_id != project_id || self.run_id != run_id || self.trace_id != trace_id {
            return Err(ReconciliationError::IdentityMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewerFinding {
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub finding_id: String,
    pub reviewer_id: String,
    pub reviewer_kind: ReviewerKind,
    pub reviewer_version: String,
    pub severity: FindingSeverity,
    pub category: String,
    pub affected_contract: String,
    pub consequence: String,
    pub claim_digest: String,
    pub evidence: Vec<EvidenceRef>,
    pub suggested_disposition: Option<Disposition>,
    pub rationale: String,
    pub conflict: Option<ConflictKind>,
}

impl ReviewerFinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
        finding_id: impl Into<String>,
        reviewer_id: impl Into<String>,
        reviewer_kind: ReviewerKind,
        severity: FindingSeverity,
        category: impl Into<String>,
        affected_contract: impl Into<String>,
        consequence: impl Into<String>,
        claim_digest: impl Into<String>,
        rationale: impl Into<String>,
    ) -> Result<Self, ReconciliationError> {
        let finding = Self {
            project_id,
            run_id,
            trace_id,
            finding_id: finding_id.into(),
            reviewer_id: reviewer_id.into(),
            reviewer_kind,
            reviewer_version: "reviewer-v1".into(),
            severity,
            category: category.into(),
            affected_contract: affected_contract.into(),
            consequence: consequence.into(),
            claim_digest: claim_digest.into(),
            evidence: Vec::new(),
            suggested_disposition: None,
            rationale: rationale.into(),
            conflict: None,
        };
        finding.validate_shape()?;
        Ok(finding)
    }

    pub fn with_evidence(mut self, evidence: Vec<EvidenceRef>) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn with_disposition(mut self, disposition: Option<Disposition>) -> Self {
        self.suggested_disposition = disposition;
        self
    }

    pub fn with_conflict(mut self, conflict: Option<ConflictKind>) -> Self {
        self.conflict = conflict;
        self
    }

    fn validate_shape(&self) -> Result<(), ReconciliationError> {
        if !bounded_text(&self.finding_id)
            || !bounded_text(&self.reviewer_id)
            || !bounded_text(&self.reviewer_version)
            || !bounded_text(&self.category)
            || !bounded_text(&self.affected_contract)
            || !bounded_text(&self.consequence)
            || !bounded_text(&self.rationale)
            || [
                &self.category,
                &self.affected_contract,
                &self.consequence,
                &self.rationale,
            ]
            .iter()
            .any(|value| contains_sensitive_marker(value))
            || !valid_digest(&self.claim_digest)
            || self.evidence.len() > MAX_RECONCILIATION_EVIDENCE
        {
            return Err(ReconciliationError::InvalidFinding);
        }
        let mut evidence_ids = BTreeSet::new();
        for evidence in &self.evidence {
            evidence.validate_scope(self.project_id, self.run_id, self.trace_id)?;
            if !evidence_ids.insert((&evidence.evidence_id, &evidence.digest)) {
                return Err(ReconciliationError::DuplicateEvidence);
            }
        }
        Ok(())
    }

    fn validate_for(
        &self,
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
        planner_id: &str,
        judge_id: &str,
    ) -> Result<(), ReconciliationError> {
        self.validate_shape()?;
        if self.project_id != project_id || self.run_id != run_id || self.trace_id != trace_id {
            return Err(ReconciliationError::IdentityMismatch);
        }
        if self.reviewer_id == planner_id || self.reviewer_id == judge_id {
            return Err(ReconciliationError::SelfApproval);
        }
        Ok(())
    }

    fn canonical_key(&self) -> String {
        let mut evidence = self
            .evidence
            .iter()
            .map(|item| format!("{}:{}", item.evidence_id, item.digest))
            .collect::<Vec<_>>();
        evidence.sort();
        stable_digest(&format!(
            "{}|{}|{}",
            normalize(&self.affected_contract),
            normalize(&self.consequence),
            evidence.join("|")
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationRequest {
    pub schema_version: u32,
    pub idempotency_key: String,
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub plan_digest: String,
    pub policy_revision: String,
    pub planner_id: String,
    pub judge_id: String,
    pub round: u8,
    pub cancelled: bool,
    pub findings: Vec<ReviewerFinding>,
}

impl ReconciliationRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        run_id: RunId,
        trace_id: TraceId,
        idempotency_key: impl Into<String>,
        plan_digest: impl Into<String>,
        policy_revision: impl Into<String>,
        planner_id: impl Into<String>,
        judge_id: impl Into<String>,
        round: u8,
        findings: Vec<ReviewerFinding>,
    ) -> Result<Self, ReconciliationError> {
        let request = Self {
            schema_version: PLANNING_RECONCILIATION_SCHEMA_VERSION,
            idempotency_key: idempotency_key.into(),
            project_id,
            run_id,
            trace_id,
            plan_digest: plan_digest.into(),
            policy_revision: policy_revision.into(),
            planner_id: planner_id.into(),
            judge_id: judge_id.into(),
            round,
            cancelled: false,
            findings,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_cancelled(mut self, cancelled: bool) -> Self {
        self.cancelled = cancelled;
        self
    }

    pub fn validate(&self) -> Result<(), ReconciliationError> {
        if self.round > MAX_RECONCILIATION_ROUNDS {
            return Err(ReconciliationError::RoundOverflow);
        }
        if self.schema_version != PLANNING_RECONCILIATION_SCHEMA_VERSION {
            return Err(ReconciliationError::SchemaMismatch);
        }
        if self.planner_id == self.judge_id {
            return Err(ReconciliationError::SelfApproval);
        }
        if !bounded_text(&self.idempotency_key)
            || !valid_digest(&self.plan_digest)
            || !bounded_text(&self.policy_revision)
            || !bounded_text(&self.planner_id)
            || !bounded_text(&self.judge_id)
            || self.round == 0
        {
            return Err(ReconciliationError::InvalidIdentity);
        }
        if self.findings.len() > MAX_RECONCILIATION_FINDINGS {
            return Err(ReconciliationError::BoundsExceeded);
        }

        let mut finding_ids = BTreeSet::new();
        let mut reviewer_ids = BTreeSet::new();
        for finding in &self.findings {
            finding.validate_for(
                self.project_id,
                self.run_id,
                self.trace_id,
                &self.planner_id,
                &self.judge_id,
            )?;
            if !finding_ids.insert(&finding.finding_id) {
                return Err(ReconciliationError::DuplicateFinding);
            }
            reviewer_ids.insert(&finding.reviewer_id);
        }
        if reviewer_ids.len() > MAX_RECONCILIATION_REVIEWERS {
            return Err(ReconciliationError::BoundsExceeded);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionDecision {
    pub finding_ids: Vec<String>,
    pub canonical_key: String,
    pub severity: FindingSeverity,
    pub disposition: Disposition,
    pub rationale: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub reviewer_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disagreement {
    pub kind: ConflictKind,
    pub finding_ids: Vec<String>,
    pub dispositions: Vec<Disposition>,
    pub rationale: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationMetrics {
    pub input_findings: usize,
    pub deduplicated_findings: usize,
    pub decisions: usize,
    pub disagreements: usize,
    pub human_required: usize,
    pub reviewer_disagreements: usize,
    pub policy_product_conflicts: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub mitigated: usize,
    pub deferred: usize,
    pub split: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalPlanStatus {
    Ready,
    HumanRequired,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalPlan {
    pub schema_version: u32,
    pub idempotency_key: String,
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub plan_digest: String,
    pub policy_revision: String,
    pub planner_id: String,
    pub judge_id: String,
    pub round: u8,
    pub status: FinalPlanStatus,
    pub findings: Vec<ReviewerFinding>,
    pub decisions: Vec<DispositionDecision>,
    pub disagreements: Vec<Disagreement>,
    pub metrics: ReconciliationMetrics,
    pub fingerprint: String,
}

impl FinalPlan {
    pub fn decision_for(&self, finding_id: &str) -> Option<&DispositionDecision> {
        self.decisions
            .iter()
            .find(|decision| decision.finding_ids.iter().any(|id| id == finding_id))
    }

    /// A reconciled plan is data only; execution requires a separate
    /// application command and an independent capability/policy decision.
    pub fn can_execute(&self) -> bool {
        false
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn can_merge(&self) -> bool {
        false
    }

    /// Reopen creates a new mutable draft and never mutates this immutable
    /// artifact. The caller must submit the draft through the normal path.
    pub fn reopen(&self) -> Result<ReconciliationDraft, ReconciliationError> {
        Ok(ReconciliationDraft {
            schema_version: self.schema_version,
            idempotency_key: self.idempotency_key.clone(),
            project_id: self.project_id,
            run_id: self.run_id,
            trace_id: self.trace_id,
            plan_digest: self.plan_digest.clone(),
            policy_revision: self.policy_revision.clone(),
            planner_id: self.planner_id.clone(),
            judge_id: self.judge_id.clone(),
            round: self.round,
            findings: self.findings.clone(),
            reopened_from: Some(self.fingerprint.clone()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationDraft {
    pub schema_version: u32,
    pub idempotency_key: String,
    pub project_id: ProjectId,
    pub run_id: RunId,
    pub trace_id: TraceId,
    pub plan_digest: String,
    pub policy_revision: String,
    pub planner_id: String,
    pub judge_id: String,
    pub round: u8,
    pub findings: Vec<ReviewerFinding>,
    pub reopened_from: Option<String>,
}

impl ReconciliationDraft {
    /// Re-enters the normal reconciliation path after a rollback/reopen.
    pub fn into_request(self) -> Result<ReconciliationRequest, ReconciliationError> {
        ReconciliationRequest::new(
            self.project_id,
            self.run_id,
            self.trace_id,
            self.idempotency_key,
            self.plan_digest,
            self.policy_revision,
            self.planner_id,
            self.judge_id,
            self.round,
            self.findings,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationOutcome {
    FinalPlan(Box<FinalPlan>),
    Cancelled { fingerprint: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReconciliationError {
    #[error("reconciliation schema is unsupported")]
    SchemaMismatch,
    #[error("reconciliation identity is invalid")]
    InvalidIdentity,
    #[error("reconciliation finding is invalid")]
    InvalidFinding,
    #[error("reconciliation evidence is invalid")]
    InvalidEvidence,
    #[error("reconciliation evidence is duplicated")]
    DuplicateEvidence,
    #[error("reconciliation finding is duplicated")]
    DuplicateFinding,
    #[error("reconciliation input exceeds bounds")]
    BoundsExceeded,
    #[error("planner, reviewer and judge identities cannot self-approve")]
    SelfApproval,
    #[error("reconciliation round exceeds the approved maximum")]
    RoundOverflow,
    #[error("reconciliation identity does not match its parent scope")]
    IdentityMismatch,
}

pub struct PlanningReconciliation;

impl PlanningReconciliation {
    pub fn reconcile(
        request: &ReconciliationRequest,
    ) -> Result<ReconciliationOutcome, ReconciliationError> {
        request.validate()?;
        let request_fingerprint = request_fingerprint(request);
        if request.cancelled {
            return Ok(ReconciliationOutcome::Cancelled {
                fingerprint: request_fingerprint,
            });
        }

        let mut groups: BTreeMap<String, Vec<ReviewerFinding>> = BTreeMap::new();
        for finding in &request.findings {
            groups
                .entry(finding.canonical_key())
                .or_default()
                .push(finding.clone());
        }

        let mut findings = request.findings.clone();
        findings.sort_by(|left, right| {
            left.canonical_key()
                .cmp(&right.canonical_key())
                .then_with(|| left.finding_id.cmp(&right.finding_id))
        });

        let mut decisions = Vec::with_capacity(groups.len());
        let mut disagreements = Vec::new();
        let mut metrics = ReconciliationMetrics {
            input_findings: findings.len(),
            decisions: groups.len(),
            deduplicated_findings: findings.len().saturating_sub(groups.len()),
            ..ReconciliationMetrics::default()
        };

        for (canonical_key, mut group) in groups {
            group.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
            let finding_ids = group
                .iter()
                .map(|finding| finding.finding_id.clone())
                .collect::<Vec<_>>();
            let severity = group
                .iter()
                .map(|finding| finding.severity)
                .max()
                .unwrap_or(FindingSeverity::Info);
            let reviewer_ids = group
                .iter()
                .map(|finding| finding.reviewer_id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let evidence_refs = merge_evidence(&group);
            let proposals = group
                .iter()
                .filter_map(|finding| finding.suggested_disposition)
                .collect::<BTreeSet<_>>();
            let proposal_values = proposals.iter().copied().collect::<Vec<_>>();
            let policy_product_conflict = group
                .iter()
                .any(|finding| finding.conflict == Some(ConflictKind::PolicyProduct));
            let reviewer_disagreement = proposals.len() > 1;
            let has_verified_evidence = evidence_refs
                .iter()
                .any(|evidence| evidence.status.backs_disposition());

            if policy_product_conflict {
                disagreements.push(Disagreement {
                    kind: ConflictKind::PolicyProduct,
                    finding_ids: finding_ids.clone(),
                    dispositions: proposal_values.clone(),
                    rationale: "unresolved policy/product conflict requires human decision".into(),
                });
                metrics.policy_product_conflicts += 1;
            }
            if reviewer_disagreement {
                disagreements.push(Disagreement {
                    kind: ConflictKind::ReviewerDisagreement,
                    finding_ids: finding_ids.clone(),
                    dispositions: proposal_values.clone(),
                    rationale: "reviewer dispositions disagree and require human decision".into(),
                });
                metrics.reviewer_disagreements += 1;
            }

            let mut disposition = proposal_values
                .first()
                .copied()
                .unwrap_or(Disposition::Defer);
            let rationale = if policy_product_conflict {
                disposition = Disposition::HumanRequired;
                "unresolved policy/product conflict requires human decision"
            } else if reviewer_disagreement {
                disposition = Disposition::HumanRequired;
                "reviewer dispositions disagree and require human decision"
            } else if matches!(severity, FindingSeverity::High | FindingSeverity::Critical)
                && !has_verified_evidence
            {
                disposition = Disposition::HumanRequired;
                "high or critical finding lacks verified evidence"
            } else if proposal_values.is_empty() {
                "no reviewer disposition supplied; finding remains deferred"
            } else {
                "disposition is supported by the reconciled reviewer record"
            };

            if disposition == Disposition::HumanRequired {
                metrics.human_required += group.len();
            }
            match disposition {
                Disposition::Accept => metrics.accepted += group.len(),
                Disposition::Reject => metrics.rejected += group.len(),
                Disposition::Mitigate => metrics.mitigated += group.len(),
                Disposition::Defer => metrics.deferred += group.len(),
                Disposition::Split => metrics.split += group.len(),
                Disposition::HumanRequired => {}
            }
            decisions.push(DispositionDecision {
                finding_ids,
                canonical_key,
                severity,
                disposition,
                rationale: rationale.into(),
                evidence_refs,
                reviewer_ids,
            });
        }

        metrics.disagreements = disagreements.len();
        let status = if metrics.human_required > 0 {
            FinalPlanStatus::HumanRequired
        } else {
            FinalPlanStatus::Ready
        };
        let fingerprint = stable_digest(&format!(
            "{}|{:?}|{:?}|{:?}|{:?}",
            request_fingerprint, status, findings, decisions, disagreements
        ));
        Ok(ReconciliationOutcome::FinalPlan(Box::new(FinalPlan {
            schema_version: PLANNING_RECONCILIATION_SCHEMA_VERSION,
            idempotency_key: request.idempotency_key.clone(),
            project_id: request.project_id,
            run_id: request.run_id,
            trace_id: request.trace_id,
            plan_digest: request.plan_digest.clone(),
            policy_revision: request.policy_revision.clone(),
            planner_id: request.planner_id.clone(),
            judge_id: request.judge_id.clone(),
            round: request.round,
            status,
            findings,
            decisions,
            disagreements,
            metrics,
            fingerprint,
        })))
    }
}

fn merge_evidence(group: &[ReviewerFinding]) -> Vec<EvidenceRef> {
    let mut values = BTreeMap::new();
    for finding in group {
        for evidence in &finding.evidence {
            values
                .entry((evidence.evidence_id.clone(), evidence.digest.clone()))
                .or_insert_with(|| evidence.clone());
        }
    }
    values.into_values().collect()
}

fn request_fingerprint(request: &ReconciliationRequest) -> String {
    let mut findings = request.findings.clone();
    findings.sort_by(|left, right| {
        left.canonical_key()
            .cmp(&right.canonical_key())
            .then_with(|| left.finding_id.cmp(&right.finding_id))
    });
    stable_digest(&format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}",
        request.idempotency_key,
        request.project_id,
        request.run_id,
        request.trace_id,
        request.plan_digest,
        request.policy_revision,
        request.planner_id,
        request.judge_id,
        request.round,
        findings
    ))
}

fn bounded_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_RECONCILIATION_TEXT
        && !value.chars().any(char::is_control)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn stable_digest(value: &str) -> String {
    let mut state = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}

fn contains_sensitive_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "authorization:",
        "bearer ",
        "api_key=",
        "api-key=",
        "password=",
        "private key",
        "secret=",
        "token=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}
