use agent_core::claim_evidence::{
    Claim, ClaimClass, ClaimEvidenceError, ClaimEvidenceKind, ClaimResolution, EvidenceRecord,
    EvidenceScope, EvidenceStatus, FactState, ResolutionOutcome, CLAIM_EVIDENCE_SCHEMA_VERSION,
    MAX_CLAIM_EVIDENCE_REFERENCES, MAX_EVIDENCE_RECORDS, MAX_REQUIRED_EVIDENCE,
};
use agent_core::{ProjectId, RunId, TraceId};

fn identity(
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    head_sha: &str,
    tree_sha: &str,
) -> EvidenceScope {
    EvidenceScope::new(
        project_id,
        run_id,
        trace_id,
        digest('9'),
        Some(head_sha.to_owned()),
        Some(tree_sha.to_owned()),
        Some("policy-v1".to_owned()),
        Some("claim-evidence-v1".to_owned()),
    )
    .unwrap()
}

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn claim(scope: &EvidenceScope) -> Claim {
    Claim::new(
        scope.project_id,
        scope.run_id,
        scope.trace_id,
        "claim-1",
        ClaimClass::PlanFinding,
        digest('c'),
        vec![ClaimEvidenceKind::Commit, ClaimEvidenceKind::Tree],
        scope.clone(),
    )
    .unwrap()
}

fn evidence(
    claim: &Claim,
    scope: &EvidenceScope,
    evidence_id: &str,
    kind: ClaimEvidenceKind,
    status: EvidenceStatus,
    evidence_digest: char,
) -> EvidenceRecord {
    EvidenceRecord::new(
        claim.claim_id(),
        evidence_id,
        kind,
        scope.clone(),
        digest(evidence_digest),
        "resolver-git-v1",
        status,
    )
    .unwrap()
}

#[test]
// @spec:AC-1410
fn claim_starts_without_proof_and_verification_requires_exact_bounded_evidence() {
    let project_id = ProjectId::new();
    let run_id = RunId::new();
    let trace_id = TraceId::new();
    let scope = identity(
        project_id,
        run_id,
        trace_id,
        &digest('1')[..40],
        &digest('2')[..40],
    );
    let mut claim = claim(&scope);

    assert_eq!(claim.state(), FactState::NoProof);
    assert_eq!(claim.evidence_ids(), &[] as &[String]);

    assert_eq!(
        claim
            .apply_resolution(ClaimResolution::verified(Vec::new()), &[])
            .unwrap_err(),
        ClaimEvidenceError::MissingEvidence
    );

    let wrong_tree = EvidenceScope::new(
        project_id,
        run_id,
        trace_id,
        scope.identity_digest().to_owned(),
        scope.head_sha().map(str::to_owned),
        Some(digest('3')),
        scope.policy_revision().map(str::to_owned),
        scope.schema_revision().map(str::to_owned),
    )
    .unwrap();
    let commit = evidence(
        &claim,
        &scope,
        "evidence-commit",
        ClaimEvidenceKind::Commit,
        EvidenceStatus::Verified,
        'a',
    );
    let tree = evidence(
        &claim,
        &wrong_tree,
        "evidence-tree",
        ClaimEvidenceKind::Tree,
        EvidenceStatus::Verified,
        'b',
    );
    assert_eq!(
        claim
            .apply_resolution(
                ClaimResolution::verified(vec!["evidence-commit".into(), "evidence-tree".into(),]),
                &[commit.clone(), tree],
            )
            .unwrap_err(),
        ClaimEvidenceError::IdentityMismatch
    );

    let tree = evidence(
        &claim,
        &scope,
        "evidence-tree",
        ClaimEvidenceKind::Tree,
        EvidenceStatus::Verified,
        'b',
    );
    assert_eq!(
        claim
            .apply_resolution(
                ClaimResolution::verified(vec!["evidence-commit".into(), "evidence-tree".into(),]),
                &[commit, tree],
            )
            .unwrap(),
        ResolutionOutcome::Applied
    );
    assert_eq!(claim.state(), FactState::Verified);
    assert_eq!(
        claim.evidence_ids(),
        &["evidence-commit".to_owned(), "evidence-tree".to_owned()]
    );
}

