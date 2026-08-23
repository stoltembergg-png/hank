use agent_core::{
    Memory, MemoryError, MemoryProvenance, MemoryStatus, MemoryType, ProjectId, ProvenanceSource,
};

fn memory(content: String) -> Memory {
    Memory::new_candidate(
        ProjectId::new(),
        content,
        MemoryType::Semantic,
        MemoryProvenance {
            source: ProvenanceSource::UserInput,
            extractor: None,
            confidence: 0.9,
            original_context: None,
        },
    )
}

// @spec:AC-731
#[test]
fn valid_memory_is_candidate_and_validates_with_bounded_content() {
    let item = memory("bounded fact".into());
    assert_eq!(item.status, MemoryStatus::Candidate);
    assert_eq!(item.version, 1);
    item.validate().unwrap();
}

// @spec:AC-732
#[test]
fn missing_or_oversized_content_and_invalid_confidence_fail_closed() {
    assert!(matches!(
        memory(String::new()).validate(),
        Err(MemoryError::ContentRequired)
    ));
    assert!(matches!(
        memory("x".repeat(16_385)).validate(),
        Err(MemoryError::ContentTooLarge)
    ));
    let mut invalid = memory("fact".into());
    invalid.provenance.confidence = f32::NAN;
    assert!(matches!(
        invalid.validate(),
        Err(MemoryError::InvalidConfidence)
    ));
}

// @spec:AC-733
#[test]
fn approval_archive_and_restore_are_versioned_and_deterministic() {
    let mut item = memory("fact".into());
    item.approve(0.8, Some("summary".into())).unwrap();
    assert_eq!(item.status, MemoryStatus::Approved);
    assert_eq!(item.version, 2);
    item.archive().unwrap();
    assert_eq!(item.status, MemoryStatus::Archived);
    assert_eq!(item.version, 3);
    item.restore().unwrap();
    assert_eq!(item.status, MemoryStatus::Approved);
    assert_eq!(item.version, 4);
}

// @spec:AC-734
#[test]
fn archived_memory_cannot_be_approved_without_restore() {
    let mut item = memory("fact".into());
    item.archive().unwrap();
    assert!(matches!(
        item.approve(0.5, None),
        Err(MemoryError::InvalidTransition)
    ));
}
