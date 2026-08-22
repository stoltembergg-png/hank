//! Python-specific deny-default capability policy layered on the common evaluator.

use agent_core::ids::ProjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PythonCapability {
    FilesystemRead,
    FilesystemWrite,
    Network,
    Process,
    PackageInstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonPermissionRequest {
    pub project_id: Option<ProjectId>,
    pub requested_project_id: ProjectId,
    pub capability: PythonCapability,
    pub declared: bool,
    pub approved: bool,
    pub budget_available: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonPermissionDecision {
    Allowed,
    Denied(PythonPermissionDenyReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonPermissionDenyReason {
    MissingProject,
    CrossProject,
    UndeclaredCapability,
    ApprovalRequired,
    BudgetUnavailable,
    Revoked,
}

#[derive(Debug, Default)]
pub struct PythonPermissionPolicy;

impl PythonPermissionPolicy {
    pub fn evaluate(request: PythonPermissionRequest) -> PythonPermissionDecision {
        let Some(project_id) = request.project_id else {
            return PythonPermissionDecision::Denied(PythonPermissionDenyReason::MissingProject);
        };
        if project_id != request.requested_project_id {
            return PythonPermissionDecision::Denied(PythonPermissionDenyReason::CrossProject);
        }
        if request.revoked {
            return PythonPermissionDecision::Denied(PythonPermissionDenyReason::Revoked);
        }
        if !request.declared {
            return PythonPermissionDecision::Denied(
                PythonPermissionDenyReason::UndeclaredCapability,
            );
        }
        if !request.budget_available {
            return PythonPermissionDecision::Denied(PythonPermissionDenyReason::BudgetUnavailable);
        }
        if !request.approved {
            return PythonPermissionDecision::Denied(PythonPermissionDenyReason::ApprovalRequired);
        }
        PythonPermissionDecision::Allowed
    }
}
