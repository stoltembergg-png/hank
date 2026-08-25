//! Bounded parallel invocation planning without worker/provider execution.

use crate::{CycleDecision, DepthDecision, InvocationGraph};
use agent_protocol::InvocationRequest;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParallelBatchError {
    #[error("parallel batch fan-out or concurrency limit is invalid")]
    InvalidLimit,
    #[error("parallel child preflight was rejected")]
    PreflightRejected,
    #[error("parallel child invocation is duplicate")]
    Duplicate,
    #[error("parallel child project scope mismatches parent")]
    ScopeMismatch,
    #[error("parallel parent graph is incomplete")]
    GraphIncomplete,
    #[error("parallel join contains an invalid or duplicate child")]
    InvalidJoin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParallelChildOutcome {
    Completed(uuid::Uuid),
    Failed(uuid::Uuid),
    Cancelled(uuid::Uuid),
}

#[derive(Debug)]
pub struct ParallelBatch {
    children: Vec<InvocationRequest>,
    concurrency_limit: usize,
    cancelled: bool,
}

impl ParallelBatch {
    pub fn prepare(
        graph: &InvocationGraph,
        parent: Option<uuid::Uuid>,
        candidates: Vec<(InvocationRequest, CycleDecision, DepthDecision)>,
        max_fanout: usize,
        concurrency_limit: usize,
    ) -> Result<Self, ParallelBatchError> {
        if max_fanout == 0 || concurrency_limit == 0 || candidates.len() > max_fanout {
            return Err(ParallelBatchError::InvalidLimit);
        }
        let parent_project = parent
            .map(|id| graph.request(id).ok_or(ParallelBatchError::GraphIncomplete))
            .transpose()?
            .map(|request| request.project_id);
        let mut seen = HashSet::new();
        let mut children = Vec::with_capacity(candidates.len());
        for (request, cycle, depth) in candidates {
            if request.validate().is_err()
                || cycle != CycleDecision::Pass
                || !matches!(depth, DepthDecision::Pass { .. })
            {
                return Err(ParallelBatchError::PreflightRejected);
            }
            if !seen.insert(request.invocation_id) {
                return Err(ParallelBatchError::Duplicate);
            }
            if parent_project.is_some_and(|project| project != request.project_id) {
                return Err(ParallelBatchError::ScopeMismatch);
            }
            children.push(request);
        }
        Ok(Self {
            children,
            concurrency_limit,
            cancelled: false,
        })
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
    pub fn concurrency_limit(&self) -> usize {
        self.concurrency_limit
    }
    pub fn ids(&self) -> Vec<uuid::Uuid> {
        self.children
            .iter()
            .map(|child| child.invocation_id)
            .collect()
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
        outcomes: Vec<ParallelChildOutcome>,
    ) -> Result<Vec<ParallelChildOutcome>, ParallelBatchError> {
        if outcomes.len() != self.children.len() {
            return Err(ParallelBatchError::InvalidJoin);
        }
        let mut by_id = HashMap::new();
        for outcome in outcomes {
            let id = outcome_id(&outcome);
            if by_id.insert(id, outcome).is_some() {
                return Err(ParallelBatchError::InvalidJoin);
            }
        }
        self.children
            .iter()
            .map(|child| {
                by_id
                    .remove(&child.invocation_id)
                    .ok_or(ParallelBatchError::InvalidJoin)
            })
            .collect()
    }
}

fn outcome_id(outcome: &ParallelChildOutcome) -> uuid::Uuid {
    match outcome {
        ParallelChildOutcome::Completed(id)
        | ParallelChildOutcome::Failed(id)
        | ParallelChildOutcome::Cancelled(id) => *id,
    }
}
