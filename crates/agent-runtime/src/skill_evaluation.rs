//! Bounded, non-activating evaluation of a Skill candidate.
//!
//! The evaluator compares a candidate with an immutable project baseline using
//! only deterministic fixture evidence. It produces a redacted report and
//! never creates, promotes, activates, executes, or mutates a Skill.

use crate::skill_repo::SkillRecord;
use crate::skill_testing::{DeterministicSkillTestRunner, SkillFixture, SkillTestReport};
use crate::skill_validation::{
    SkillValidationReport, SkillValidationService, SkillValidationStatus,
};
use agent_core::{
    Action, BudgetLimits, Capability, CapabilitySet, DomainError, ParsedSkill, ProjectId, Resource,
    SkillScope,
};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SKILL_EVALUATE_CAPABILITY: &str = "skill:evaluate";
pub const SKILL_EVALUATION_SCHEMA_VERSION: u32 = 1;

const MAX_ACTOR_ID_BYTES: usize = 128;
const MAX_EVALUATION_TESTS: u16 = 64;
const MAX_EVALUATION_STEPS: u16 = 64;
const MAX_REASONS: usize = 16;

#[derive(Debug, Clone)]
pub struct SkillEvaluationPolicy {
    pub allow: bool,
    pub allowed_capabilities: CapabilitySet,
    pub max_tests: u16,
    /// Zero is a valid policy: it represents an evaluation that times out
    /// before the candidate can execute even one bounded step.
    pub max_steps: u16,
}

