//! Deterministic, read-only cycle preflight for invocation ancestry.

use crate::InvocationGraph;
use agent_protocol::InvocationRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleDecision {
    Pass,
    RejectSelfLoop,
    RejectAncestorCycle { path_len: usize },
    RejectGraphIncomplete,
}

pub struct CycleDetector;

impl CycleDetector {
    pub fn check(
        graph: &InvocationGraph,
        parent: Option<uuid::Uuid>,
        candidate: &InvocationRequest,
    ) -> CycleDecision {
        if candidate.caller_id == candidate.callee_id {
            return CycleDecision::RejectSelfLoop;
        }
        if candidate.validate().is_err() {
            return CycleDecision::RejectGraphIncomplete;
        }
        let mut current = parent;
        let mut path_len = 0;
        while let Some(id) = current {
            let Some(node) = graph.request(id) else {
                return CycleDecision::RejectGraphIncomplete;
            };
            if node.project_id != candidate.project_id {
                return CycleDecision::RejectGraphIncomplete;
            }
            path_len += 1;
            if node.caller_id == candidate.callee_id || node.callee_id == candidate.callee_id {
                return CycleDecision::RejectAncestorCycle { path_len };
            }
            current = graph.parent(id).flatten();
        }
        CycleDecision::Pass
    }
}
