//! Deterministic, bounded invocation depth preflight.

use crate::InvocationGraph;
use agent_protocol::InvocationRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthDecision {
    Pass { depth: u16 },
    RejectDepthLimit,
    RejectDepthMismatch,
    RejectGraphIncomplete,
}

pub struct DepthLimiter;

impl DepthLimiter {
    pub fn check(
        graph: &InvocationGraph,
        parent: Option<uuid::Uuid>,
        candidate: &InvocationRequest,
        max_depth: u16,
    ) -> DepthDecision {
        if max_depth == 0 || candidate.validate().is_err() {
            return DepthDecision::RejectDepthLimit;
        }
        let mut current = parent;
        let mut depth = 0u16;
        while let Some(id) = current {
            let Some(node) = graph.request(id) else {
                return DepthDecision::RejectGraphIncomplete;
            };
            if node.project_id != candidate.project_id {
                return DepthDecision::RejectGraphIncomplete;
            }
            depth = match depth.checked_add(1) {
                Some(value) => value,
                None => return DepthDecision::RejectDepthLimit,
            };
            current = graph.parent(id).flatten();
        }
        if candidate.depth > max_depth {
            return DepthDecision::RejectDepthLimit;
        }
        if candidate.depth != depth {
            return DepthDecision::RejectDepthMismatch;
        }
        if depth > max_depth {
            return DepthDecision::RejectDepthLimit;
        }
        DepthDecision::Pass { depth }
    }
}
