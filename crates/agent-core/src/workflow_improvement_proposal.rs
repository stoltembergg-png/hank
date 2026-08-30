//! Proposal-only workflow graph diff; the active workflow is never mutated.

use std::collections::{HashMap, HashSet};
use thiserror::Error;

const MAX_TEXT: usize = 256;
const MAX_NODES: usize = 128;
const MAX_EDGES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowProposalRequest {
    pub workflow_id: String,
    pub active_version: String,
    pub proposed_version: String,
    pub candidate_id: String,
    pub policy_id: String,
    pub edges: Vec<(&'static str, &'static str)>,
    pub nodes: Vec<&'static str>,
    pub states: Vec<&'static str>,
    pub privileged_node: bool,
    pub state_break: bool,
    pub budget_escalation: bool,
}
impl WorkflowProposalRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workflow: &str,
        active: &str,
        proposed: &str,
        candidate: &str,
        policy: &str,
        edges: Vec<(&'static str, &'static str)>,
        nodes: Vec<&'static str>,
        states: Vec<&'static str>,
        privileged: bool,
        state_break: bool,
        budget: bool,
    ) -> Result<Self, WorkflowProposalError> {
        if [workflow, active, proposed, candidate, policy]
            .iter()
            .any(|v| v.is_empty() || v.len() > MAX_TEXT)
        {
            return Err(WorkflowProposalError::InvalidIdentity);
        }
        Ok(Self {
            workflow_id: workflow.into(),
            active_version: active.into(),
            proposed_version: proposed.into(),
            candidate_id: candidate.into(),
            policy_id: policy.into(),
            edges,
            nodes,
            states,
            privileged_node: privileged,
            state_break,
            budget_escalation: budget,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowProposalError {
    #[error("workflow proposal identity is invalid")]
    InvalidIdentity,
    #[error("workflow proposal graph is oversized")]
    GraphTooLarge,
    #[error("workflow proposal contains a cycle")]
    Cycle,
    #[error("workflow proposal introduces a privileged capability")]
    CapabilityEscalation,
    #[error("workflow proposal requires explicit policy approval")]
    PolicyRequired,
    #[error("workflow proposal breaks state compatibility")]
    StateIncompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowImprovementProposal {
    active_version: String,
    rollback_version: String,
    fingerprint: String,
}
impl WorkflowImprovementProposal {
    pub fn create(request: WorkflowProposalRequest) -> Result<Self, WorkflowProposalError> {
        if request.nodes.is_empty()
            || request.nodes.len() > MAX_NODES
            || request.edges.len() > MAX_EDGES
        {
            return Err(WorkflowProposalError::GraphTooLarge);
        }
        if request.privileged_node {
            return Err(WorkflowProposalError::CapabilityEscalation);
        }
        if request.budget_escalation {
            return Err(WorkflowProposalError::PolicyRequired);
        }
        if request.state_break {
            return Err(WorkflowProposalError::StateIncompatible);
        }
        let node_set: HashSet<&str> = request.nodes.iter().copied().collect();
        let mut indegree: HashMap<&str, usize> = node_set.iter().map(|node| (*node, 0)).collect();
        let mut outgoing: HashMap<&str, Vec<&str>> =
            node_set.iter().map(|node| (*node, Vec::new())).collect();
        for (from, to) in &request.edges {
            if !node_set.contains(from) || !node_set.contains(to) {
                return Err(WorkflowProposalError::GraphTooLarge);
            }
            *indegree.get_mut(to).unwrap() += 1;
            outgoing.get_mut(from).unwrap().push(to);
        }
        let mut queue: Vec<&str> = indegree
            .iter()
            .filter_map(|(node, degree)| (*degree == 0).then_some(*node))
            .collect();
        let mut visited = 0;
        while let Some(node) = queue.pop() {
            visited += 1;
            for next in &outgoing[node] {
                let degree = indegree.get_mut(next).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push(next);
                }
            }
        }
        if visited != node_set.len() {
            return Err(WorkflowProposalError::Cycle);
        }
        let material = format!(
            "{}|{}|{}|{}|{}|{:?}|{:?}|{:?}",
            request.workflow_id,
            request.active_version,
            request.proposed_version,
            request.candidate_id,
            request.policy_id,
            request.edges,
            request.nodes,
            request.states
        );
        Ok(Self {
            active_version: request.active_version.clone(),
            rollback_version: request.active_version,
            fingerprint: digest(&material),
        })
    }
    pub fn active_version(&self) -> &str {
        &self.active_version
    }
    pub fn rollback_version(&self) -> &str {
        &self.rollback_version
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
