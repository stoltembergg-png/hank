//! Deterministic, bounded workflow run coordination.

use crate::{WorkflowGraph, WorkflowNodeType};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const MAX_RUN_ID_BYTES: usize = 128;
const MAX_FAILURE_CODE_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRunState {
    Ready,
    InFlight,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExecutionError {
    #[error("workflow run identity is invalid")]
    InvalidIdentity,
    #[error("workflow graph is invalid for execution")]
    InvalidGraph,
    #[error("workflow run backpressure limit was reached")]
    Backpressure,
    #[error("workflow node is unknown")]
    UnknownNode,
    #[error("workflow node transition is invalid")]
    InvalidTransition,
    #[error("workflow node dispatch was duplicated")]
    DuplicateDispatch,
    #[error("workflow run is terminal")]
    Terminal,
    #[error("workflow node retry budget was exhausted")]
    RetryExhausted,
    #[error("workflow failure code is invalid")]
    InvalidFailureCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryEnvelope {
    pub node_id: String,
    pub attempt: u8,
    pub max_attempts: u8,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowRun {
    run_id: String,
    workflow_id: String,
    workflow_version: u32,
    state: RunState,
    max_in_flight: usize,
    in_flight: usize,
    nodes: BTreeMap<String, NodeRuntime>,
    predecessors: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone)]
struct NodeRuntime {
    state: NodeRunState,
    attempts: u8,
    max_attempts: u8,
    failure_code: Option<String>,
    _node_type: WorkflowNodeType,
}

impl WorkflowRun {
    pub fn start(
        run_id: impl Into<String>,
        graph: &WorkflowGraph,
        max_in_flight: usize,
    ) -> Result<Self, ExecutionError> {
        let run_id = run_id.into();
        if !valid_bounded_id(&run_id) || max_in_flight == 0 {
            return Err(if max_in_flight == 0 {
                ExecutionError::Backpressure
            } else {
                ExecutionError::InvalidIdentity
            });
        }
        graph.validate().map_err(|_| ExecutionError::InvalidGraph)?;
        if graph.nodes.is_empty() {
            return Err(ExecutionError::InvalidGraph);
        }

        let mut predecessors: BTreeMap<String, BTreeSet<String>> = graph
            .nodes
            .keys()
            .map(|node_id| (node_id.clone(), BTreeSet::new()))
            .collect();
        for edge in &graph.edges {
            predecessors
                .entry(edge.target_node.clone())
                .or_default()
                .insert(edge.source_node.clone());
        }
        let nodes = graph
            .nodes
            .values()
            .map(|node| {
                (
                    node.node_id.clone(),
                    NodeRuntime {
                        state: NodeRunState::Ready,
                        attempts: 0,
                        max_attempts: node.retry.max_attempts,
                        failure_code: None,
                        _node_type: node.node_type,
                    },
                )
            })
            .collect();
        Ok(Self {
            run_id,
            workflow_id: graph.workflow_id.clone(),
            workflow_version: graph.workflow_version,
            state: RunState::Running,
            max_in_flight,
            in_flight: 0,
            nodes,
            predecessors,
        })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    pub fn workflow_version(&self) -> u32 {
        self.workflow_version
    }

    pub fn state(&self) -> RunState {
        self.state
    }

    pub fn node_state(&self, node_id: &str) -> Option<NodeRunState> {
        self.nodes.get(node_id).map(|node| node.state)
    }

    pub fn node_failure_code(&self, node_id: &str) -> Option<&str> {
        self.nodes
            .get(node_id)
            .and_then(|node| node.failure_code.as_deref())
    }

    pub fn ready_nodes(&self) -> Vec<String> {
        if self.state != RunState::Running {
            return Vec::new();
        }
        self.nodes
            .iter()
            .filter(|(node_id, node)| {
                node.state == NodeRunState::Ready
                    && self
                        .predecessors
                        .get(*node_id)
                        .into_iter()
                        .flatten()
                        .all(|predecessor| {
                            self.nodes
                                .get(predecessor)
                                .is_some_and(|node| node.state == NodeRunState::Succeeded)
                        })
            })
            .map(|(node_id, _)| node_id.clone())
            .collect()
    }

    pub fn dispatch(&mut self, node_id: &str) -> Result<(), ExecutionError> {
        self.ensure_running()?;
        if self.in_flight >= self.max_in_flight {
            return Err(ExecutionError::Backpressure);
        }
        let node_state = self
            .nodes
            .get(node_id)
            .ok_or(ExecutionError::UnknownNode)?
            .state;
        if node_state == NodeRunState::InFlight {
            return Err(ExecutionError::DuplicateDispatch);
        }
        if node_state != NodeRunState::Ready
            || !self
                .predecessors
                .get(node_id)
                .into_iter()
                .flatten()
                .all(|predecessor| {
                    self.nodes
                        .get(predecessor)
                        .is_some_and(|node| node.state == NodeRunState::Succeeded)
                })
        {
            return Err(ExecutionError::InvalidTransition);
        }
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ExecutionError::UnknownNode)?;
        node.state = NodeRunState::InFlight;
        node.attempts = node.attempts.saturating_add(1);
        self.in_flight += 1;
        Ok(())
    }

    pub fn complete(&mut self, node_id: &str) -> Result<(), ExecutionError> {
        self.ensure_running()?;
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ExecutionError::UnknownNode)?;
        if node.state != NodeRunState::InFlight {
            return Err(ExecutionError::InvalidTransition);
        }
        node.state = NodeRunState::Succeeded;
        self.in_flight = self.in_flight.saturating_sub(1);
        if self
            .nodes
            .values()
            .all(|node| node.state == NodeRunState::Succeeded)
        {
            self.state = RunState::Completed;
        }
        Ok(())
    }

    pub fn fail(&mut self, node_id: &str, reason: &str) -> Result<(), ExecutionError> {
        self.ensure_running()?;
        let reason = sanitize_failure_code(reason)?;
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ExecutionError::UnknownNode)?;
        if node.state != NodeRunState::InFlight {
            return Err(ExecutionError::InvalidTransition);
        }
        node.state = NodeRunState::Failed;
        node.failure_code = Some(reason);
        self.in_flight = self.in_flight.saturating_sub(1);
        self.state = RunState::Failed;
        Ok(())
    }

    pub fn retry(&mut self, node_id: &str, reason: &str) -> Result<RetryEnvelope, ExecutionError> {
        self.ensure_running()?;
        let reason = sanitize_failure_code(reason)?;
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or(ExecutionError::UnknownNode)?;
        if node.state != NodeRunState::InFlight {
            return Err(ExecutionError::InvalidTransition);
        }
        if node.attempts >= node.max_attempts {
            node.state = NodeRunState::Failed;
            self.in_flight = self.in_flight.saturating_sub(1);
            self.state = RunState::Failed;
            return Err(ExecutionError::RetryExhausted);
        }
        node.state = NodeRunState::Ready;
        self.in_flight = self.in_flight.saturating_sub(1);
        Ok(RetryEnvelope {
            node_id: node_id.to_string(),
            attempt: node.attempts.saturating_add(1),
            max_attempts: node.max_attempts,
            reason,
        })
    }

    pub fn cancel(&mut self) -> Result<(), ExecutionError> {
        self.ensure_running()?;
        for node in self.nodes.values_mut() {
            if !matches!(node.state, NodeRunState::Succeeded | NodeRunState::Failed) {
                node.state = NodeRunState::Cancelled;
            }
        }
        self.in_flight = 0;
        self.state = RunState::Cancelled;
        Ok(())
    }

    fn ensure_running(&self) -> Result<(), ExecutionError> {
        (self.state == RunState::Running)
            .then_some(())
            .ok_or(ExecutionError::Terminal)
    }
}

fn valid_bounded_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_RUN_ID_BYTES
        && value.chars().all(|character| !character.is_control())
}

fn sanitize_failure_code(value: &str) -> Result<String, ExecutionError> {
    if value.trim().is_empty()
        || value.len() > MAX_FAILURE_CODE_BYTES
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err(ExecutionError::InvalidFailureCode);
    }
    Ok(value.to_string())
}
