use agent_core::{DedupeDecision, DedupeEntry, DedupeIndex, DedupeInput, MemoryKind, ProjectId};

fn input(project_id: ProjectId, content: &str) -> DedupeInput {
    DedupeInput {
        project_id,
        agent_id: None,
        kind: MemoryKind::Fact,
        canonical_key: "claim-1".into(),
        content: content.into(),
        trace_id: "trace-1".into(),
    }
}

// @spec:AC-749
#[test]
fn exact_duplicate_is_deterministic_and_scoped() {
    let project = ProjectId::new();
    let mut index = DedupeIndex::default();
    let existing = DedupeEntry::from_input("mem-1".into(), &input(project, "A fact"));
    index.commit(existing.clone()).unwrap();
    assert_eq!(
        index.decide(&input(project, "  a   FACT ")).unwrap(),
        DedupeDecision::Duplicate {
            existing_id: "mem-1".into()
        }
    );
    assert_eq!(
        index.decide(&input(ProjectId::new(), "A fact")).unwrap(),
        DedupeDecision::New
    );
}

// @spec:AC-750
#[test]
fn conflicting_same_claim_remains_reviewable() {
    let project = ProjectId::new();
    let mut index = DedupeIndex::default();
    index
        .commit(DedupeEntry::from_input(
            "mem-1".into(),
            &input(project, "fact A"),
        ))
        .unwrap();
    assert_eq!(
        index.decide(&input(project, "fact B")).unwrap(),
        DedupeDecision::Conflict {
            existing_id: "mem-1".into()
        }
    );
}

// @spec:AC-751
#[test]
fn retry_is_idempotent_and_rollback_restores_index() {
    let project = ProjectId::new();
    let mut index = DedupeIndex::default();
    let entry = DedupeEntry::from_input("mem-1".into(), &input(project, "fact"));
    index.commit(entry).unwrap();
    assert!(index
        .commit(DedupeEntry::from_input(
            "mem-1".into(),
            &input(project, "fact")
        ))
        .is_err());
    index.rollback("mem-1").unwrap();
    assert_eq!(
        index.decide(&input(project, "fact")).unwrap(),
        DedupeDecision::New
    );
}

// @spec:AC-752
#[test]
fn input_is_bounded_and_cross_scope_never_matches() {
    let project = ProjectId::new();
    let mut index = DedupeIndex::default();
    assert!(index
        .commit(DedupeEntry::from_input(
            "mem-1".into(),
            &input(project, &"x".repeat(16_385))
        ))
        .is_err());
}
