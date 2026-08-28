//! Provenance-bound, non-activating Skill candidate generation.
//!
//! This boundary accepts only bounded proposal data and observation references.
//! It parses the proposal as untrusted data, emits a project-scoped draft or a
//! quarantine decision, and returns only hashes in the evaluator handoff. It
//! never persists, activates, promotes, executes, or mutates a Skill.

use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, DomainError, ParsedSkill, ProjectId, Resource,
    SkillFileInput, SkillFileRole, SkillParseRequest, SkillParser, SkillScope,
    DEFAULT_MAX_DOCUMENT_BYTES,
};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SKILL_CANDIDATE_GENERATE_CAPABILITY: &str = "skill:candidate.generate";
pub const SKILL_CANDIDATE_SCHEMA_VERSION: u32 = 1;

const MAX_AGENT_ID_BYTES: usize = 128;
const MAX_BASE_VERSION_BYTES: usize = 64;
const MAX_OBSERVATIONS: u16 = 32;
const MAX_OBSERVATION_ID_BYTES: usize = 128;
const MAX_OBSERVATION_SOURCE_BYTES: usize = 64;
const MAX_REASONS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCandidateStatus {
    Draft,
    Quarantined,
    Discarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCandidateReason {
    InstructionOverride,
    CapabilityEscalation,
    ScopeMismatch,
    TraceMismatch,
    ArtifactPoisoning,
    SensitiveContent,
    EvidenceRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillObservationRef {
    pub observation_id: String,
    pub digest: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCandidateProposal {
    pub document: String,
    pub files: Vec<SkillFileInput>,
}

#[derive(Debug, Clone)]
pub struct SkillCandidatePolicy {
    pub allow: bool,
    pub allowed_capabilities: CapabilitySet,
    pub max_observations: u16,
    pub max_document_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct SkillCandidateRequest {
    pub project_id: ProjectId,
    pub agent_id: String,
    pub capability: Capability,
    pub policy: SkillCandidatePolicy,
    pub budget: BudgetLimits,
    pub trace_id: TraceId,
    pub base_version: String,
    pub observations: Vec<SkillObservationRef>,
    pub proposal: SkillCandidateProposal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillEvaluationHandoff {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub candidate_id: String,
    pub trace_id: TraceId,
    pub capability: String,
    pub status: SkillCandidateStatus,
    pub candidate_digest: String,
    pub source_digest: String,
    pub policy_digest: String,
    pub budget_digest: String,
    pub rollback_version: Option<String>,
    pub report_digest: String,
}

#[derive(Debug, Clone)]
pub struct SkillCandidate {
    pub candidate_id: String,
    pub project_id: ProjectId,
    pub agent_id: String,
    pub trace_id: TraceId,
    pub base_version: String,
    pub status: SkillCandidateStatus,
    pub reasons: Vec<SkillCandidateReason>,
    pub observations: Vec<SkillObservationRef>,
    pub parsed: ParsedSkill,
    pub candidate_digest: String,
    pub handoff: SkillEvaluationHandoff,
}

impl PartialEq for SkillCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.candidate_id == other.candidate_id
            && self.project_id == other.project_id
            && self.agent_id == other.agent_id
            && self.trace_id == other.trace_id
            && self.base_version == other.base_version
            && self.status == other.status
            && self.reasons == other.reasons
            && self.observations == other.observations
            && self.candidate_digest == other.candidate_digest
            && self.handoff == other.handoff
    }
}

impl Eq for SkillCandidate {}

impl SkillCandidate {
    /// Discard is an in-memory terminal transition. It preserves the proposal
    /// digest and rollback metadata and is idempotent for an already discarded
    /// candidate.
    pub fn discard(&mut self) -> Result<(), DomainError> {
        match self.status {
            SkillCandidateStatus::Draft | SkillCandidateStatus::Discarded => {
                self.status = SkillCandidateStatus::Discarded;
                self.handoff.status = SkillCandidateStatus::Discarded;
                self.handoff.report_digest = handoff_digest(&self.handoff);
                Ok(())
            }
            SkillCandidateStatus::Quarantined => Err(DomainError::InvalidStateTransition {
                from: "quarantined".into(),
                to: "discarded".into(),
            }),
        }
    }
}

pub struct SkillCandidateGenerationService;

impl SkillCandidateGenerationService {
    pub fn generate(request: SkillCandidateRequest) -> Result<SkillCandidate, DomainError> {
        validate_request(&request)?;
        let observations = normalize_observations(&request.observations)?;
        let proposal = request.proposal.clone();
        let parsed = SkillParser::default()
            .parse(SkillParseRequest {
                document: proposal.document,
                files: proposal.files,
                project_id: Some(request.project_id),
            })
            .map_err(|_| DomainError::Validation("skill candidate proposal rejected".into()))?;

        let candidate_digest = candidate_digest(&parsed);
        let source_digest = digest_json(&observations);
        let policy_digest = policy_digest(&request.policy);
        let budget_digest = digest_json(&request.budget);
        let candidate_id = format!(
            "candidate-{}",
            digest_json(&(
                request.project_id,
                &request.agent_id,
                request.trace_id,
                &request.base_version,
                &candidate_digest,
                &source_digest,
            ))
        );
        let (status, reasons) = classify(&request, &parsed);
        let status = status.unwrap_or(SkillCandidateStatus::Draft);
        let mut handoff = SkillEvaluationHandoff {
            schema_version: SKILL_CANDIDATE_SCHEMA_VERSION,
            project_id: request.project_id,
            candidate_id: candidate_id.clone(),
            trace_id: request.trace_id,
            capability: SKILL_CANDIDATE_GENERATE_CAPABILITY.into(),
            status: status.clone(),
            candidate_digest: candidate_digest.clone(),
            source_digest,
            policy_digest,
            budget_digest,
            rollback_version: Some(request.base_version.clone()),
            report_digest: String::new(),
        };
        handoff.report_digest = handoff_digest(&handoff);

        Ok(SkillCandidate {
            candidate_id,
            project_id: request.project_id,
            agent_id: request.agent_id,
            trace_id: request.trace_id,
            base_version: request.base_version,
            status,
            reasons,
            observations,
            parsed,
            candidate_digest,
            handoff,
        })
    }
}

fn validate_request(request: &SkillCandidateRequest) -> Result<(), DomainError> {
    if request.agent_id.trim().is_empty() || request.agent_id.len() > MAX_AGENT_ID_BYTES {
        return Err(DomainError::Validation("candidate agent is invalid".into()));
    }
    if request.base_version.trim().is_empty()
        || request.base_version.len() > MAX_BASE_VERSION_BYTES
        || semver::Version::parse(&request.base_version).is_err()
    {
        return Err(DomainError::Validation(
            "candidate base version is invalid".into(),
        ));
    }
    if request.trace_id.as_uuid().is_nil() {
        return Err(DomainError::Validation(
            "candidate trace is required".into(),
        ));
    }
    let expected =
        Capability::new(Resource::Skill, Action::Create).with_scope(request.project_id.to_string());
    if request.capability != expected
        || !request.policy.allow
        || !request.policy.allowed_capabilities.contains(&expected)
    {
        return Err(DomainError::PermissionDenied {
            capability: SKILL_CANDIDATE_GENERATE_CAPABILITY.into(),
            reason: "candidate generation capability is not authorized".into(),
        });
    }
    if request.policy.max_observations == 0
        || request.policy.max_observations > MAX_OBSERVATIONS
        || request.policy.max_document_bytes == 0
        || request.policy.max_document_bytes > DEFAULT_MAX_DOCUMENT_BYTES
    {
        return Err(DomainError::BudgetExceeded {
            budget_type: "skill_candidate_limits".into(),
            limit: format!(
                "observations<= {MAX_OBSERVATIONS}, document<= {DEFAULT_MAX_DOCUMENT_BYTES}"
            ),
            used: format!(
                "observations={}, document={}",
                request.observations.len(),
                request.proposal.document.len()
            ),
        });
    }
    if request.observations.len() > request.policy.max_observations as usize {
        return Err(DomainError::BudgetExceeded {
            budget_type: "skill_candidate_observations".into(),
            limit: request.policy.max_observations.to_string(),
            used: request.observations.len().to_string(),
        });
    }
    if request.proposal.document.len() > request.policy.max_document_bytes {
        return Err(DomainError::BudgetExceeded {
            budget_type: "skill_candidate_document_bytes".into(),
            limit: request.policy.max_document_bytes.to_string(),
            used: request.proposal.document.len().to_string(),
        });
    }
    request.budget.validate()
}

fn normalize_observations(
    observations: &[SkillObservationRef],
) -> Result<Vec<SkillObservationRef>, DomainError> {
    let mut normalized = observations.to_vec();
    normalized.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    let mut deduped: Vec<SkillObservationRef> = Vec::with_capacity(normalized.len());
    for observation in normalized {
        if observation.observation_id.trim().is_empty()
            || observation.observation_id.len() > MAX_OBSERVATION_ID_BYTES
            || observation.observation_id.contains("..")
            || observation.observation_id.chars().any(char::is_control)
            || observation.source.trim().is_empty()
            || observation.source.len() > MAX_OBSERVATION_SOURCE_BYTES
            || observation.source.chars().any(char::is_control)
            || !is_digest(&observation.digest)
        {
            return Err(DomainError::Validation(
                "candidate observation is invalid".into(),
            ));
        }
        if let Some(previous) = deduped.last() {
            if previous.observation_id == observation.observation_id {
                if previous != &observation {
                    return Err(DomainError::Duplicate(
                        "candidate observation identity has conflicting evidence".into(),
                    ));
                }
                continue;
            }
        }
        deduped.push(observation);
    }
    if deduped.is_empty() {
        return Err(DomainError::Validation(
            "candidate provenance requires an observation".into(),
        ));
    }
    Ok(deduped)
}

fn classify(
    request: &SkillCandidateRequest,
    parsed: &ParsedSkill,
) -> (Option<SkillCandidateStatus>, Vec<SkillCandidateReason>) {
    let mut reasons = Vec::new();
    if parsed.quarantined
        || parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == agent_core::SkillDiagnosticSeverity::Quarantine
        })
    {
        reasons.push(SkillCandidateReason::InstructionOverride);
    }
    if parsed.manifest.scope != SkillScope::Project
        || parsed.provenance.project_id != Some(request.project_id)
    {
        reasons.push(SkillCandidateReason::ScopeMismatch);
    }
    if parsed.manifest.trace.trace_id != request.trace_id
        || parsed.provenance.trace_id != request.trace_id
    {
        reasons.push(SkillCandidateReason::TraceMismatch);
    }
    if parsed.manifest.capabilities.iter().any(|capability| {
        capability.scope.as_deref() != Some(&request.project_id.to_string())
            || !request.policy.allowed_capabilities.contains(capability)
    }) {
        reasons.push(SkillCandidateReason::CapabilityEscalation);
    }
    if parsed
        .artifacts
        .iter()
        .any(|artifact| artifact.role == SkillFileRole::Script)
    {
        reasons.push(SkillCandidateReason::ArtifactPoisoning);
    }
    if parsed
        .instructions
        .iter()
        .any(|section| contains_sensitive_marker(&section.content))
        || parsed
            .artifacts
            .iter()
            .any(|artifact| contains_sensitive_marker(&artifact.content))
    {
        reasons.push(SkillCandidateReason::SensitiveContent);
    }
    reasons.truncate(MAX_REASONS);
    if reasons.is_empty() {
        (None, reasons)
    } else {
        (Some(SkillCandidateStatus::Quarantined), reasons)
    }
}

fn candidate_digest(parsed: &ParsedSkill) -> String {
    digest_json(&(
        &parsed.manifest,
        &parsed.instructions,
        &parsed.artifacts,
        &parsed.links,
        &parsed.diagnostics,
        parsed.quarantined,
    ))
}

fn policy_digest(policy: &SkillCandidatePolicy) -> String {
    digest_json(&(
        policy.allow,
        &policy.allowed_capabilities,
        policy.max_observations,
        policy.max_document_bytes,
    ))
}

fn handoff_digest(handoff: &SkillEvaluationHandoff) -> String {
    digest_json(&(
        handoff.schema_version,
        handoff.project_id,
        &handoff.candidate_id,
        handoff.trace_id,
        &handoff.capability,
        &handoff.status,
        &handoff.candidate_digest,
        &handoff.source_digest,
        &handoff.policy_digest,
        &handoff.budget_digest,
        &handoff.rollback_version,
    ))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
