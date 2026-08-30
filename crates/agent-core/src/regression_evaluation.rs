//! Fail-closed regression evidence; no rollout or test-selection side effects.
use thiserror::Error;

const MAX_TEXT: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactClass {
    Workflow,
    Skill,
    Security,
    Provider,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegressionRequest {
    pub corpus_id: String,
    pub corpus_revision: String,
    pub baseline_id: String,
    pub candidate_id: String,
    pub candidate_sha: String,
    pub impact: ImpactClass,
}
impl RegressionRequest {
    pub fn new(
        corpus: &str,
        revision: &str,
        baseline: &str,
        candidate: &str,
        sha: &str,
        impact: ImpactClass,
    ) -> Result<Self, RegressionError> {
        if [corpus, revision, baseline, candidate, sha]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_TEXT)
        {
            return Err(RegressionError::InvalidIdentity);
        }
        Ok(Self {
            corpus_id: corpus.into(),
            corpus_revision: revision.into(),
            baseline_id: baseline.into(),
            candidate_id: candidate.into(),
            candidate_sha: sha.into(),
            impact,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionOutcome {
    Passed,
    FixtureMissing,
    Skipped,
    NoRun,
    StaleIdentity,
    ClassifierUnknown,
    CriticalFailure,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionStatus {
    Pass,
    NoGo,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegressionError {
    #[error("regression identity is invalid")]
    InvalidIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegressionReport {
    status: RegressionStatus,
    fingerprint: String,
}
impl RegressionReport {
    pub fn evaluate(
        request: RegressionRequest,
        outcome: RegressionOutcome,
    ) -> Result<Self, RegressionError> {
        let status = match outcome {
            RegressionOutcome::Passed => RegressionStatus::Pass,
            _ => RegressionStatus::NoGo,
        };
        let material = format!(
            "{}|{}|{}|{}|{}|{:?}|{:?}",
            request.corpus_id,
            request.corpus_revision,
            request.baseline_id,
            request.candidate_id,
            request.candidate_sha,
            request.impact,
            outcome
        );
        Ok(Self {
            status,
            fingerprint: digest(&material),
        })
    }
    pub fn status(&self) -> RegressionStatus {
        self.status
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn can_activate(&self) -> bool {
        false
    }
}
fn digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
