use agent_core::ids::ProjectId;
use tool_core::{
    PythonCapability, PythonPermissionDecision, PythonPermissionDenyReason, PythonPermissionPolicy,
    PythonPermissionRequest,
};

fn request(project: ProjectId, capability: PythonCapability) -> PythonPermissionRequest {
    PythonPermissionRequest {
        project_id: Some(project),
        requested_project_id: project,
        capability,
        declared: true,
        approved: true,
        budget_available: true,
        revoked: false,
    }
}

// @spec:AC-719 @spec:AC-720
#[test]
fn python_capability_matrix_allows_only_declared_approved_budgeted_access() {
    let project = ProjectId::new();
    assert_eq!(
        PythonPermissionPolicy::evaluate(request(project, PythonCapability::FilesystemRead)),
        PythonPermissionDecision::Allowed
    );
    let mut undeclared = request(project, PythonCapability::Network);
    undeclared.declared = false;
    assert_eq!(
        PythonPermissionPolicy::evaluate(undeclared),
        PythonPermissionDecision::Denied(PythonPermissionDenyReason::UndeclaredCapability)
    );
    let mut unapproved = request(project, PythonCapability::Process);
    unapproved.approved = false;
    assert_eq!(
        PythonPermissionPolicy::evaluate(unapproved),
        PythonPermissionDecision::Denied(PythonPermissionDenyReason::ApprovalRequired)
    );
}

// @spec:AC-721 @spec:AC-722
#[test]
fn python_policy_rejects_cross_project_revoke_and_budget_exhaustion() {
    let project = ProjectId::new();
    let other = ProjectId::new();
    let mut cross = request(project, PythonCapability::FilesystemWrite);
    cross.requested_project_id = other;
    assert_eq!(
        PythonPermissionPolicy::evaluate(cross),
        PythonPermissionDecision::Denied(PythonPermissionDenyReason::CrossProject)
    );
    let mut revoked = request(project, PythonCapability::PackageInstall);
    revoked.revoked = true;
    assert_eq!(
        PythonPermissionPolicy::evaluate(revoked),
        PythonPermissionDecision::Denied(PythonPermissionDenyReason::Revoked)
    );
}
