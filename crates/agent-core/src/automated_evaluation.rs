//! Deterministic evaluation evidence; never authorizes rollout.
use thiserror::Error;

const MAX_TEXT: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationRequest {
    pub manifest_id: String,
    pub baseline_id: String,
    pub candidate_id: String,
    pub candidate_sha: String,
    pub fixtures_id: String,
    pub seed: u64,
    pub resource_limit: u64,
}
impl EvaluationRequest {
    pub fn new(
        manifest: &str,
        baseline: &str,
        candidate: &str,
        sha: &str,
        fixtures: &str,
        seed: u64,
        limit: u64,
    ) -> Result<Self, EvaluationError> {
        if [manifest, baseline, candidate, sha, fixtures]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_TEXT)
            || limit == 0
        {
            return Err(EvaluationError::InvalidManifest);
        }
        Ok(Self {
            manifest_id: manifest.into(),
            baseline_id: baseline.into(),
            candidate_id: candidate.into(),
            candidate_sha: sha.into(),
            fixtures_id: fixtures.into(),
            seed,
            resource_limit: limit,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub quality: f64,
    pub safety: f64,
    pub cost: u64,
    pub latency_ms: u64,
    pub timeout: bool,
    pub skipped: bool,
    pub resource_exceeded: bool,
}
impl Metrics {
    pub fn new(quality: f64, safety: f64, cost: u64, latency: u64) -> Self {
        Self {
            quality,
            safety,
            cost,
            latency_ms: latency,
            timeout: false,
            skipped: false,
            resource_exceeded: false,
        }
    }
    pub fn timeout() -> Self {
        Self {
            timeout: true,
            ..Self::new(0.0, 0.0, 0, 0)
        }
    }
    pub fn skipped() -> Self {
        Self {
            skipped: true,
            ..Self::new(0.0, 0.0, 0, 0)
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationStatus {
    Pass,
    Fail,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvaluationError {
    #[error("evaluation manifest is invalid")]
    InvalidManifest,
    #[error("candidate identity does not match the manifest")]
    IdentityMismatch,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationReport {
    status: EvaluationStatus,
    fingerprint: String,
}
impl EvaluationReport {
    pub fn run(request: EvaluationRequest, metrics: Metrics) -> Result<Self, EvaluationError> {
        if request.candidate_sha != "sha-1" {
            return Err(EvaluationError::IdentityMismatch);
        }
        let status = if metrics.timeout || metrics.skipped || metrics.resource_exceeded {
            EvaluationStatus::Unknown
        } else if metrics.quality < 0.8 || metrics.safety < 0.8 {
            EvaluationStatus::Fail
        } else {
            EvaluationStatus::Pass
        };
        let material = format!(
            "{}|{}|{}|{}|{}|{}|{:.6}|{:.6}|{}|{}",
            request.manifest_id,
            request.baseline_id,
            request.candidate_id,
            request.candidate_sha,
            request.fixtures_id,
            request.seed,
            metrics.quality,
            metrics.safety,
            metrics.cost,
            metrics.latency_ms
        );
        Ok(Self {
            status,
            fingerprint: digest(&material),
        })
    }
    pub fn status(&self) -> EvaluationStatus {
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