#[test]
// @spec:AC-1411
fn state_machine_rejects_unsafe_downgrade_and_replays_are_idempotent() {
    let project_id = ProjectId::new();
    let run_id = RunId::new();
    let trace_id = TraceId::new();
    let scope = identity(
        project_id,
        run_id,
        trace_id,
        &digest('1')[..40],
        &digest('2')[..40],
    );
    let mut claim = claim(&scope);

    assert!(FactState::NoProof.can_transition_to(FactState::Unverified));
    assert!(!FactState::Verified.can_transition_to(FactState::Unverified));
    assert_eq!(
        claim
            .apply_resolution(ClaimResolution::no_proof(), &[])
            .unwrap(),
        ResolutionOutcome::Idempotent
    );
    assert_eq!(
        claim
            .apply_resolution(ClaimResolution::unverified(Vec::new()), &[])
            .unwrap(),
        ResolutionOutcome::Applied
    );

    let commit = evidence(
        &claim,
        &scope,
        "evidence-commit",
        ClaimEvidenceKind::Commit,
        EvidenceStatus::Verified,
        'a',
    );
    let tree = evidence(
        &claim,
        &scope,
        "evidence-tree",
        ClaimEvidenceKind::Tree,
        EvidenceStatus::Verified,
        'b',
    );
    let resolution =
        ClaimResolution::verified(vec!["evidence-commit".into(), "evidence-tree".into()]);
    assert_eq!(
        claim
            .apply_resolution(resolution.clone(), &[commit.clone(), tree.clone()])
            .unwrap(),
        ResolutionOutcome::Applied
    );
    assert_eq!(
        claim.apply_resolution(resolution, &[commit, tree]).unwrap(),
        ResolutionOutcome::Idempotent
    );
    assert_eq!(
        claim
            .apply_resolution(ClaimResolution::unverified(Vec::new()), &[])
            .unwrap_err(),
        ClaimEvidenceError::InvalidTransition {
            from: FactState::Verified,
            to: FactState::Unverified,
        }
    );
}

#[test]
// @spec:AC-1412
fn missing_stale_and_conflicting_evidence_never_promote_a_claim() {
    let project_id = ProjectId::new();
    let run_id = RunId::new();
    let trace_id = TraceId::new();
    let scope = identity(
        project_id,
        run_id,
        trace_id,
        &digest('1')[..40],
        &digest('2')[..40],
    );
    let mut claim = claim(&scope);

    let only_commit = evidence(
        &claim,
        &scope,
        "evidence-commit",
        ClaimEvidenceKind::Commit,
        EvidenceStatus::Verified,
        'a',
    );
    assert_eq!(
        claim
            .apply_resolution(
                ClaimResolution::verified(vec!["evidence-commit".into()]),
                &[only_commit],
            )
            .unwrap_err(),
        ClaimEvidenceError::MissingRequiredEvidence
    );

    let stale = evidence(
        &claim,
        &scope,
        "evidence-stale",
        ClaimEvidenceKind::Tree,
        EvidenceStatus::Stale,
        'b',
    );
    assert_eq!(
        claim
            .apply_resolution(
                ClaimResolution::stale(vec!["evidence-stale".into()]),
                &[stale],
            )
            .unwrap(),
        ResolutionOutcome::Applied
    );
    assert_eq!(claim.state(), FactState::Stale);

    let conflict = evidence(
        &claim,
        &scope,
        "evidence-conflict",
        ClaimEvidenceKind::Tree,
        EvidenceStatus::Conflicting,
        'd',
    );
    assert_eq!(
        claim
            .apply_resolution(
                ClaimResolution::conflicting(vec!["evidence-conflict".into()]),
                &[conflict],
            )
            .unwrap(),
        ResolutionOutcome::Applied
    );
    assert_eq!(claim.state(), FactState::Conflicting);

    let no_proof = ClaimResolution::no_proof();
    assert_eq!(
        claim.apply_resolution(no_proof, &[]).unwrap(),
        ResolutionOutcome::Applied
    );
    assert_eq!(claim.state(), FactState::NoProof);
}

