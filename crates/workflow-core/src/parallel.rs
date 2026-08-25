//! Declarative, bounded ParallelNode planning and deterministic joins.

use std::collections::{HashMap, HashSet};
use thiserror::Error;

const MAX_FANOUT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinPolicy {
    All,
    Any,
    Quorum(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchResult {
    Success(String),
    Failed(String),
    Cancelled(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinDecision {
    Satisfied,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedBranches {
    pub ordered: Vec<BranchResult>,
    pub decision: JoinDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParallelError {
    #[error("parallel fan-out or concurrency limit is invalid")]
    InvalidLimit,
    #[error("parallel fan-out exceeds configured bound")]
    FanoutExceeded,
    #[error("parallel branch identifier is duplicated or empty")]
    DuplicateBranch,
    #[error("parallel join policy is invalid")]
    InvalidPolicy,
    #[error("parallel join results are incomplete or duplicated")]
    InvalidJoin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelPlan {
    branches: Vec<String>,
    concurrency_limit: usize,
    cancelled: bool,
}

impl ParallelPlan {
    pub fn new(
        branches: Vec<String>,
        max_fanout: usize,
        concurrency_limit: usize,
    ) -> Result<Self, ParallelError> {
        if branches.is_empty() || max_fanout == 0 || concurrency_limit == 0 {
            return Err(ParallelError::InvalidLimit);
        }
        if max_fanout > MAX_FANOUT || branches.len() > max_fanout {
            return Err(ParallelError::FanoutExceeded);
        }
        let mut seen = HashSet::with_capacity(branches.len());
        if branches
            .iter()
            .any(|branch| branch.trim().is_empty() || !seen.insert(branch))
        {
            return Err(ParallelError::DuplicateBranch);
        }
        Ok(Self {
            branches,
            concurrency_limit,
            cancelled: false,
        })
    }

    pub fn branches(&self) -> &[String] {
        &self.branches
    }
    pub fn concurrency_limit(&self) -> usize {
        self.concurrency_limit
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn cancel(&mut self) -> bool {
        if self.cancelled {
            return false;
        }
        self.cancelled = true;
        true
    }

    pub fn join(
        &self,
        policy: JoinPolicy,
        results: Vec<BranchResult>,
    ) -> Result<JoinedBranches, ParallelError> {
        let quorum = match policy {
            JoinPolicy::Quorum(value) if value == 0 || value > self.branches.len() => {
                return Err(ParallelError::InvalidPolicy)
            }
            JoinPolicy::Quorum(value) => value,
            _ => 0,
        };
        if results.len() != self.branches.len() {
            return Err(ParallelError::InvalidJoin);
        }
        let mut by_id = HashMap::with_capacity(results.len());
        for result in results {
            let id = result.id().to_string();
            if !self.branches.iter().any(|branch| branch == &id)
                || by_id.insert(id, result).is_some()
            {
                return Err(ParallelError::InvalidJoin);
            }
        }
        let ordered = self
            .branches
            .iter()
            .map(|id| by_id.remove(id).ok_or(ParallelError::InvalidJoin))
            .collect::<Result<Vec<_>, _>>()?;
        let successes = ordered
            .iter()
            .filter(|result| matches!(result, BranchResult::Success(_)))
            .count();
        let cancelled = self.cancelled
            || ordered
                .iter()
                .any(|result| matches!(result, BranchResult::Cancelled(_)));
        let decision = if cancelled {
            JoinDecision::Cancelled
        } else {
            match policy {
                JoinPolicy::All => {
                    if successes == self.branches.len() {
                        JoinDecision::Satisfied
                    } else {
                        JoinDecision::Failed
                    }
                }
                JoinPolicy::Any => {
                    if successes > 0 {
                        JoinDecision::Satisfied
                    } else {
                        JoinDecision::Failed
                    }
                }
                JoinPolicy::Quorum(_) => {
                    if successes >= quorum {
                        JoinDecision::Satisfied
                    } else {
                        JoinDecision::Failed
                    }
                }
            }
        };
        Ok(JoinedBranches { ordered, decision })
    }
}

impl BranchResult {
    fn id(&self) -> &str {
        match self {
            Self::Success(id) | Self::Failed(id) | Self::Cancelled(id) => id,
        }
    }
}