#[derive(Debug, Clone)]
pub struct SkillEvaluationRequest {
    pub project_id: ProjectId,
    pub actor_id: String,
    pub capability: Capability,
    pub policy: SkillEvaluationPolicy,
    pub budget: BudgetLimits,
    pub trace_id: TraceId,
    pub baseline: SkillRecord,
    pub candidate: ParsedSkill,
    pub validation: SkillValidationReport,
    pub baseline_fixture: SkillFixture,
    pub candidate_fixture: SkillFixture,
    pub baseline_report: SkillTestReport,
    pub candidate_report: SkillTestReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillEvaluationStatus {
    Passed,
    Failed,
    TimedOut,
    Inconclusive,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillEvaluationReason {
    BaselineMismatch,
    CandidateIdentityMismatch,
    ValidationRejected,
    CapabilityDrift,
    FixtureRejected,
    EvidenceMismatch,
    TestFailed,
    Timeout,
    Inconclusive,
    BudgetExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvaluationReport {
    pub schema_version: u32,
    pub project_id: ProjectId,
    pub skill_id: agent_core::SkillId,
    pub baseline_version: String,
    pub candidate_version: String,
    pub trace_id: TraceId,
    pub status: SkillEvaluationStatus,
    pub baseline_score: u16,
    pub candidate_score: u16,
    pub score_delta: i16,
    pub baseline_content_digest: String,
    pub candidate_content_digest: String,
    pub baseline_test_digest: String,
    pub candidate_test_digest: String,
    pub validation_report_digest: String,
    pub policy_digest: String,
    pub budget_digest: String,
    pub reasons: Vec<SkillEvaluationReason>,
    pub rollback_version: Option<String>,
    pub report_digest: String,
}

pub struct SkillEvaluationService;

impl SkillEvaluationService {
    pub fn evaluate(request: SkillEvaluationRequest) -> Result<SkillEvaluationReport, DomainError> {
        validate_request(&request)?;

        if request.baseline.skill.manifest.scope != SkillScope::Project
            || request.baseline.skill.project_id != Some(request.project_id)
        {
            return Err(DomainError::PermissionDenied {
                capability: SKILL_EVALUATE_CAPABILITY.into(),
                reason: "evaluation baseline is outside the project scope".into(),
            });
        }

        let baseline_content_digest = digest_json(&(
            &request.baseline.skill,
            &request.baseline.parsed,
            request.baseline.revision,
            &request.baseline.version_id,
            &request.baseline.content_hash,
        ));
        let candidate_content_digest = digest_json(&request.candidate);

        if request.candidate.manifest.id != request.baseline.skill.manifest.id
            || request.candidate.manifest.scope != SkillScope::Project
            || request.candidate.provenance.project_id != Some(request.project_id)
            || request.candidate.provenance.trace_id != request.trace_id
        {
            return Ok(report(
                &request,
                SkillEvaluationStatus::Quarantined,
                0,
                0,
                vec![SkillEvaluationReason::CandidateIdentityMismatch],
                EvaluationDigests::new(
                    baseline_content_digest,
                    candidate_content_digest,
                    String::new(),
                    String::new(),
                ),
            ));
        }

        if !same_capabilities(
            &request.baseline.skill.manifest.capabilities,
            &request.candidate.manifest.capabilities,
        ) {
            return Ok(report(
                &request,
                SkillEvaluationStatus::Quarantined,
                0,
                0,
                vec![SkillEvaluationReason::CapabilityDrift],
                EvaluationDigests::new(
                    baseline_content_digest,
                    candidate_content_digest,
                    String::new(),
                    String::new(),
                ),
            ));
        }

        if request.validation.status != SkillValidationStatus::Passed
            || !SkillValidationService::report_is_approved(
                &request.candidate,
                request.project_id,
                request.candidate.manifest.id,
                &request.candidate.manifest.version,
                &request.validation,
            )
        {
            return Ok(report(
                &request,
                SkillEvaluationStatus::Quarantined,
                0,
                0,
                vec![SkillEvaluationReason::ValidationRejected],
                EvaluationDigests::new(
                    baseline_content_digest,
                    candidate_content_digest,
                    String::new(),
                    String::new(),
                ),
            ));
        }

        let baseline_digest = match evidence_digest(
            &request.baseline_fixture,
            &request.baseline_report,
            request.project_id,
            request.baseline.skill.manifest.id,
            &request.baseline.skill.manifest.version,
            request.baseline.skill.manifest.trace.trace_id,
        ) {
            Ok(digest) => digest,
            Err(reason) => {
                return Ok(report(
                    &request,
                    SkillEvaluationStatus::Quarantined,
                    0,
                    0,
                    vec![reason],
                    EvaluationDigests::new(
                        baseline_content_digest,
                        candidate_content_digest,
                        String::new(),
                        String::new(),
                    ),
                ));
            }
        };
        let candidate_digest = match evidence_digest(
            &request.candidate_fixture,
            &request.candidate_report,
            request.project_id,
            request.candidate.manifest.id,
            &request.candidate.manifest.version,
            request.trace_id,
        ) {
            Ok(digest) => digest,
            Err(reason) => {
                return Ok(report(
                    &request,
                    SkillEvaluationStatus::Quarantined,
                    score(&request.baseline_report),
                    0,
                    vec![reason],
                    EvaluationDigests::new(
                        baseline_content_digest,
                        candidate_content_digest,
                        baseline_digest,
                        String::new(),
                    ),
                ));
            }
        };

        if request.policy.max_tests < 2 {
            return Ok(report(
                &request,
                SkillEvaluationStatus::Quarantined,
                score(&request.baseline_report),
                0,
                vec![SkillEvaluationReason::BudgetExceeded],
                EvaluationDigests::new(
                    baseline_content_digest,
                    candidate_content_digest,
                    baseline_digest,
                    candidate_digest,
                ),
            ));
        }

        let baseline_score = score(&request.baseline_report);
        let candidate_score = score(&request.candidate_report);
        if request.candidate_report.steps_executed > request.policy.max_steps {
            return Ok(report(
                &request,
                SkillEvaluationStatus::TimedOut,
                baseline_score,
                candidate_score,
                vec![SkillEvaluationReason::Timeout],
                EvaluationDigests::new(
                    baseline_content_digest,
                    candidate_content_digest,
                    baseline_digest,
                    candidate_digest,
                ),
            ));
        }

        let status = match request.candidate_report.status.as_str() {
            "passed" if candidate_score >= baseline_score => SkillEvaluationStatus::Passed,
            "passed" => SkillEvaluationStatus::Failed,
            "failed" => SkillEvaluationStatus::Failed,
            "timed_out" => SkillEvaluationStatus::TimedOut,
            "inconclusive" => SkillEvaluationStatus::Inconclusive,
            _ => SkillEvaluationStatus::Quarantined,
        };
        let reason = match status {
            SkillEvaluationStatus::Passed => None,
            SkillEvaluationStatus::Failed => Some(SkillEvaluationReason::TestFailed),
            SkillEvaluationStatus::TimedOut => Some(SkillEvaluationReason::Timeout),
            SkillEvaluationStatus::Inconclusive => Some(SkillEvaluationReason::Inconclusive),
            SkillEvaluationStatus::Quarantined => Some(SkillEvaluationReason::EvidenceMismatch),
        };
        Ok(report(
            &request,
            status,
            baseline_score,
            candidate_score,
            reason.into_iter().collect(),
            EvaluationDigests::new(
                baseline_content_digest,
                candidate_content_digest,
                baseline_digest,
                candidate_digest,
            ),
        ))
    }
}

fn validate_request(request: &SkillEvaluationRequest) -> Result<(), DomainError> {
    if request.actor_id.trim().is_empty() || request.actor_id.len() > MAX_ACTOR_ID_BYTES {
        return Err(DomainError::Validation(
            "evaluation actor is invalid".into(),
        ));
    }
    let expected =
        Capability::new(Resource::Skill, Action::Read).with_scope(request.project_id.to_string());
    if request.capability != expected
        || !request.policy.allow
        || !request.policy.allowed_capabilities.contains(&expected)
    {
        return Err(DomainError::PermissionDenied {
            capability: SKILL_EVALUATE_CAPABILITY.into(),
            reason: "evaluation capability is not authorized".into(),
        });
    }
    if request.policy.max_tests == 0
        || request.policy.max_tests > MAX_EVALUATION_TESTS
        || request.policy.max_steps > MAX_EVALUATION_STEPS
    {
        return Err(DomainError::BudgetExceeded {
            budget_type: "skill_evaluation_limits".into(),
            limit: format!("tests<= {MAX_EVALUATION_TESTS}, steps<= {MAX_EVALUATION_STEPS}"),
            used: format!(
                "tests={}, steps={}",
                request.policy.max_tests, request.policy.max_steps
            ),
        });
    }
    if request.trace_id.as_uuid().is_nil() {
        return Err(DomainError::Validation(
            "evaluation trace is required".into(),
        ));
    }
    request.budget.validate()
}

fn evidence_digest(
    fixture: &SkillFixture,
    report: &SkillTestReport,
    project_id: ProjectId,
    skill_id: agent_core::SkillId,
    version: &str,
    trace_id: TraceId,
) -> Result<String, SkillEvaluationReason> {
    let generated = DeterministicSkillTestRunner::run(fixture)
        .map_err(|_| SkillEvaluationReason::FixtureRejected)?;
    if report.project_id != project_id
        || report.skill_id != skill_id
        || report.version != version
        || report.trace_id != trace_id
        || report.fixture_digest != generated.fixture_digest
        || report.steps_executed != generated.steps_executed
        || report.steps_executed == 0
        || report.activation_requested
        || !matches!(
            report.status.as_str(),
            "passed" | "failed" | "timed_out" | "inconclusive"
        )
    {
        return Err(SkillEvaluationReason::EvidenceMismatch);
    }
    Ok(report.fixture_digest.clone())
}

fn score(report: &SkillTestReport) -> u16 {
    u16::from(report.status == "passed") * 100
}

fn same_capabilities(baseline: &[Capability], candidate: &[Capability]) -> bool {
    digest_json(baseline) == digest_json(candidate)
}

#[derive(Debug)]
struct EvaluationDigests {
    baseline_content: String,
    candidate_content: String,
    baseline_test: String,
    candidate_test: String,
}

impl EvaluationDigests {
    fn new(
        baseline_content: String,
        candidate_content: String,
        baseline_test: String,
        candidate_test: String,
    ) -> Self {
        Self {
            baseline_content,
            candidate_content,
            baseline_test,
            candidate_test,
        }
    }
}

fn report(
    request: &SkillEvaluationRequest,
    status: SkillEvaluationStatus,
    baseline_score: u16,
    candidate_score: u16,
    mut reasons: Vec<SkillEvaluationReason>,
    digests: EvaluationDigests,
) -> SkillEvaluationReport {
    reasons.truncate(MAX_REASONS);
    let score_delta = candidate_score as i16 - baseline_score as i16;
    let fingerprint = EvaluationFingerprint {
        schema_version: SKILL_EVALUATION_SCHEMA_VERSION,
        project_id: request.project_id,
        skill_id: request.baseline.skill.manifest.id,
        baseline_version: request.baseline.skill.manifest.version.clone(),
        candidate_version: request.candidate.manifest.version.clone(),
        trace_id: request.trace_id,
        status,
        baseline_score,
        candidate_score,
        score_delta,
        baseline_content_digest: digests.baseline_content.clone(),
        candidate_content_digest: digests.candidate_content.clone(),
        baseline_test_digest: digests.baseline_test.clone(),
        candidate_test_digest: digests.candidate_test.clone(),
        validation_report_digest: request.validation.report_digest.clone(),
        policy_digest: policy_digest(&request.policy),
        budget_digest: digest_json(&request.budget),
        reasons: reasons.clone(),
        rollback_version: Some(request.baseline.skill.manifest.version.clone()),
    };
    SkillEvaluationReport {
        schema_version: SKILL_EVALUATION_SCHEMA_VERSION,
        project_id: request.project_id,
        skill_id: request.baseline.skill.manifest.id,
        baseline_version: request.baseline.skill.manifest.version.clone(),
        candidate_version: request.candidate.manifest.version.clone(),
        trace_id: request.trace_id,
        status,
        baseline_score,
        candidate_score,
        score_delta,
        baseline_content_digest: digests.baseline_content,
        candidate_content_digest: digests.candidate_content,
        baseline_test_digest: digests.baseline_test,
        candidate_test_digest: digests.candidate_test,
        validation_report_digest: request.validation.report_digest.clone(),
        policy_digest: policy_digest(&request.policy),
        budget_digest: digest_json(&request.budget),
        reasons,
        rollback_version: Some(request.baseline.skill.manifest.version.clone()),
        report_digest: digest_json(&fingerprint),
    }
}

#[derive(Debug, Serialize)]
struct EvaluationFingerprint {
    schema_version: u32,
    project_id: ProjectId,
    skill_id: agent_core::SkillId,
    baseline_version: String,
    candidate_version: String,
    trace_id: TraceId,
    status: SkillEvaluationStatus,
    baseline_score: u16,
    candidate_score: u16,
    score_delta: i16,
    baseline_content_digest: String,
    candidate_content_digest: String,
    baseline_test_digest: String,
    candidate_test_digest: String,
    validation_report_digest: String,
    policy_digest: String,
    budget_digest: String,
    reasons: Vec<SkillEvaluationReason>,
    rollback_version: Option<String>,
}

fn policy_digest(policy: &SkillEvaluationPolicy) -> String {
    digest_json(&(
        policy.allow,
        &policy.allowed_capabilities,
        policy.max_tests,
        policy.max_steps,
    ))
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}