#[test]
// @spec:AC-1413
fn wire_contract_is_versioned_unknown_fields_fail_closed_and_claim_text_is_not_a_fact() {
    let project_id = ProjectId::new();
    let run_id = RunId::new();
    let trace_id = TraceId::new();
    let scope = identity(
        project_id,
        run_id,
        trace_id,
        &digest('1')[..40],
        &digest('2')[..40],
    );
    let claim = claim(&scope);
    let encoded = serde_json::to_value(&claim).unwrap();
    assert_eq!(encoded["schema_version"], CLAIM_EVIDENCE_SCHEMA_VERSION);
    assert!(encoded.get("claim_text").is_none());
    assert_eq!(
        serde_json::from_value::<Claim>(encoded.clone()).unwrap(),
        claim
    );

    let mut unknown = encoded.as_object().unwrap().clone();
    unknown.insert("claim_text".into(), serde_json::json!("looks verified"));
    assert!(serde_json::from_value::<Claim>(serde_json::Value::Object(unknown)).is_err());

    let mut unsupported = encoded.as_object().unwrap().clone();
    unsupported.insert("schema_version".into(), serde_json::json!(2));
    assert!(serde_json::from_value::<Claim>(serde_json::Value::Object(unsupported)).is_err());

    let mut forged = encoded.as_object().unwrap().clone();
    forged.insert("state".into(), serde_json::json!("verified"));
    assert!(serde_json::from_value::<Claim>(serde_json::Value::Object(forged)).is_err());
}

#[test]
// @spec:AC-1414
fn bounds_duplicates_sensitive_values_and_malformed_evidence_fail_closed() {
    let project_id = ProjectId::new();
    let run_id = RunId::new();
    let trace_id = TraceId::new();
    let scope = identity(
        project_id,
        run_id,
        trace_id,
        &digest('1')[..40],
        &digest('2')[..40],
    );

    assert_eq!(
        Claim::new(
            project_id,
            run_id,
            trace_id,
            "claim-1",
            ClaimClass::PlanFinding,
            "not-a-digest",
            vec![ClaimEvidenceKind::Commit],
            scope.clone(),
        )
        .unwrap_err(),
        ClaimEvidenceError::InvalidDigest
    );
    assert_eq!(
        Claim::new(
            project_id,
            run_id,
            trace_id,
            "claim-1",
            ClaimClass::PlanFinding,
            digest('c'),
            vec![ClaimEvidenceKind::Commit, ClaimEvidenceKind::Commit],
            scope.clone(),
        )
        .unwrap_err(),
        ClaimEvidenceError::DuplicateEvidence
    );
    assert_eq!(
        Claim::new(
            project_id,
            run_id,
            trace_id,
            "claim-1",
            ClaimClass::PlanFinding,
            digest('c'),
            vec![ClaimEvidenceKind::Commit; MAX_REQUIRED_EVIDENCE + 1],
            scope.clone(),
        )
        .unwrap_err(),
        ClaimEvidenceError::BoundsExceeded
    );

    let claim = claim(&scope);
    let evidence_record = evidence(
        &claim,
        &scope,
        "evidence-1",
        ClaimEvidenceKind::Commit,
        EvidenceStatus::Verified,
        'a',
    );
    assert_eq!(
        claim
            .clone()
            .apply_resolution(
                ClaimResolution::verified(vec!["evidence-1".into(), "evidence-1".into(),]),
                &[evidence_record],
            )
            .unwrap_err(),
        ClaimEvidenceError::DuplicateEvidence
    );

    let too_many = (0..=MAX_CLAIM_EVIDENCE_REFERENCES)
        .map(|index| format!("evidence-{index}"))
        .collect();
    assert_eq!(
        claim
            .clone()
            .apply_resolution(ClaimResolution::verified(too_many), &[])
            .unwrap_err(),
        ClaimEvidenceError::BoundsExceeded
    );

    let too_many_records = vec![
        evidence(
            &claim,
            &scope,
            "evidence-1",
            ClaimEvidenceKind::Commit,
            EvidenceStatus::Verified,
            'a',
        );
        MAX_EVIDENCE_RECORDS + 1
    ];
    assert_eq!(
        claim
            .clone()
            .apply_resolution(
                ClaimResolution::verified(vec!["evidence-1".into()]),
                &too_many_records,
            )
            .unwrap_err(),
        ClaimEvidenceError::BoundsExceeded
    );

    let sensitive = EvidenceRecord::new(
        claim.claim_id(),
        "evidence-sensitive",
        ClaimEvidenceKind::Commit,
        scope,
        digest('a'),
        "resolver token=should-not-store",
        EvidenceStatus::Verified,
    );
    assert_eq!(sensitive.unwrap_err(), ClaimEvidenceError::SensitiveValue);
    assert_eq!(
        ClaimResolution::verified(vec!["evidence-1".into()]).with_reason("secret=should-not-store"),
        Err(ClaimEvidenceError::SensitiveValue)
    );
}

