//! Pure issue handoff payload; no external publication.
const MAX: usize = 512;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueRequest {
    pub candidate_id: String,
    pub evidence_id: String,
    pub repository: String,
    pub sha: String,
    pub tree: String,
    pub policy: String,
    pub decision: String,
    pub risk: String,
    pub next_gate: String,
    pub policy_allowed: bool,
}
impl IssueRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        c: &str,
        e: &str,
        r: &str,
        s: &str,
        t: &str,
        p: &str,
        d: &str,
        risk: &str,
        next: &str,
        allowed: bool,
    ) -> Result<Self, IssueError> {
        if [c, e, r, s, t, p, d, next]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX)
        {
            return Err(IssueError::InvalidIdentity);
        }
        Ok(Self {
            candidate_id: c.into(),
            evidence_id: e.into(),
            repository: r.into(),
            sha: s.into(),
            tree: t.into(),
            policy: p.into(),
            decision: d.into(),
            risk: risk.into(),
            next_gate: next.into(),
            policy_allowed: allowed,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueError {
    InvalidIdentity,
    PolicyDenied,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuePayload {
    body: String,
    key: String,
    decision: String,
}
impl IssuePayload {
    pub fn create(r: IssueRequest) -> Result<Self, IssueError> {
        if !r.policy_allowed {
            return Err(IssueError::PolicyDenied);
        }
        let risk = redact(&r.risk);
        let body=format!("Candidate: {}\nEvidence: {}\nRepository: {}\nSHA: {}\nTree: {}\nDecision: [{}]\nRisk: {}\nNext gate: {}",r.candidate_id,r.evidence_id,r.repository,r.sha,r.tree,r.decision,risk,r.next_gate);
        let key = digest(&format!(
            "{}|{}|{}|{}",
            r.repository, r.candidate_id, r.evidence_id, r.sha
        ));
        Ok(Self {
            body,
            key,
            decision: r.decision,
        })
    }
    pub fn decision(&self) -> &str {
        &self.decision
    }
    pub fn body(&self) -> &str {
        &self.body
    }
    pub fn idempotency_key(&self) -> &str {
        &self.key
    }
}
fn redact(v: &str) -> String {
    let lower = v.to_ascii_lowercase();
    if lower.contains("token=") || lower.contains("secret=") || lower.contains("api_key=") {
        "[REDACTED]".into()
    } else {
        v.replace("ignore instructions", "[ESCAPED_TEXT]")
    }
}
fn digest(v: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in v.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3)
    }
    format!("{h:016x}")
}
