use agent_core::skill_improvement_proposal::*;

fn valid() -> ProposalRequest {
    ProposalRequest::new(
        "skill-1",
        "1.0.0",
        "candidate-1",
        "source-observation-1",
        "policy-1",
        vec![ChangedFile::new(
            "SKILL.md",
            FileChangeKind::Modified,
            "improve safely",
        )],
        vec!["skill:evaluate"],
        vec!["tests/basic.json"],
    )
}

// @spec:AC-1355
#[test]
fn proposal_is_versioned_stable_and_does_not_change_active_skill() {
    let proposal = SkillImprovementProposal::create(valid()).unwrap();
    assert_eq!(proposal.active_version(), "1.0.0");
    assert_eq!(proposal.status(), ProposalStatus::Draft);
    assert_eq!(
        proposal.fingerprint(),
        SkillImprovementProposal::create(valid())
            .unwrap()
            .fingerprint()
    );
    assert!(!proposal.can_activate());
}

// @spec:AC-1356
#[test]
fn unsafe_paths_and_secret_content_are_rejected() {
    let mut traversal = valid();
    traversal.files[0] = ChangedFile::new("../SKILL.md", FileChangeKind::Modified, "safe");
    assert!(matches!(
        SkillImprovementProposal::create(traversal),
        Err(ProposalError::UnsafePath)
    ));
    let mut secret = valid();
    secret.files[0] = ChangedFile::new("SKILL.md", FileChangeKind::Modified, "token=secret");
    assert!(matches!(
        SkillImprovementProposal::create(secret),
        Err(ProposalError::SecretLikeContent)
    ));
}
