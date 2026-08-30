use agent_core::workflow_improvement_proposal::*;

fn valid() -> WorkflowProposalRequest {
    WorkflowProposalRequest::new(
        "workflow-1",
        "1.0.0",
        "2.0.0",
        "candidate-1",
        "policy-1",
        vec![("start", "finish")],
        vec!["start", "finish"],
        vec!["running", "completed"],
        false,
        false,
        false,
    )
    .unwrap()
}

// @spec:AC-1357
#[test]
fn valid_diff_has_stable_digest_and_preserves_previous_version() {
    let proposal = WorkflowImprovementProposal::create(valid()).unwrap();
    assert_eq!(proposal.active_version(), "1.0.0");
    assert_eq!(proposal.rollback_version(), "1.0.0");
    assert_eq!(
        proposal.fingerprint(),
        WorkflowImprovementProposal::create(valid())
            .unwrap()
            .fingerprint()
    );
    assert!(!proposal.can_activate());
}

// @spec:AC-1358
#[test]
fn cycle_privileged_node_state_break_and_budget_escalation_are_blocked() {
    let mut cycle = valid();
    cycle.edges.push(("finish", "start"));
    assert!(matches!(
        WorkflowImprovementProposal::create(cycle),
        Err(WorkflowProposalError::Cycle)
    ));
    let mut privileged = valid();
    privileged.privileged_node = true;
    assert!(matches!(
        WorkflowImprovementProposal::create(privileged),
        Err(WorkflowProposalError::CapabilityEscalation)
    ));
    let mut budget = valid();
    budget.budget_escalation = true;
    assert!(matches!(
        WorkflowImprovementProposal::create(budget),
        Err(WorkflowProposalError::PolicyRequired)
    ));
}
