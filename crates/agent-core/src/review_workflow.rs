//! Deterministic, fail-closed review aggregation without external authority.

use std::collections::BTreeSet;
use thiserror::Error;

pub const MAX_REVIEW_EVIDENCE: usize = 8;
pub const MAX_REVIEW_FINDINGS: usize = 128;
pub const MAX_REVIEW_TEXT: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReviewError {
    #[error("invalid review value")]
    InvalidValue,
    #[error("review evidence is stale or has wrong identity")]
    StaleEvidence,
    #[error("review evidence is incomplete")]
    IncompleteEvidence,
    #[error("review input exceeds bounds")]
    BoundsExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReviewSource {
    Reviewer,
    Qa,
    Security,
    Architecture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Pass,
    Fail,
    Skipped,
    Cancelled,
    Missing,
    Stale,
    Malformed,
}

impl EvidenceStatus {
    fn valid(self) -> bool {
        matches!(self, Self::Pass | Self::Fail)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewContext {
    pub project_id: String,
    pub task_id: String,
    pub repository: String,
    pub worktree: String,
    pub branch: String,
    pub commit: String,
    pub tree: String,
    pub policy: String,
}

impl ReviewContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project: &str,
        task: &str,
        repo: &str,
        worktree: &str,
        branch: &str,
        commit: &str,
        tree: &str,
        policy: &str,
    ) -> Result<Self, ReviewError> {
        let values = [project, task, repo, worktree, branch, commit, tree, policy];
        if values
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_REVIEW_TEXT || v.chars().any(char::is_control))
        {
            return Err(ReviewError::InvalidValue);
        }
        Ok(Self {
            project_id: project.into(),
            task_id: task.into(),
            repository: repo.into(),
            worktree: worktree.into(),
            branch: branch.into(),
            commit: commit.into(),
            tree: tree.into(),
            policy: policy.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewEvidence {
    pub source: ReviewSource,
    pub status: EvidenceStatus,
    pub commit: String,
    pub tree: String,
    pub policy: String,
    pub digest: String,
}

impl ReviewEvidence {
    pub fn new(
        source: ReviewSource,
        status: EvidenceStatus,
        commit: &str,
        tree: &str,
        policy: &str,
        digest: &str,
    ) -> Result<Self, ReviewError> {
        let values = [commit, tree, policy, digest];
        if values
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_REVIEW_TEXT || v.chars().any(char::is_control))
        {
            return Err(ReviewError::InvalidValue);
        }
        Ok(Self {
            source,
            status,
            commit: commit.into(),
            tree: tree.into(),
            policy: policy.into(),
            digest: digest.into(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    Info,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFinding {
    pub severity: FindingSeverity,
    pub code: String,
    pub text: String,
}

impl ReviewFinding {
    pub fn new(severity: FindingSeverity, code: &str, text: &str) -> Result<Self, ReviewError> {
        if [code, text]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_REVIEW_TEXT || v.chars().any(char::is_control))
        {
            return Err(ReviewError::InvalidValue);
        }
        Ok(Self {
            severity,
            code: code.into(),
            text: text.into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReviewInput {
    pub context: ReviewContext,
    pub evidence: Vec<ReviewEvidence>,
    pub findings: Vec<ReviewFinding>,
}
impl ReviewInput {
    pub fn new(
        context: ReviewContext,
        evidence: Vec<ReviewEvidence>,
        findings: Vec<ReviewFinding>,
    ) -> Result<Self, ReviewError> {
        if evidence.len() > MAX_REVIEW_EVIDENCE || findings.len() > MAX_REVIEW_FINDINGS {
            return Err(ReviewError::BoundsExceeded);
        }
        Ok(Self {
            context,
            evidence,
            findings,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Advisory,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct ReviewReport {
    state: ReviewState,
    fingerprint: String,
    blockers: usize,
    unknown_evidence: bool,
}
impl ReviewReport {
    pub fn state(&self) -> ReviewState {
        self.state
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn blockers(&self) -> usize {
        self.blockers
    }
    pub fn unknown_evidence(&self) -> bool {
        self.unknown_evidence
    }
    pub fn can_mark_ready(&self) -> bool {
        false
    }
    pub fn can_approve(&self) -> bool {
        false
    }
    pub fn can_merge(&self) -> bool {
        false
    }
}

pub struct ReviewWorkflow;
impl ReviewWorkflow {
    pub fn evaluate(input: &ReviewInput) -> Result<ReviewReport, ReviewError> {
        let mut sources = BTreeSet::new();
        let mut unknown = false;
        let mut failed = false;
        for item in &input.evidence {
            if item.commit != input.context.commit
                || item.tree != input.context.tree
                || item.policy != input.context.policy
            {
                return Err(ReviewError::StaleEvidence);
            }
            if !item.status.valid() {
                unknown = true;
            }
            if item.status == EvidenceStatus::Fail {
                failed = true;
            }
            sources.insert(item.source);
        }
        let required = [
            ReviewSource::Reviewer,
            ReviewSource::Qa,
            ReviewSource::Security,
            ReviewSource::Architecture,
        ];
        if required.iter().any(|source| !sources.contains(source)) {
            unknown = true;
        }
        let blockers = input
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Blocker)
            .count();
        let mut digest_input = format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            input.context.project_id,
            input.context.task_id,
            input.context.repository,
            input.context.worktree,
            input.context.branch,
            input.context.commit,
            input.context.tree,
            input.context.policy
        );
        for item in &input.evidence {
            digest_input.push_str(&format!(
                "{:?}:{:?}:{}",
                item.source, item.status, item.digest
            ));
        }
        for finding in &input.findings {
            digest_input.push_str(&format!(
                "{:?}:{}:{}",
                finding.severity, finding.code, finding.text
            ));
        }
        let fingerprint = stable_digest(&digest_input);
        Ok(ReviewReport {
            state: if unknown || failed || blockers > 0 {
                ReviewState::Blocked
            } else {
                ReviewState::Advisory
            },
            fingerprint,
            blockers,
            unknown_evidence: unknown,
        })
    }
}

fn stable_digest(value: &str) -> String {
    let mut state = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}
