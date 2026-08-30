use agent_core::agent_configuration_proposal::*;

fn valid() -> ConfigProposalRequest {
    ConfigProposalRequest::new(
        "agent-1",
        "1.0.0",
        "candidate-1",
        "policy-1",
        vec![ConfigChange::new(ConfigField::Model, "model-a", "model-b")],
        false,
        false,
        false,
        false,
    )
    .unwrap()
}

// @spec:AC-1359
#[test]
fn valid_config_proposal_preserves_old_config_and_has_stable_digest() {
    let proposal = AgentConfigurationProposal::create(valid()).unwrap();
    assert_eq!(proposal.active_version(), "1.0.0");
    assert_eq!(proposal.precedence(), PrecedenceClass::Agent);
    assert_eq!(
        proposal.fingerprint(),
        AgentConfigurationProposal::create(valid())
            .unwrap()
            .fingerprint()
    );
    assert!(!proposal.can_activate());
}

// @spec:AC-1360
#[test]
fn forbidden_instruction_capability_autonomy_and_budget_changes_are_blocked() {
    let mut security = valid();
    security.changes[0] = ConfigChange::new(ConfigField::SecurityInstruction, "old", "new");
    assert!(matches!(
        AgentConfigurationProposal::create(security),
        Err(ConfigProposalError::ImmutablePolicy)
    ));
    let mut capability = valid();
    capability.capability_delta = true;
    assert!(matches!(
        AgentConfigurationProposal::create(capability),
        Err(ConfigProposalError::PolicyRequired)
    ));
    let mut autonomy = valid();
    autonomy.autonomy_delta = true;
    assert!(matches!(
        AgentConfigurationProposal::create(autonomy),
        Err(ConfigProposalError::PolicyRequired)
    ));
    let mut budget = valid();
    budget.budget_delta = true;
    assert!(matches!(
        AgentConfigurationProposal::create(budget),
        Err(ConfigProposalError::PolicyRequired)
    ));
}
