//! Pure bounded fix-review planning. External task/worktree mutation stays outside.

use std::collections::BTreeSet;
use thiserror::Error;

const MAX_TEXT: usize = 256;
const MAX_CYCLE_CAP: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixReviewMapping {
    project: String,
    task: String,
    repository: String,
    worktree: String,
    branch: String,
    commit: String,
    tree: String,
    policy: String,
}
impl FixReviewMapping {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project: &str,
        task: &str,
        repository: &str,
        worktree: &str,
        branch: &str,
        commit: &str,
        tree: &str,
        policy: &str,
    ) -> Result<Self, FixReviewError> {
        let values = [
            project, task, repository, worktree, branch, commit, tree, policy,
        ];
        if values.iter().any(|value| {
            value.is_empty() || value.len() > MAX_TEXT || value.chars().any(char::is_control)
        }) {
            return Err(FixReviewError::InvalidValue);
        }
        Ok(Self {
            project: project.into(),
            task: task.into(),
            repository: repository.into(),
            worktree: worktree.into(),
            branch: branch.into(),
            commit: commit.into(),
            tree: tree.into(),
            policy: policy.into(),
        })
    }
    pub fn commit(&self) -> &str {
        &self.commit
    }
    pub fn tree(&self) -> &str {
        &self.tree
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFinding {
    id: String,
    summary: String,
    commit: String,
    tree: String,
    review: String,
}
impl ReviewFinding {
    pub fn blocker(
        id: &str,
        summary: &str,
        commit: &str,
        tree: &str,
        review: &str,
    ) -> Result<Self, FixReviewError> {
        let values = [id, summary, commit, tree, review];
        if values.iter().any(|value| {
            value.is_empty() || value.len() > MAX_TEXT || value.chars().any(char::is_control)
        }) {
            return Err(FixReviewError::InvalidValue);
        }
        Ok(Self {
            id: id.into(),
            summary: summary.into(),
            commit: commit.into(),
            tree: tree.into(),
            review: review.into(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixReviewState {
    CorrectionPlanned,
    Escalated,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FixReviewError {
    #[error("invalid fix-review value")]
    InvalidValue,
    #[error("review evidence is stale")]
    StaleEvidence,
    #[error("cycle cap is invalid")]
    InvalidCap,
    #[error("finding is not a blocker")]
    NotBlocker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionTask {
    mapping: FixReviewMapping,
    finding_id: String,
    supersedes_review: String,
    next_cycle: u32,
}
impl CorrectionTask {
    pub fn mapping(&self) -> &FixReviewMapping {
        &self.mapping
    }
    pub fn supersedes_review(&self) -> &str {
        &self.supersedes_review
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixReviewPlan {
    state: FixReviewState,
    task: Option<CorrectionTask>,
    next_cycle: u32,
    fingerprint: String,
}
impl FixReviewPlan {
    pub fn state(&self) -> FixReviewState {
        self.state
    }
    pub fn task(&self) -> Option<&CorrectionTask> {
        self.task.as_ref()
    }
    pub fn next_cycle(&self) -> u32 {
        self.next_cycle
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

pub struct FixReviewWorkflow;
impl FixReviewWorkflow {
    pub fn plan(
        mapping: &FixReviewMapping,
        finding: &ReviewFinding,
        cycle: u32,
        cap: u32,
    ) -> Result<FixReviewPlan, FixReviewError> {
        if cap == 0 || cap > MAX_CYCLE_CAP || cycle > cap {
            return Err(FixReviewError::InvalidCap);
        }
        if finding.commit != mapping.commit || finding.tree != mapping.tree {
            return Err(FixReviewError::StaleEvidence);
        }
        if cycle == cap {
            return Ok(FixReviewPlan {
                state: FixReviewState::Escalated,
                task: None,
                next_cycle: cycle,
                fingerprint: digest(&format!(
                    "{}:{}:{}:escalated",
                    finding.id, mapping.task, cycle
                )),
            });
        }
        let task = CorrectionTask {
            mapping: mapping.clone(),
            finding_id: finding.id.clone(),
            supersedes_review: finding.review.clone(),
            next_cycle: cycle + 1,
        };
        Ok(FixReviewPlan {
            state: FixReviewState::CorrectionPlanned,
            task: Some(task),
            next_cycle: cycle + 1,
            fingerprint: digest(&format!(
                "{}:{}:{}:{}",
                finding.id, mapping.task, cycle, finding.summary
            )),
        })
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

#[allow(dead_code)]
fn _bounded_names(names: &[&str]) -> bool {
    let mut set = BTreeSet::new();
    names.iter().all(|name| set.insert(*name))
}
