//! Deterministic improvement scoring with fail-closed blockers.
use thiserror::Error;
const MAX_TEXT: usize = 256;
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    pub quality: Option<f64>,
    pub security: Option<f64>,
    pub regression: Option<f64>,
    pub cost: Option<f64>,
}
impl Metrics {
    pub fn new(q: f64, s: f64, r: f64, c: f64) -> Self {
        Self {
            quality: Some(q),
            security: Some(s),
            regression: Some(r),
            cost: Some(c),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreRequest {
    pub policy_id: String,
    pub evidence_id: String,
    pub metrics: Metrics,
    pub security_failure: bool,
    pub regression_failure: bool,
    pub evidence_stale: bool,
}
impl ScoreRequest {
    pub fn new(policy: &str, evidence: &str, metrics: Metrics) -> Result<Self, ScoreError> {
        if [policy, evidence]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_TEXT)
        {
            return Err(ScoreError::InvalidPolicy);
        }
        Ok(Self {
            policy_id: policy.into(),
            evidence_id: evidence.into(),
            metrics,
            security_failure: false,
            regression_failure: false,
            evidence_stale: false,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreClass {
    Pass,
    Unknown,
    NoGo,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScoreError {
    #[error("score policy or evidence identity is invalid")]
    InvalidPolicy,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ImprovementScore {
    value: f64,
    class: ScoreClass,
    fingerprint: String,
}
impl ImprovementScore {
    pub fn calculate(request: ScoreRequest) -> Result<Self, ScoreError> {
        if request.security_failure || request.regression_failure || request.evidence_stale {
            return Ok(Self {
                value: 0.0,
                class: ScoreClass::NoGo,
                fingerprint: digest(&format!(
                    "{}|{}|blocked",
                    request.policy_id, request.evidence_id
                )),
            });
        }
        let Some((q, s, r, c)) = request
            .metrics
            .quality
            .zip(request.metrics.security)
            .zip(request.metrics.regression)
            .zip(request.metrics.cost)
            .map(|(((q, s), r), c)| (q, s, r, c))
        else {
            return Ok(Self {
                value: 0.0,
                class: ScoreClass::Unknown,
                fingerprint: digest(&format!(
                    "{}|{}|unknown",
                    request.policy_id, request.evidence_id
                )),
            });
        };
        let value = (q * 0.4 + s * 0.3 + r * 0.2 + c * 0.1).clamp(0.0, 1.0);
        let class = if value >= 0.8 {
            ScoreClass::Pass
        } else {
            ScoreClass::NoGo
        };
        Ok(Self {
            value,
            class,
            fingerprint: digest(&format!(
                "{}|{}|{q:.6}|{s:.6}|{r:.6}|{c:.6}",
                request.policy_id, request.evidence_id
            )),
        })
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn class(&self) -> ScoreClass {
        self.class
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn can_activate(&self) -> bool {
        false
    }
}
fn digest(v: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in v.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
