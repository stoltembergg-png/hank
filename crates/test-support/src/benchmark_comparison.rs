//! Deterministic comparison of one immutable baseline and one candidate run.
//!
//! The comparison owns the canonical core split, so a caller cannot select a
//! convenient subset of cases or relabel holdout data. It produces bounded
//! evidence only; it has no promotion, activation or pointer mutation path.

use crate::digest::fnv1a64;
use crate::evaluation::{
    BaselineReport, EvaluationContractError, HoldoutPartition, MetricName, MetricValue,
};
use crate::evaluation_corpus::{
    core_evaluation_corpus, CoreEvaluationFixture, EvaluationCorpusError,
    CORE_EVALUATION_CORPUS_SCHEMA_VERSION,
};
use crate::evaluation_runner::{
    NativeEvaluationEnvironment, NativeEvaluationRun, NativeEvaluationRunnerError,
    MAX_NATIVE_EVALUATION_CASES, NATIVE_EVALUATION_RUN_SCHEMA_VERSION,
};
use ring::digest::{digest as sha256_digest, SHA256};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const BENCHMARK_COMPARISON_SCHEMA_VERSION: u32 = 1;
pub const INDEPENDENT_REVIEW_SCHEMA_VERSION: u32 = 1;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_REVIEW_VERSION_BYTES: usize = 128;
const ED25519_PUBLIC_KEY_BYTES: usize = 32;
const ED25519_SIGNATURE_BYTES: usize = 64;
const MAX_POLICY_VIOLATIONS_INCREASE: u64 = 32;
const MAX_COST_INCREASE: u64 = 1_000_000;
const MAX_LATENCY_INCREASE: u64 = 60_000;
const MAX_FAILED_TOOL_CALLS_INCREASE: u64 = 32;
const COMPARED_METRICS: [MetricName; 17] = [
    MetricName::Success,
    MetricName::TerminalState,
    MetricName::TestsPassing,
    MetricName::ToolCalls,
    MetricName::FailedToolCalls,
    MetricName::Retries,
    MetricName::Tokens,
    MetricName::Cost,
    MetricName::LatencyMs,
    MetricName::HumanIntervention,
    MetricName::EvidenceQuality,
    MetricName::PolicyViolations,
    MetricName::ContextMisses,
    MetricName::MemoryHits,
    MetricName::EvidenceConflicts,
    MetricName::SkillSelection,
    MetricName::ExternalSideEffectAttempts,
];

pub const MAX_BENCHMARK_COMPARISON_DELTAS: usize =
    MAX_NATIVE_EVALUATION_CASES * COMPARED_METRICS.len();