#[test]
// @spec:AC-1415
fn foreign_claim_scope_and_sha_tree_mismatches_are_rejected() {
    let project_id = ProjectId::new();
    let run_id = RunId::new();
    let trace_id = TraceId::new();
    let scope = identity(
        project_id,
        run_id,
        trace_id,
        &digest('1')[..40],
        &digest('2')[..40],
    );
    let mut claim = claim(&scope);
    let foreign_project = ProjectId::new();
    let foreign_scope = identity(
        foreign_project,
        run_id,
        trace_id,
        scope.head_sha().unwrap(),
        scope.tree_sha().unwrap(),
    );
    let foreign = evidence(
        &claim,
        &foreign_scope,
        "evidence-foreign",
        ClaimEvidenceKind::Commit,
        EvidenceStatus::Verified,
        'a',
    );
    assert_eq!(
        claim
            .apply_resolution(
                ClaimResolution::verified(vec!["evidence-foreign".into()]),
                &[foreign],
            )
            .unwrap_err(),
        ClaimEvidenceError::IdentityMismatch
    );

    let wrong_claim = EvidenceRecord::new(
        "claim-foreign",
        "evidence-wrong-claim",
        ClaimEvidenceKind::Commit,
        scope,
        digest('a'),
        "resolver-git-v1",
        EvidenceStatus::Verified,
    )
    .unwrap();
    assert_eq!(
        claim
            .apply_resolution(
                ClaimResolution::verified(vec!["evidence-wrong-claim".into()]),
                &[wrong_claim],
            )
            .unwrap_err(),
        ClaimEvidenceError::ClaimMismatch
    );
}

#[test]
// @spec:AC-1416
fn all_wire_entities_round_trip_without_authority_or_unbounded_payloads() {
    let project_id = ProjectId::new();
    let run_id = RunId::new();
    let trace_id = TraceId::new();
    let scope = identity(
        project_id,
        run_id,
        trace_id,
        &digest('1')[..40],
        &digest('2')[..40],
    );
    let claim = claim(&scope);
    let record = evidence(
        &claim,
        &scope,
        "evidence-commit",
        ClaimEvidenceKind::Commit,
        EvidenceStatus::Verified,
        'a',
    );
    let resolution = ClaimResolution::verified(vec!["evidence-commit".into()]);
    assert!(!record.reason().is_empty());
    assert!(!resolution.reason().is_empty());

    assert_eq!(
        serde_json::from_value::<EvidenceScope>(serde_json::to_value(&scope).unwrap()).unwrap(),
        scope
    );
    assert_eq!(
        serde_json::from_value::<EvidenceRecord>(serde_json::to_value(&record).unwrap()).unwrap(),
        record
    );
    assert_eq!(
        serde_json::from_value::<ClaimResolution>(serde_json::to_value(&resolution).unwrap())
            .unwrap(),
        resolution
    );

    let mut unknown_scope = serde_json::to_value(&scope)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    unknown_scope.insert("authority".into(), serde_json::json!(true));
    assert!(
        serde_json::from_value::<EvidenceScope>(serde_json::Value::Object(unknown_scope)).is_err()
    );

    let mut unknown_record = serde_json::to_value(&record)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    unknown_record.insert("payload".into(), serde_json::json!("unbounded"));
    assert!(
        serde_json::from_value::<EvidenceRecord>(serde_json::Value::Object(unknown_record))
            .is_err()
    );

    let mut unsupported_record = serde_json::to_value(&record)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    unsupported_record.insert("schema_version".into(), serde_json::json!(2));
    assert!(
        serde_json::from_value::<EvidenceRecord>(serde_json::Value::Object(unsupported_record))
            .is_err()
    );

    let mut unknown_resolution = serde_json::to_value(&resolution)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    unknown_resolution.insert("approve".into(), serde_json::json!(true));
    assert!(
        serde_json::from_value::<ClaimResolution>(serde_json::Value::Object(unknown_resolution))
            .is_err()
    );

    let mut forged_resolution = serde_json::to_value(ClaimResolution::no_proof())
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    forged_resolution.insert("state".into(), serde_json::json!("verified"));
    assert!(
        serde_json::from_value::<ClaimResolution>(serde_json::Value::Object(forged_resolution))
            .is_err()
    );
    assert!(!claim.can_execute());
    assert!(!claim.can_approve());
    assert!(!claim.can_merge());
}
