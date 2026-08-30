//! Pure review-bound PR proposal; no GitHub calls or merge authority.
const MAX: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrRequest {
    pub candidate: String,
    pub issue: String,
    pub branch: String,
    pub base_sha: String,
    pub head_sha: String,
    pub tree: String,
    pub proposal_evidence: String,
    pub evaluation_evidence: String,
    pub regression_evidence: String,
    pub rollback_evidence: String,
}
impl PrRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        c: &str,
        i: &str,
        b: &str,
        base: &str,
        head: &str,
        tree: &str,
        p: &str,
        e: &str,
        r: &str,
        rb: &str,
    ) -> Result<Self, PrError> {
        if [c, i, b, base, head, tree]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX)
        {
            return Err(PrError::InvalidIdentity);
        }
        Ok(Self {
            candidate: c.into(),
            issue: i.into(),
            branch: b.into(),
            base_sha: base.into(),
            head_sha: head.into(),
            tree: tree.into(),
            proposal_evidence: p.into(),
            evaluation_evidence: e.into(),
            regression_evidence: r.into(),
            rollback_evidence: rb.into(),
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrError {
    InvalidIdentity,
    MissingEvidence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftStatus {
    Current,
    Stale,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrDraft {
    key: String,
    base_sha: String,
    head_sha: String,
    tree: String,
}
impl PrDraft {
    pub fn create(r: PrRequest) -> Result<Self, PrError> {
        if [
            r.proposal_evidence.as_str(),
            r.evaluation_evidence.as_str(),
            r.regression_evidence.as_str(),
            r.rollback_evidence.as_str(),
        ]
        .iter()
        .any(|v| v.is_empty() || v.len() > MAX)
        {
            return Err(PrError::MissingEvidence);
        }
        let key = digest(&format!(
            "{}|{}|{}|{}",
            r.candidate, r.issue, r.branch, r.head_sha
        ));
        Ok(Self {
            key,
            base_sha: r.base_sha,
            head_sha: r.head_sha,
            tree: r.tree,
        })
    }
    pub fn is_draft(&self) -> bool {
        true
    }
    pub fn review_required(&self) -> bool {
        true
    }
    pub fn approved(&self) -> bool {
        false
    }
    pub fn idempotency_key(&self) -> &str {
        &self.key
    }
    pub fn status(&self, head: &str, tree: &str) -> DraftStatus {
        if head == self.head_sha && tree == self.tree {
            DraftStatus::Current
        } else {
            DraftStatus::Stale
        }
    }
    pub fn base_sha(&self) -> &str {
        &self.base_sha
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