#[derive(Debug, Error)]
pub enum BenchmarkComparisonError {
    #[error("benchmark comparison schema version is unsupported")]
    UnsupportedSchemaVersion,
    #[error("benchmark comparison input is outside its bound")]
    BoundsExceeded,
    #[error("benchmark comparison input is invalid")]
    InvalidInput,
    #[error("benchmark comparison policy is invalid")]
    InvalidPolicy,
    #[error("an independent review artifact is required")]
    MissingIndependentReview,
    #[error("independent review artifact is invalid")]
    InvalidReview,
    #[error("candidate cannot independently review itself")]
    SelfApproval,
    #[error("independent review targets a different comparison")]
    ReviewTargetMismatch,
    #[error("independent review rejected the comparison")]
    ReviewRejected,
    #[error("baseline and candidate use incomparable environments")]
    IncomparableEnvironment,
    #[error("baseline and candidate use different suites")]
    SuiteMismatch,
    #[error("baseline and candidate use different corpus schemas")]
    CorpusSchemaMismatch,
    #[error("{side} run is missing canonical case `{case_id}`")]
    MissingCase { side: String, case_id: String },
    #[error("{side} run contains unknown case `{case_id}`")]
    UnknownCase { side: String, case_id: String },
    #[error("{side} run contains duplicate case `{case_id}`")]
    DuplicateCase { side: String, case_id: String },
    #[error("{side} report for case `{case_id}` is invalid: {source}")]
    InvalidReport {
        side: String,
        case_id: String,
        #[source]
        source: EvaluationContractError,
    },
    #[error("canonical baseline report for case `{case_id}` was modified")]
    BaselineMismatch { case_id: String },
    #[error("{side} report for case `{case_id}` has inconsistent structured metrics")]
    InconsistentMetric { side: String, case_id: String },
    #[error("canonical benchmark corpus is invalid: {0}")]
    CanonicalCorpus(#[source] EvaluationCorpusError),
    #[error("benchmark comparison serialization failed: {0}")]
    Serialization(#[source] serde_json::Error),
    #[error("benchmark comparison digest is invalid")]
    InvalidDigest,
    #[error("native run is invalid: {0}")]
    InvalidRun(#[source] NativeEvaluationRunnerError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkComparisonPolicy {
    pub revision: String,
    pub max_success_regression: u32,
    pub max_policy_violations_increase: u64,
    pub max_evidence_quality_regression: f64,
    pub max_cost_increase: u64,
    pub max_latency_increase: u64,
    pub max_failed_tool_calls_increase: u64,
}

impl Default for BenchmarkComparisonPolicy {
    fn default() -> Self {
        Self {
            revision: "benchmark-policy-v1".into(),
            max_success_regression: 0,
            max_policy_violations_increase: 0,
            max_evidence_quality_regression: 0.0,
            max_cost_increase: 0,
            max_latency_increase: 0,
            max_failed_tool_calls_increase: 0,
        }
    }
}

impl BenchmarkComparisonPolicy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        revision: impl Into<String>,
        max_success_regression: u32,
        max_policy_violations_increase: u64,
        max_evidence_quality_regression: f64,
        max_cost_increase: u64,
        max_latency_increase: u64,
        max_failed_tool_calls_increase: u64,
    ) -> Result<Self, BenchmarkComparisonError> {
        let policy = Self {
            revision: revision.into(),
            max_success_regression,
            max_policy_violations_increase,
            max_evidence_quality_regression,
            max_cost_increase,
            max_latency_increase,
            max_failed_tool_calls_increase,
        };
        policy.validate()?;
        Ok(policy)
    }

    fn validate(&self) -> Result<(), BenchmarkComparisonError> {
        validate_text(&self.revision, MAX_IDENTIFIER_BYTES)?;
        if self.max_success_regression as usize > MAX_NATIVE_EVALUATION_CASES
            || self.max_policy_violations_increase > MAX_POLICY_VIOLATIONS_INCREASE
            || !self.max_evidence_quality_regression.is_finite()
            || !(0.0..=1.0).contains(&self.max_evidence_quality_regression)
            || self.max_cost_increase > MAX_COST_INCREASE
            || self.max_latency_increase > MAX_LATENCY_INCREASE
            || self.max_failed_tool_calls_increase > MAX_FAILED_TOOL_CALLS_INCREASE
        {
            return Err(BenchmarkComparisonError::InvalidPolicy);
        }
        Ok(())
    }

    /// Stable identity of the exact policy contents. Reviewers sign this
    /// digest so thresholds cannot be changed after review.
    pub fn digest(&self) -> String {
        let encoded = serde_json::to_vec(self).expect("benchmark policy is serializable");
        sha256_hex(&encoded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndependentReviewDisposition {
    Reviewed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentReviewArtifact {
    pub schema_version: u32,
    pub reviewer_id: String,
    pub reviewer_version: String,
    pub baseline_id: String,
    pub candidate_id: String,
    pub baseline_run_digest: String,
    pub candidate_run_digest: String,
    pub policy_digest: String,
    pub disposition: IndependentReviewDisposition,
    pub signature: String,
    pub artifact_digest: String,
}

/// Trusted review signer. Its private key must stay in the independent review
/// service; consumers should pass only [`IndependentReviewVerifier`] to the
/// comparison gate.
pub struct IndependentReviewSigner {
    reviewer_id: String,
    reviewer_version: String,
    key_pair: Ed25519KeyPair,
}

impl IndependentReviewSigner {
    /// Creates a signer from an Ed25519 seed held by the trusted reviewer.
    ///
    /// The seed is never serialized into a review artifact.
    pub fn from_seed(
        reviewer_id: impl Into<String>,
        reviewer_version: impl Into<String>,
        seed: impl AsRef<[u8]>,
    ) -> Result<Self, BenchmarkComparisonError> {
        let reviewer_id = reviewer_id.into();
        let reviewer_version = reviewer_version.into();
        validate_reviewer_identity(&reviewer_id, &reviewer_version)?;
        let key_pair = Ed25519KeyPair::from_seed_unchecked(seed.as_ref())
            .map_err(|_| BenchmarkComparisonError::InvalidReview)?;
        Ok(Self {
            reviewer_id,
            reviewer_version,
            key_pair,
        })
    }

    pub fn verifier(&self) -> IndependentReviewVerifier {
        IndependentReviewVerifier {
            reviewer_id: self.reviewer_id.clone(),
            reviewer_version: self.reviewer_version.clone(),
            public_key: self.key_pair.public_key().as_ref().to_vec(),
        }
    }

    pub fn issue(
        &self,
        baseline_id: impl Into<String>,
        candidate_id: impl Into<String>,
        baseline_run_digest: impl AsRef<str>,
        candidate_run_digest: impl AsRef<str>,
        policy_digest: impl AsRef<str>,
        disposition: IndependentReviewDisposition,
    ) -> Result<IndependentReviewArtifact, BenchmarkComparisonError> {
        let mut artifact = IndependentReviewArtifact {
            schema_version: INDEPENDENT_REVIEW_SCHEMA_VERSION,
            reviewer_id: self.reviewer_id.clone(),
            reviewer_version: self.reviewer_version.clone(),
            baseline_id: baseline_id.into(),
            candidate_id: candidate_id.into(),
            baseline_run_digest: baseline_run_digest.as_ref().to_owned(),
            candidate_run_digest: candidate_run_digest.as_ref().to_owned(),
            policy_digest: policy_digest.as_ref().to_owned(),
            disposition,
            signature: String::new(),
            artifact_digest: String::new(),
        };
        artifact.validate_identity()?;
        artifact.signature = hex_encode(self.key_pair.sign(&artifact.signing_payload()).as_ref());
        artifact.artifact_digest = artifact.content_digest();
        artifact.validate()?;
        Ok(artifact)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndependentReviewVerifier {
    reviewer_id: String,
    reviewer_version: String,
    public_key: Vec<u8>,
}

impl IndependentReviewVerifier {
    pub fn new(
        reviewer_id: impl Into<String>,
        reviewer_version: impl Into<String>,
        public_key: impl AsRef<[u8]>,
    ) -> Result<Self, BenchmarkComparisonError> {
        let reviewer_id = reviewer_id.into();
        let reviewer_version = reviewer_version.into();
        validate_reviewer_identity(&reviewer_id, &reviewer_version)?;
        if public_key.as_ref().len() != ED25519_PUBLIC_KEY_BYTES {
            return Err(BenchmarkComparisonError::InvalidReview);
        }
        Ok(Self {
            reviewer_id,
            reviewer_version,
            public_key: public_key.as_ref().to_vec(),
        })
    }

    fn verify_for(
        &self,
        artifact: &IndependentReviewArtifact,
        baseline_id: &str,
        candidate_id: &str,
        baseline_run_digest: &str,
        candidate_run_digest: &str,
        policy_digest: &str,
    ) -> Result<(), BenchmarkComparisonError> {
        artifact.validate()?;
        if artifact.reviewer_id != self.reviewer_id
            || artifact.reviewer_version != self.reviewer_version
        {
            return Err(BenchmarkComparisonError::InvalidReview);
        }
        if artifact.baseline_id != baseline_id
            || artifact.candidate_id != candidate_id
            || artifact.baseline_run_digest != baseline_run_digest
            || artifact.candidate_run_digest != candidate_run_digest
            || artifact.policy_digest != policy_digest
        {
            return Err(BenchmarkComparisonError::ReviewTargetMismatch);
        }
        if artifact.disposition == IndependentReviewDisposition::Rejected {
            return Err(BenchmarkComparisonError::ReviewRejected);
        }
        let signature = decode_hex(&artifact.signature, ED25519_SIGNATURE_BYTES)?;
        UnparsedPublicKey::new(&ED25519, self.public_key.as_slice())
            .verify(&artifact.signing_payload(), &signature)
            .map_err(|_| BenchmarkComparisonError::InvalidReview)
    }
}

impl IndependentReviewArtifact {
    pub fn validate(&self) -> Result<(), BenchmarkComparisonError> {
        self.validate_identity()?;
        decode_hex(&self.signature, ED25519_SIGNATURE_BYTES)?;
        if self.artifact_digest.is_empty() || self.artifact_digest != self.content_digest() {
            return Err(BenchmarkComparisonError::InvalidDigest);
        }
        Ok(())
    }

    fn validate_identity(&self) -> Result<(), BenchmarkComparisonError> {
        if self.schema_version != INDEPENDENT_REVIEW_SCHEMA_VERSION {
            return Err(BenchmarkComparisonError::UnsupportedSchemaVersion);
        }
        validate_text(&self.reviewer_id, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.reviewer_version, MAX_REVIEW_VERSION_BYTES)?;
        validate_text(&self.baseline_id, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.candidate_id, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.baseline_run_digest, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.candidate_run_digest, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.policy_digest, MAX_IDENTIFIER_BYTES)?;
        if self.baseline_id == self.candidate_id {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        if self.reviewer_id == self.baseline_id || self.reviewer_id == self.candidate_id {
            return Err(BenchmarkComparisonError::SelfApproval);
        }
        Ok(())
    }

    fn signing_payload(&self) -> Vec<u8> {
        let mut content = self.clone();
        content.signature.clear();
        content.artifact_digest.clear();
        serde_json::to_vec(&content).expect("review artifact is serializable")
    }

    fn content_digest(&self) -> String {
        let mut content = self.clone();
        content.artifact_digest.clear();
        let encoded = serde_json::to_vec(&content).expect("review artifact is serializable");
        format!("{:016x}", fnv1a64(&encoded))
    }
}

fn validate_reviewer_identity(
    reviewer_id: &str,
    reviewer_version: &str,
) -> Result<(), BenchmarkComparisonError> {
    validate_text(reviewer_id, MAX_IDENTIFIER_BYTES)?;
    validate_text(reviewer_version, MAX_REVIEW_VERSION_BYTES)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkMetricDelta {
    pub case_id: String,
    pub partition: HoldoutPartition,
    pub metric: MetricName,
    pub baseline: MetricValue,
    pub candidate: MetricValue,
    pub delta: f64,
    pub regressed: bool,
}

impl BenchmarkMetricDelta {
    fn validate(&self) -> Result<(), BenchmarkComparisonError> {
        validate_text(&self.case_id, MAX_IDENTIFIER_BYTES)?;
        if !COMPARED_METRICS.contains(&self.metric) || !self.delta.is_finite() {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        match self.metric {
            MetricName::Success | MetricName::HumanIntervention => {
                if !matches!(
                    (&self.baseline, &self.candidate),
                    (MetricValue::Boolean(_), MetricValue::Boolean(_))
                ) {
                    return Err(BenchmarkComparisonError::InvalidInput);
                }
            }
            MetricName::TerminalState | MetricName::SkillSelection => {
                if !matches!(
                    (&self.baseline, &self.candidate),
                    (MetricValue::Category(_), MetricValue::Category(_))
                ) {
                    return Err(BenchmarkComparisonError::InvalidInput);
                }
            }
            MetricName::EvidenceQuality => {
                if !matches!(
                    (&self.baseline, &self.candidate),
                    (MetricValue::Ratio(_), MetricValue::Ratio(_))
                ) {
                    return Err(BenchmarkComparisonError::InvalidInput);
                }
            }
            MetricName::LatencyMs => {
                if !matches!(
                    (&self.baseline, &self.candidate),
                    (MetricValue::DurationMs(_), MetricValue::DurationMs(_))
                ) {
                    return Err(BenchmarkComparisonError::InvalidInput);
                }
            }
            MetricName::TestsPassing
            | MetricName::ToolCalls
            | MetricName::FailedToolCalls
            | MetricName::Retries
            | MetricName::Tokens
            | MetricName::Cost
            | MetricName::PolicyViolations
            | MetricName::ContextMisses
            | MetricName::MemoryHits
            | MetricName::EvidenceConflicts
            | MetricName::ExternalSideEffectAttempts => {
                if !matches!(
                    (&self.baseline, &self.candidate),
                    (MetricValue::Count(_), MetricValue::Count(_))
                ) {
                    return Err(BenchmarkComparisonError::InvalidInput);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkPartitionSummary {
    pub partition: HoldoutPartition,
    pub case_count: u32,
    pub baseline_successes: u32,
    pub candidate_successes: u32,
    pub regression_count: u32,
    pub deltas: Vec<BenchmarkMetricDelta>,
}

impl BenchmarkPartitionSummary {
    fn validate(&self) -> Result<(), BenchmarkComparisonError> {
        if self.case_count == 0
            || self.case_count as usize > MAX_NATIVE_EVALUATION_CASES
            || self.baseline_successes > self.case_count
            || self.candidate_successes > self.case_count
            || self.deltas.len() > MAX_BENCHMARK_COMPARISON_DELTAS
        {
            return Err(BenchmarkComparisonError::BoundsExceeded);
        }
        let mut keys = BTreeSet::new();
        let mut regressions = 0usize;
        for delta in &self.deltas {
            delta.validate()?;
            if delta.partition != self.partition
                || !keys.insert((delta.case_id.clone(), delta.metric))
            {
                return Err(BenchmarkComparisonError::InvalidInput);
            }
            if delta.regressed {
                regressions += 1;
            }
        }
        if regressions != self.regression_count as usize {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkComparisonStatus {
    Pass,
    Regression,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkComparisonReport {
    pub schema_version: u32,
    pub suite_id: String,
    pub corpus_schema_version: u32,
    pub policy: BenchmarkComparisonPolicy,
    pub baseline_id: String,
    pub candidate_id: String,
    pub baseline_run_digest: String,
    pub candidate_run_digest: String,
    pub baseline_environment: NativeEvaluationEnvironment,
    pub candidate_environment: NativeEvaluationEnvironment,
    pub training: BenchmarkPartitionSummary,
    pub holdout: BenchmarkPartitionSummary,
    pub regressions: Vec<BenchmarkMetricDelta>,
    pub independent_review: IndependentReviewArtifact,
    pub status: BenchmarkComparisonStatus,
    pub report_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkComparisonReportWire {
    schema_version: u32,
    suite_id: String,
    corpus_schema_version: u32,
    policy: BenchmarkComparisonPolicy,
    baseline_id: String,
    candidate_id: String,
    baseline_run_digest: String,
    candidate_run_digest: String,
    baseline_environment: NativeEvaluationEnvironment,
    candidate_environment: NativeEvaluationEnvironment,
    training: BenchmarkPartitionSummary,
    holdout: BenchmarkPartitionSummary,
    regressions: Vec<BenchmarkMetricDelta>,
    independent_review: IndependentReviewArtifact,
    status: BenchmarkComparisonStatus,
    report_digest: String,
}

impl<'de> Deserialize<'de> for BenchmarkComparisonReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BenchmarkComparisonReportWire::deserialize(deserializer)?;
        let report = Self {
            schema_version: wire.schema_version,
            suite_id: wire.suite_id,
            corpus_schema_version: wire.corpus_schema_version,
            policy: wire.policy,
            baseline_id: wire.baseline_id,
            candidate_id: wire.candidate_id,
            baseline_run_digest: wire.baseline_run_digest,
            candidate_run_digest: wire.candidate_run_digest,
            baseline_environment: wire.baseline_environment,
            candidate_environment: wire.candidate_environment,
            training: wire.training,
            holdout: wire.holdout,
            regressions: wire.regressions,
            independent_review: wire.independent_review,
            status: wire.status,
            report_digest: wire.report_digest,
        };
        report.validate().map_err(D::Error::custom)?;
        Ok(report)
    }
}

impl BenchmarkComparisonReport {
    /// Validates the bounded, canonical shape and the report's own digest.
    ///
    /// This does not authenticate the reviewer or prove that the candidate
    /// values came from the referenced runs. Call
    /// [`BenchmarkComparison::verify_report`] before treating a deserialized
    /// report as comparison evidence.
    pub fn validate(&self) -> Result<(), BenchmarkComparisonError> {
        self.validate_shape()?;
        if self.report_digest.is_empty() || self.report_digest != self.content_digest() {
            return Err(BenchmarkComparisonError::InvalidDigest);
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), BenchmarkComparisonError> {
        if self.schema_version != BENCHMARK_COMPARISON_SCHEMA_VERSION {
            return Err(BenchmarkComparisonError::UnsupportedSchemaVersion);
        }
        if self.corpus_schema_version != CORE_EVALUATION_CORPUS_SCHEMA_VERSION {
            return Err(BenchmarkComparisonError::CorpusSchemaMismatch);
        }
        validate_text(&self.suite_id, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.baseline_id, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.candidate_id, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.baseline_run_digest, MAX_IDENTIFIER_BYTES)?;
        validate_text(&self.candidate_run_digest, MAX_IDENTIFIER_BYTES)?;
        if self.baseline_id == self.candidate_id {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        self.policy.validate()?;
        validate_environment(&self.baseline_environment)?;
        validate_environment(&self.candidate_environment)?;
        if self.baseline_environment.policy_digest != self.candidate_environment.policy_digest
            || self.baseline_environment.schema_digest != self.candidate_environment.schema_digest
            || self.baseline_environment.environment_digest
                != self.candidate_environment.environment_digest
        {
            return Err(BenchmarkComparisonError::IncomparableEnvironment);
        }
        let corpus = core_evaluation_corpus().map_err(BenchmarkComparisonError::CanonicalCorpus)?;
        if self.suite_id != corpus[0].case.holdout.suite_id {
            return Err(BenchmarkComparisonError::SuiteMismatch);
        }
        let canonical_environment =
            NativeEvaluationEnvironment::from_evidence(&corpus[0].baseline.evidence)
                .map_err(|_| BenchmarkComparisonError::IncomparableEnvironment)?;
        if self.baseline_environment != canonical_environment {
            return Err(BenchmarkComparisonError::IncomparableEnvironment);
        }
        self.training.validate()?;
        self.holdout.validate()?;
        if self.training.partition != HoldoutPartition::Training
            || self.holdout.partition != HoldoutPartition::Holdout
        {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        if self.regressions.len() > MAX_BENCHMARK_COMPARISON_DELTAS
            || self.regressions.len()
                != self.training.regression_count as usize + self.holdout.regression_count as usize
        {
            return Err(BenchmarkComparisonError::BoundsExceeded);
        }
        if self
            .regressions
            .iter()
            .any(|regression| !regression.regressed)
        {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        let expected_regressions = self
            .training
            .deltas
            .iter()
            .chain(self.holdout.deltas.iter())
            .filter(|delta| delta.regressed)
            .cloned()
            .collect::<Vec<_>>();
        if expected_regressions != self.regressions {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        let expected_status = if self.regressions.is_empty() {
            BenchmarkComparisonStatus::Pass
        } else {
            BenchmarkComparisonStatus::Regression
        };
        if self.status != expected_status {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        self.independent_review.validate()?;
        if self.independent_review.policy_digest != self.policy.digest() {
            return Err(BenchmarkComparisonError::ReviewTargetMismatch);
        }
        validate_partition_shape(
            &self.training,
            HoldoutPartition::Training,
            &corpus,
            &self.policy,
        )?;
        validate_partition_shape(
            &self.holdout,
            HoldoutPartition::Holdout,
            &corpus,
            &self.policy,
        )?;
        Ok(())
    }

    fn content_digest(&self) -> String {
        let mut content = self.clone();
        content.report_digest.clear();
        let encoded = serde_json::to_vec(&content).expect("benchmark report is serializable");
        format!("{:016x}", fnv1a64(&encoded))
    }
}

pub struct BenchmarkComparison;

impl BenchmarkComparison {
    /// Recomputes a serialized report from the exact source runs and verifies
    /// its independent review signature before accepting it as evidence.
    pub fn verify_report(
        report: &BenchmarkComparisonReport,
        baseline: &NativeEvaluationRun,
        candidate: &NativeEvaluationRun,
        review_verifier: &IndependentReviewVerifier,
    ) -> Result<(), BenchmarkComparisonError> {
        report.validate()?;
        let expected = Self::compare(
            &report.baseline_id,
            &report.candidate_id,
            baseline,
            candidate,
            &report.policy,
            Some(&report.independent_review),
            review_verifier,
        )?;
        if expected != *report {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        Ok(())
    }

    pub fn compare(
        baseline_id: impl Into<String>,
        candidate_id: impl Into<String>,
        baseline: &NativeEvaluationRun,
        candidate: &NativeEvaluationRun,
        policy: &BenchmarkComparisonPolicy,
        independent_review: Option<&IndependentReviewArtifact>,
        review_verifier: &IndependentReviewVerifier,
    ) -> Result<BenchmarkComparisonReport, BenchmarkComparisonError> {
        let baseline_id = baseline_id.into();
        let candidate_id = candidate_id.into();
        validate_text(&baseline_id, MAX_IDENTIFIER_BYTES)?;
        validate_text(&candidate_id, MAX_IDENTIFIER_BYTES)?;
        if baseline_id == candidate_id {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        policy.validate()?;
        let review =
            independent_review.ok_or(BenchmarkComparisonError::MissingIndependentReview)?;

        if baseline.suite_id != candidate.suite_id {
            return Err(BenchmarkComparisonError::SuiteMismatch);
        }
        if baseline.corpus_schema_version != candidate.corpus_schema_version {
            return Err(BenchmarkComparisonError::CorpusSchemaMismatch);
        }
        if baseline.environment.policy_digest != candidate.environment.policy_digest
            || baseline.environment.schema_digest != candidate.environment.schema_digest
            || baseline.environment.environment_digest != candidate.environment.environment_digest
        {
            return Err(BenchmarkComparisonError::IncomparableEnvironment);
        }
        validate_environment(&baseline.environment)?;
        validate_environment(&candidate.environment)?;

        let corpus = core_evaluation_corpus().map_err(BenchmarkComparisonError::CanonicalCorpus)?;
        if baseline.suite_id != corpus[0].case.holdout.suite_id {
            return Err(BenchmarkComparisonError::SuiteMismatch);
        }
        if baseline.schema_version != NATIVE_EVALUATION_RUN_SCHEMA_VERSION
            || candidate.schema_version != NATIVE_EVALUATION_RUN_SCHEMA_VERSION
            || baseline.corpus_schema_version != CORE_EVALUATION_CORPUS_SCHEMA_VERSION
        {
            return Err(BenchmarkComparisonError::CorpusSchemaMismatch);
        }
        let canonical_environment =
            NativeEvaluationEnvironment::from_evidence(&corpus[0].baseline.evidence)
                .map_err(|_| BenchmarkComparisonError::IncomparableEnvironment)?;
        if baseline.environment != canonical_environment {
            return Err(BenchmarkComparisonError::IncomparableEnvironment);
        }

        let baseline_reports = index_reports(baseline, "baseline", &corpus)?;
        let candidate_reports = index_reports(candidate, "candidate", &corpus)?;
        validate_run_digest(baseline)?;
        validate_run_digest(candidate)?;
        review_verifier.verify_for(
            review,
            &baseline_id,
            &candidate_id,
            &baseline.run_digest,
            &candidate.run_digest,
            &policy.digest(),
        )?;

        for entry in &corpus {
            let baseline_report = baseline_reports
                .get(&entry.case.case_id)
                .expect("canonical baseline case was indexed");
            let candidate_report = candidate_reports
                .get(&entry.case.case_id)
                .expect("canonical candidate case was indexed");
            validate_report(
                baseline_report,
                &entry.case,
                &baseline.environment,
                "baseline",
            )?;
            validate_report(
                candidate_report,
                &entry.case,
                &candidate.environment,
                "candidate",
            )?;
            let encoded_actual = serde_json::to_vec(baseline_report)
                .map_err(BenchmarkComparisonError::Serialization)?;
            let encoded_expected = serde_json::to_vec(&entry.baseline)
                .map_err(BenchmarkComparisonError::Serialization)?;
            if encoded_actual != encoded_expected {
                return Err(BenchmarkComparisonError::BaselineMismatch {
                    case_id: entry.case.case_id.clone(),
                });
            }
        }

        let training = compare_partition(
            HoldoutPartition::Training,
            &corpus,
            &baseline_reports,
            &candidate_reports,
            policy,
        )?;
        let holdout = compare_partition(
            HoldoutPartition::Holdout,
            &corpus,
            &baseline_reports,
            &candidate_reports,
            policy,
        )?;
        let regressions = training
            .deltas
            .iter()
            .chain(holdout.deltas.iter())
            .filter(|delta| delta.regressed)
            .cloned()
            .collect::<Vec<_>>();
        if regressions.len() > MAX_BENCHMARK_COMPARISON_DELTAS {
            return Err(BenchmarkComparisonError::BoundsExceeded);
        }

        let mut report = BenchmarkComparisonReport {
            schema_version: BENCHMARK_COMPARISON_SCHEMA_VERSION,
            suite_id: baseline.suite_id.clone(),
            corpus_schema_version: baseline.corpus_schema_version,
            policy: policy.clone(),
            baseline_id,
            candidate_id,
            baseline_run_digest: baseline.run_digest.clone(),
            candidate_run_digest: candidate.run_digest.clone(),
            baseline_environment: baseline.environment.clone(),
            candidate_environment: candidate.environment.clone(),
            training,
            holdout,
            regressions,
            independent_review: review.clone(),
            status: BenchmarkComparisonStatus::Pass,
            report_digest: String::new(),
        };
        report.status = if report.regressions.is_empty() {
            BenchmarkComparisonStatus::Pass
        } else {
            BenchmarkComparisonStatus::Regression
        };
        report.report_digest = report.content_digest();
        report.validate()?;
        Ok(report)
    }
}

fn validate_environment(
    environment: &NativeEvaluationEnvironment,
) -> Result<(), BenchmarkComparisonError> {
    NativeEvaluationEnvironment::new(
        environment.head_sha.clone(),
        environment.tree_sha.clone(),
        environment.policy_digest.clone(),
        environment.schema_digest.clone(),
        environment.environment_digest.clone(),
    )
    .map(|_| ())
    .map_err(|_| BenchmarkComparisonError::IncomparableEnvironment)
}

fn validate_run_digest(run: &NativeEvaluationRun) -> Result<(), BenchmarkComparisonError> {
    if run.reports.is_empty() || run.reports.len() > MAX_NATIVE_EVALUATION_CASES {
        return Err(BenchmarkComparisonError::BoundsExceeded);
    }
    let mut content = run.clone();
    content.run_digest.clear();
    let encoded = serde_json::to_vec(&content).map_err(BenchmarkComparisonError::Serialization)?;
    let expected = format!("{:016x}", fnv1a64(&encoded));
    if run.run_digest != expected {
        return Err(BenchmarkComparisonError::InvalidDigest);
    }
    Ok(())
}

fn index_reports<'a>(
    run: &'a NativeEvaluationRun,
    side: &str,
    corpus: &[CoreEvaluationFixture],
) -> Result<BTreeMap<String, &'a BaselineReport>, BenchmarkComparisonError> {
    if run.reports.is_empty() || run.reports.len() > MAX_NATIVE_EVALUATION_CASES {
        return Err(BenchmarkComparisonError::BoundsExceeded);
    }
    let canonical_ids = corpus
        .iter()
        .map(|entry| entry.case.case_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut indexed = BTreeMap::new();
    for report in &run.reports {
        if !canonical_ids.contains(report.case_id.as_str()) {
            return Err(BenchmarkComparisonError::UnknownCase {
                side: side.into(),
                case_id: report.case_id.clone(),
            });
        }
        if indexed.insert(report.case_id.clone(), report).is_some() {
            return Err(BenchmarkComparisonError::DuplicateCase {
                side: side.into(),
                case_id: report.case_id.clone(),
            });
        }
    }
    for entry in corpus {
        if !indexed.contains_key(&entry.case.case_id) {
            return Err(BenchmarkComparisonError::MissingCase {
                side: side.into(),
                case_id: entry.case.case_id.clone(),
            });
        }
    }
    Ok(indexed)
}

fn validate_partition_shape(
    summary: &BenchmarkPartitionSummary,
    partition: HoldoutPartition,
    corpus: &[CoreEvaluationFixture],
    policy: &BenchmarkComparisonPolicy,
) -> Result<(), BenchmarkComparisonError> {
    let entries = corpus
        .iter()
        .filter(|entry| entry.case.holdout.partition == partition)
        .collect::<Vec<_>>();
    if summary.case_count as usize != entries.len()
        || summary.deltas.len() != entries.len() * COMPARED_METRICS.len()
    {
        return Err(BenchmarkComparisonError::InvalidInput);
    }

    let success_losses = summary
        .deltas
        .iter()
        .filter(|delta| delta.metric == MetricName::Success && delta.delta.is_sign_negative())
        .count();
    let mut seen = BTreeSet::new();
    for delta in &summary.deltas {
        let entry = entries
            .iter()
            .find(|entry| entry.case.case_id == delta.case_id)
            .ok_or_else(|| BenchmarkComparisonError::UnknownCase {
                side: "report".into(),
                case_id: delta.case_id.clone(),
            })?;
        let key = (delta.case_id.clone(), delta.metric);
        if !seen.insert(key) {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
        let expected_baseline = metric_value(&entry.baseline, delta.metric).ok_or_else(|| {
            BenchmarkComparisonError::InconsistentMetric {
                side: "canonical-baseline".into(),
                case_id: delta.case_id.clone(),
            }
        })?;
        if delta.baseline != expected_baseline {
            return Err(BenchmarkComparisonError::BaselineMismatch {
                case_id: delta.case_id.clone(),
            });
        }
        let expected = build_delta_from_values(
            &delta.case_id,
            partition,
            delta.metric,
            delta.baseline.clone(),
            delta.candidate.clone(),
            policy,
        )?;
        let expected_regressed = expected.regressed
            || (delta.metric == MetricName::Success
                && delta.delta.is_sign_negative()
                && success_losses > policy.max_success_regression as usize);
        if expected.delta != delta.delta || expected_regressed != delta.regressed {
            return Err(BenchmarkComparisonError::InvalidInput);
        }
    }

    for entry in entries {
        for metric in COMPARED_METRICS {
            if !seen.contains(&(entry.case.case_id.clone(), metric)) {
                return Err(BenchmarkComparisonError::InvalidInput);
            }
        }
    }

    let baseline_successes = summary
        .deltas
        .iter()
        .filter(|delta| {
            delta.metric == MetricName::Success && delta.baseline == MetricValue::Boolean(true)
        })
        .count() as u32;
    let candidate_successes = summary
        .deltas
        .iter()
        .filter(|delta| {
            delta.metric == MetricName::Success && delta.candidate == MetricValue::Boolean(true)
        })
        .count() as u32;
    if summary.baseline_successes != baseline_successes
        || summary.candidate_successes != candidate_successes
    {
        return Err(BenchmarkComparisonError::InvalidInput);
    }
    Ok(())
}

fn validate_report(
    report: &BaselineReport,
    case: &crate::evaluation::EvaluationCase,
    environment: &NativeEvaluationEnvironment,
    side: &str,
) -> Result<(), BenchmarkComparisonError> {
    report
        .validate_against(case)
        .map_err(|source| BenchmarkComparisonError::InvalidReport {
            side: side.into(),
            case_id: case.case_id.clone(),
            source,
        })?;
    if report.evidence.head_sha != environment.head_sha
        || report.evidence.tree_sha != environment.tree_sha
        || report.evidence.policy_digest != environment.policy_digest
        || report.evidence.schema_digest != environment.schema_digest
        || report.evidence.environment_digest != environment.environment_digest
    {
        return Err(BenchmarkComparisonError::IncomparableEnvironment);
    }
    if metric_value(report, MetricName::Success)
        != Some(MetricValue::Boolean(
            report.terminal == crate::evaluation::EvaluationTerminal::Pass,
        ))
        || metric_value(report, MetricName::TerminalState)
            != Some(MetricValue::Category(
                terminal_label(report.terminal).into(),
            ))
    {
        return Err(BenchmarkComparisonError::InconsistentMetric {
            side: side.into(),
            case_id: case.case_id.clone(),
        });
    }
    Ok(())
}

fn compare_partition(
    partition: HoldoutPartition,
    corpus: &[CoreEvaluationFixture],
    baseline_reports: &BTreeMap<String, &BaselineReport>,
    candidate_reports: &BTreeMap<String, &BaselineReport>,
    policy: &BenchmarkComparisonPolicy,
) -> Result<BenchmarkPartitionSummary, BenchmarkComparisonError> {
    let entries = corpus
        .iter()
        .filter(|entry| entry.case.holdout.partition == partition)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return Err(BenchmarkComparisonError::InvalidInput);
    }
    let mut deltas = Vec::with_capacity(entries.len() * COMPARED_METRICS.len());
    let mut baseline_successes = 0;
    let mut candidate_successes = 0;
    for entry in entries {
        let baseline = baseline_reports
            .get(&entry.case.case_id)
            .expect("baseline case exists");
        let candidate = candidate_reports
            .get(&entry.case.case_id)
            .expect("candidate case exists");
        if baseline.terminal == crate::evaluation::EvaluationTerminal::Pass {
            baseline_successes += 1;
        }
        if candidate.terminal == crate::evaluation::EvaluationTerminal::Pass {
            candidate_successes += 1;
        }
        for metric in COMPARED_METRICS {
            deltas.push(build_delta(
                &entry.case.case_id,
                partition,
                metric,
                baseline,
                candidate,
                policy,
            )?);
        }
    }
    let success_losses = deltas
        .iter()
        .filter(|delta| delta.metric == MetricName::Success && delta.delta.is_sign_negative())
        .count();
    if success_losses > policy.max_success_regression as usize {
        for delta in &mut deltas {
            if delta.metric == MetricName::Success && delta.delta.is_sign_negative() {
                delta.regressed = true;
            }
        }
    }
    let regression_count = deltas.iter().filter(|delta| delta.regressed).count();
    if deltas.len() > MAX_BENCHMARK_COMPARISON_DELTAS {
        return Err(BenchmarkComparisonError::BoundsExceeded);
    }
    Ok(BenchmarkPartitionSummary {
        partition,
        case_count: (deltas.len() / COMPARED_METRICS.len()) as u32,
        baseline_successes,
        candidate_successes,
        regression_count: regression_count as u32,
        deltas,
    })
}

fn build_delta(
    case_id: &str,
    partition: HoldoutPartition,
    metric: MetricName,
    baseline: &BaselineReport,
    candidate: &BaselineReport,
    policy: &BenchmarkComparisonPolicy,
) -> Result<BenchmarkMetricDelta, BenchmarkComparisonError> {
    let baseline_value = metric_value(baseline, metric).ok_or_else(|| {
        BenchmarkComparisonError::InconsistentMetric {
            side: "baseline".into(),
            case_id: case_id.into(),
        }
    })?;
    let candidate_value = metric_value(candidate, metric).ok_or_else(|| {
        BenchmarkComparisonError::InconsistentMetric {
            side: "candidate".into(),
            case_id: case_id.into(),
        }
    })?;
    build_delta_from_values(
        case_id,
        partition,
        metric,
        baseline_value,
        candidate_value,
        policy,
    )
}

fn build_delta_from_values(
    case_id: &str,
    partition: HoldoutPartition,
    metric: MetricName,
    baseline_value: MetricValue,
    candidate_value: MetricValue,
    policy: &BenchmarkComparisonPolicy,
) -> Result<BenchmarkMetricDelta, BenchmarkComparisonError> {
    let (delta, regressed) = match (&baseline_value, &candidate_value, metric) {
        (MetricValue::Boolean(base), MetricValue::Boolean(next), MetricName::Success) => {
            (i32::from(*next) as f64 - i32::from(*base) as f64, false)
        }
        (MetricValue::Boolean(base), MetricValue::Boolean(next), MetricName::HumanIntervention) => {
            (
                i32::from(*next) as f64 - i32::from(*base) as f64,
                base != next,
            )
        }
        (MetricValue::Category(base), MetricValue::Category(next), MetricName::TerminalState) => {
            (0.0, partition == HoldoutPartition::Holdout && base != next)
        }
        (MetricValue::Category(base), MetricValue::Category(next), MetricName::SkillSelection) => {
            (0.0, base != next)
        }
        (MetricValue::Count(base), MetricValue::Count(next), MetricName::TestsPassing)
        | (MetricValue::Count(base), MetricValue::Count(next), MetricName::MemoryHits) => {
            (*next as f64 - *base as f64, next < base)
        }
        (
            MetricValue::Count(base),
            MetricValue::Count(next),
            MetricName::ToolCalls
            | MetricName::Retries
            | MetricName::Tokens
            | MetricName::ContextMisses
            | MetricName::EvidenceConflicts
            | MetricName::ExternalSideEffectAttempts,
        ) => (*next as f64 - *base as f64, next > base),
        (MetricValue::Count(base), MetricValue::Count(next), MetricName::PolicyViolations) => (
            *next as f64 - *base as f64,
            *next > base.saturating_add(policy.max_policy_violations_increase),
        ),
        (MetricValue::Ratio(base), MetricValue::Ratio(next), MetricName::EvidenceQuality) => (
            *next - *base,
            *next + policy.max_evidence_quality_regression < *base,
        ),
        (MetricValue::Count(base), MetricValue::Count(next), MetricName::Cost) => (
            *next as f64 - *base as f64,
            *next > base.saturating_add(policy.max_cost_increase),
        ),
        (MetricValue::DurationMs(base), MetricValue::DurationMs(next), MetricName::LatencyMs) => (
            *next as f64 - *base as f64,
            *next > base.saturating_add(policy.max_latency_increase),
        ),
        (MetricValue::Count(base), MetricValue::Count(next), MetricName::FailedToolCalls) => (
            *next as f64 - *base as f64,
            *next > base.saturating_add(policy.max_failed_tool_calls_increase),
        ),
        _ => {
            return Err(BenchmarkComparisonError::InconsistentMetric {
                side: "comparison".into(),
                case_id: case_id.into(),
            })
        }
    };
    Ok(BenchmarkMetricDelta {
        case_id: case_id.into(),
        partition,
        metric,
        baseline: baseline_value,
        candidate: candidate_value,
        delta,
        regressed,
    })
}

fn metric_value(report: &BaselineReport, name: MetricName) -> Option<MetricValue> {
    report
        .metrics
        .iter()
        .find(|observation| observation.name == name)
        .map(|observation| observation.value.clone())
}

fn terminal_label(terminal: crate::evaluation::EvaluationTerminal) -> &'static str {
    match terminal {
        crate::evaluation::EvaluationTerminal::Pass => "pass",
        crate::evaluation::EvaluationTerminal::Fail => "fail",
        crate::evaluation::EvaluationTerminal::Blocked => "blocked",
        crate::evaluation::EvaluationTerminal::Cancelled => "cancelled",
        crate::evaluation::EvaluationTerminal::NoProof => "no_proof",
    }
}

fn validate_text(value: &str, max_bytes: usize) -> Result<(), BenchmarkComparisonError> {
    if value.trim().is_empty()
        || value.len() > max_bytes
        || value.chars().any(char::is_control)
        || [
            "-----begin",
            "api_key",
            "apikey",
            "authorization:",
            "chain_of_thought",
            "password",
            "prompt",
            "secret",
            "token=",
        ]
        .iter()
        .any(|marker| value.to_ascii_lowercase().contains(marker))
    {
        return Err(BenchmarkComparisonError::InvalidInput);
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(sha256_digest(&SHA256, bytes).as_ref())
}

fn decode_hex(value: &str, expected_bytes: usize) -> Result<Vec<u8>, BenchmarkComparisonError> {
    if value.len() != expected_bytes * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(BenchmarkComparisonError::InvalidReview);
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_digit(pair[0]).ok_or(BenchmarkComparisonError::InvalidReview)?;
            let low = hex_digit(pair[1]).ok_or(BenchmarkComparisonError::InvalidReview)?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
