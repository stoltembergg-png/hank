use agent_core::improvement_candidate::*;

fn candidate() -> ImprovementCandidate {
    ImprovementCandidate::new(
        "candidate-1",
        "project-1",
        "owner-1",
        vec!["observation-1"],
        "policy-1",
        TargetKind::Skill,
        "proposal-digest-1",
        1,
        RiskClass::Low,
    )
    .unwrap()
}

// @spec:AC-1351
#[test]
fn candidate_requires_provenance_and_starts_draft() {
    let candidate = candidate();
    assert_eq!(candidate.status(), CandidateStatus::Draft);
    assert_eq!(candidate.version(), 1);
    assert_eq!(candidate.project_id(), "project-1");
    assert!(!candidate.can_activate());
    assert!(ImprovementCandidate::new(
        "",
        "project-1",
        "owner-1",
        vec![],
        "",
        TargetKind::Skill,
        "",
        0,
        RiskClass::Low
    )
    .is_err());
}

// @spec:AC-1352
#[test]
fn lifecycle_is_ordered_and_project_isolation_is_enforced() {
    let mut candidate = candidate();
    assert_eq!(
        candidate.authorize("project-2", "owner-1"),
        Err(CandidateError::Unauthorized)
    );
    assert_eq!(candidate.authorize("project-1", "owner-1"), Ok(()));
    assert_eq!(
        candidate.transition(CandidateStatus::Approved),
        Err(CandidateError::InvalidTransition)
    );
    candidate.transition(CandidateStatus::Evaluating).unwrap();
    candidate.transition(CandidateStatus::Rejected).unwrap();
    assert!(!candidate.can_activate());
}
