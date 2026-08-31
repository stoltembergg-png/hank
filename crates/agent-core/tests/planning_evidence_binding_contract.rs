use agent_core::planning_evidence_binding::{
    EvidenceBindingMetrics, PlanningEvidenceAdapter, PlanningEvidenceBindingError,
    PlanningEvidenceBindingOutcome,
};
use agent_core::planning_reconciliation::{
    Disposition, EvidenceRef, EvidenceStatus as PlanningEvidenceStatus, FindingSeverity,
    ReviewerFinding, ReviewerKind,
};
use agent_core::{
    ClaimClass, ClaimEvidenceKind, ClaimEvidenceStatus, EvidenceRecord, EvidenceScope, FactState,
    ProjectId, RunId, TraceId,
};

fn scope() -> (ProjectId, RunId, TraceId) {
    (ProjectId::new(), RunId::new(), TraceId::new())
}

fn identity(project_id: ProjectId, run_id: RunId, trace_id: TraceId) -> EvidenceScope {
    EvidenceScope::new(
        project_id,
        run_id,
        trace_id,
        "a".repeat(64),
        Some("b".repeat(40)),
        Some("c".repeat(40)),
        Some("policy-r1".into()),
        Some("schema-r1".into()),
    )
    .unwrap()
}

fn evidence_ref(
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    evidence_id: &str,
    digest: &str,
    status: PlanningEvidenceStatus,
) -> EvidenceRef {
    EvidenceRef::new(project_id, run_id, trace_id, evidence_id, digest, status).unwrap()
}

fn finding(
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    evidence: Vec<EvidenceRef>,
    disposition: Option<Disposition>,
) -> ReviewerFinding {
    ReviewerFinding::new(
        project_id,
        run_id,
        trace_id,
        "finding-1",
        "reviewer-1",
        ReviewerKind::Security,
        FindingSeverity::Medium,
        "evidence",
        "claim contract",
        "the claim needs resolver evidence",
        "d".repeat(64),
        "the reviewer record is bounded and auditable",
    )
    .unwrap()
    .with_evidence(evidence)
    .with_disposition(disposition)
}

fn record(
    finding_id: &str,
    evidence_id: &str,
    kind: ClaimEvidenceKind,
    scope: EvidenceScope,
    digest: &str,
    status: ClaimEvidenceStatus,
) -> EvidenceRecord {
    EvidenceRecord::new(
        finding_id,
        evidence_id,
        kind,
        scope,
        digest,
        "resolver-fake",
        status,
    )
    .unwrap()
}

fn request(
    finding: ReviewerFinding,
    expected_identity: EvidenceScope,
    required_evidence: Vec<ClaimEvidenceKind>,
    evidence_records: Vec<EvidenceRecord>,
) -> agent_core::planning_evidence_binding::PlanningEvidenceBindingRequest {
    agent_core::planning_evidence_binding::PlanningEvidenceBindingRequest::new(
        finding.project_id,
        finding.run_id,
        finding.trace_id,
        "binding-idem-1",
        finding,
        expected_identity,
        required_evidence,
        evidence_records,
    )
    .unwrap()
}

fn bound(
    outcome: PlanningEvidenceBindingOutcome,
) -> agent_core::planning_evidence_binding::PlanningEvidenceBinding {
    match outcome {
        PlanningEvidenceBindingOutcome::Bound(value) => *value,
        PlanningEvidenceBindingOutcome::Cancelled { .. } => panic!("expected a bound finding"),
    }
}

#[test]
// @spec:AC-1420
fn resolver_mapping_requires_exact_evidence_and_blocks_no_proof_mitigation() {
    let (project_id, run_id, trace_id) = scope();
    let expected = identity(project_id, run_id, trace_id);
    let no_proof = bound(
        PlanningEvidenceAdapter::bind(&request(
            finding(
                project_id,
                run_id,
                trace_id,
                Vec::new(),
                Some(Disposition::Mitigate),
            ),
            expected.clone(),
            vec![ClaimEvidenceKind::Artifact],
            Vec::new(),
        ))
        .unwrap(),
    );

    assert_eq!(no_proof.state(), FactState::NoProof);
    assert_eq!(
        no_proof.effective_disposition(),
        Some(Disposition::HumanRequired)
    );
    assert!(!no_proof.mitigation_allowed());
    assert_eq!(no_proof.metrics().no_proof, 1);
    assert_eq!(no_proof.claim().claim_class(), ClaimClass::PlanFinding);

    let fabricated_ref = evidence_ref(
        project_id,
        run_id,
        trace_id,
        "fabricated",
        &"e".repeat(64),
        PlanningEvidenceStatus::Verified,
    );
    let fabricated = PlanningEvidenceAdapter::bind(&request(
        finding(
            project_id,
            run_id,
            trace_id,
            vec![fabricated_ref],
            Some(Disposition::Mitigate),
        ),
        expected.clone(),
        vec![ClaimEvidenceKind::Artifact],
        Vec::new(),
    ));
    assert_eq!(
        fabricated.unwrap_err(),
        PlanningEvidenceBindingError::MissingEvidenceReference
    );

    let digest = "f".repeat(64);
    let mismatched_ref = evidence_ref(
        project_id,
        run_id,
        trace_id,
        "evidence-1",
        &digest,
        PlanningEvidenceStatus::Verified,
    );
    let mismatched_record = record(
        "finding-1",
        "evidence-1",
        ClaimEvidenceKind::Artifact,
        expected.clone(),
        &"1".repeat(64),
        ClaimEvidenceStatus::Verified,
    );
    assert_eq!(
        PlanningEvidenceAdapter::bind(&request(
            finding(
                project_id,
                run_id,
                trace_id,
                vec![mismatched_ref],
                Some(Disposition::Mitigate),
            ),
            expected,
            vec![ClaimEvidenceKind::Artifact],
            vec![mismatched_record],
        ))
        .unwrap_err(),
        PlanningEvidenceBindingError::EvidenceReferenceMismatch
    );
}

#[test]
// @spec:AC-1421
fn verified_mapping_binds_claim_and_evidence_with_identity() {
    let (project_id, run_id, trace_id) = scope();
    let expected = identity(project_id, run_id, trace_id);
    let commit_digest = "1".repeat(64);
    let tree_digest = "2".repeat(64);
    let finding = finding(
        project_id,
        run_id,
        trace_id,
        vec![
            evidence_ref(
                project_id,
                run_id,
                trace_id,
                "evidence-tree",
                &tree_digest,
                PlanningEvidenceStatus::Verified,
            ),
            evidence_ref(
                project_id,
                run_id,
                trace_id,
                "evidence-commit",
                &commit_digest,
                PlanningEvidenceStatus::Verified,
            ),
        ],
        Some(Disposition::Mitigate),
    );
    let result = bound(
        PlanningEvidenceAdapter::bind(&request(
            finding,
            expected.clone(),
            vec![ClaimEvidenceKind::Commit, ClaimEvidenceKind::Tree],
            vec![
                record(
                    "finding-1",
                    "evidence-tree",
                    ClaimEvidenceKind::Tree,
                    expected.clone(),
                    &tree_digest,
                    ClaimEvidenceStatus::Verified,
                ),
                record(
                    "finding-1",
                    "evidence-commit",
                    ClaimEvidenceKind::Commit,
                    expected,
                    &commit_digest,
                    ClaimEvidenceStatus::Verified,
                ),
            ],
        ))
        .unwrap(),
    );

    assert_eq!(result.state(), FactState::Verified);
    assert_eq!(result.claim().claim_class(), ClaimClass::PlanFinding);
    assert_eq!(result.claim().claim_digest(), "d".repeat(64));
    assert_eq!(
        result.claim().evidence_ids(),
        ["evidence-commit", "evidence-tree"]
    );
    assert_eq!(result.evidence_records().len(), 2);
    assert_eq!(result.metrics().verified, 2);
    assert!(result.mitigation_allowed());
    assert!(!result.claim().can_execute());
    assert!(!result.claim().can_approve());
    assert!(!result.claim().can_merge());
}

#[test]
// @spec:AC-1422
fn stale_conflicting_foreign_and_sha_tree_evidence_fail_closed() {
    let (project_id, run_id, trace_id) = scope();
    let expected = identity(project_id, run_id, trace_id);
    let digest = "3".repeat(64);
    let ref_for =
        |status| evidence_ref(project_id, run_id, trace_id, "evidence-1", &digest, status);

    let stale = bound(
        PlanningEvidenceAdapter::bind(&request(
            finding(
                project_id,
                run_id,
                trace_id,
                vec![ref_for(PlanningEvidenceStatus::Stale)],
                Some(Disposition::Mitigate),
            ),
            expected.clone(),
            vec![ClaimEvidenceKind::Artifact],
            vec![record(
                "finding-1",
                "evidence-1",
                ClaimEvidenceKind::Artifact,
                expected.clone(),
                &digest,
                ClaimEvidenceStatus::Stale,
            )],
        ))
        .unwrap(),
    );
    assert_eq!(stale.state(), FactState::Stale);
    assert!(!stale.mitigation_allowed());
    assert_eq!(stale.metrics().stale, 1);

    let conflicting = bound(
        PlanningEvidenceAdapter::bind(&request(
            finding(
                project_id,
                run_id,
                trace_id,
                vec![ref_for(PlanningEvidenceStatus::Conflicting)],
                Some(Disposition::Mitigate),
            ),
            expected.clone(),
            vec![ClaimEvidenceKind::Artifact],
            vec![record(
                "finding-1",
                "evidence-1",
                ClaimEvidenceKind::Artifact,
                expected.clone(),
                &digest,
                ClaimEvidenceStatus::Conflicting,
            )],
        ))
        .unwrap(),
    );
    assert_eq!(conflicting.state(), FactState::Conflicting);
    assert!(!conflicting.mitigation_allowed());
    assert_eq!(conflicting.metrics().conflicting, 1);

    let mut foreign_claim = record(
        "other-finding",
        "evidence-1",
        ClaimEvidenceKind::Artifact,
        expected.clone(),
        &digest,
        ClaimEvidenceStatus::Verified,
    );
    assert_eq!(
        PlanningEvidenceAdapter::bind(&request(
            finding(
                project_id,
                run_id,
                trace_id,
                vec![ref_for(PlanningEvidenceStatus::Verified)],
                Some(Disposition::Mitigate),
            ),
            expected.clone(),
            vec![ClaimEvidenceKind::Artifact],
            vec![foreign_claim.clone()],
        ))
        .unwrap_err(),
        PlanningEvidenceBindingError::ClaimEvidence(agent_core::ClaimEvidenceError::ClaimMismatch)
    );

    let foreign_scope = identity(project_id, run_id, TraceId::new());
    foreign_claim = record(
        "finding-1",
        "evidence-1",
        ClaimEvidenceKind::Artifact,
        foreign_scope,
        &digest,
        ClaimEvidenceStatus::Verified,
    );
    assert_eq!(
        PlanningEvidenceAdapter::bind(&request(
            finding(
                project_id,
                run_id,
                trace_id,
                vec![ref_for(PlanningEvidenceStatus::Verified)],
                Some(Disposition::Mitigate),
            ),
            expected.clone(),
            vec![ClaimEvidenceKind::Artifact],
            vec![foreign_claim],
        ))
        .unwrap_err(),
        PlanningEvidenceBindingError::ClaimEvidence(
            agent_core::ClaimEvidenceError::IdentityMismatch
        )
    );

    let mut wrong_revision_scope = expected;
    wrong_revision_scope = EvidenceScope::new(
        project_id,
        run_id,
        trace_id,
        wrong_revision_scope.identity_digest().into(),
        Some("9".repeat(40)),
        wrong_revision_scope.tree_sha().map(str::to_owned),
        wrong_revision_scope.policy_revision().map(str::to_owned),
        wrong_revision_scope.schema_revision().map(str::to_owned),
    )
    .unwrap();
    assert_eq!(
        PlanningEvidenceAdapter::bind(&request(
            finding(
                project_id,
                run_id,
                trace_id,
                vec![ref_for(PlanningEvidenceStatus::Verified)],
                Some(Disposition::Mitigate),
            ),
            identity(project_id, run_id, trace_id),
            vec![ClaimEvidenceKind::Artifact],
            vec![record(
                "finding-1",
                "evidence-1",
                ClaimEvidenceKind::Artifact,
                wrong_revision_scope,
                &digest,
                ClaimEvidenceStatus::Verified,
            )],
        ))
        .unwrap_err(),
        PlanningEvidenceBindingError::ClaimEvidence(
            agent_core::ClaimEvidenceError::IdentityMismatch
        )
    );
}

#[test]
// @spec:AC-1423
fn binding_is_idempotent_cancellable_and_rejects_unbound_records() {
    let (project_id, run_id, trace_id) = scope();
    let expected = identity(project_id, run_id, trace_id);
    let digest = "4".repeat(64);
    let binding_request = request(
        finding(
            project_id,
            run_id,
            trace_id,
            vec![evidence_ref(
                project_id,
                run_id,
                trace_id,
                "evidence-1",
                &digest,
                PlanningEvidenceStatus::Verified,
            )],
            Some(Disposition::Accept),
        ),
        expected.clone(),
        vec![ClaimEvidenceKind::Artifact],
        vec![record(
            "finding-1",
            "evidence-1",
            ClaimEvidenceKind::Artifact,
            expected.clone(),
            &digest,
            ClaimEvidenceStatus::Verified,
        )],
    );
    let first = bound(PlanningEvidenceAdapter::bind(&binding_request).unwrap());
    let second = bound(PlanningEvidenceAdapter::bind(&binding_request).unwrap());
    assert_eq!(first, second);
    assert_eq!(first.fingerprint(), second.fingerprint());

    let cancelled = PlanningEvidenceAdapter::bind(&binding_request.with_cancelled(true)).unwrap();
    assert!(matches!(
        cancelled,
        PlanningEvidenceBindingOutcome::Cancelled { .. }
    ));

    let unbound = PlanningEvidenceAdapter::bind(&request(
        finding(project_id, run_id, trace_id, Vec::new(), None),
        expected.clone(),
        vec![ClaimEvidenceKind::Artifact],
        vec![record(
            "finding-1",
            "orphan",
            ClaimEvidenceKind::Artifact,
            expected,
            &digest,
            ClaimEvidenceStatus::Verified,
        )],
    ));
    assert_eq!(
        unbound.unwrap_err(),
        PlanningEvidenceBindingError::UnexpectedEvidence
    );
}

#[test]
// @spec:AC-1424
fn binding_schema_bounds_and_observability_are_explicit() {
    let (project_id, run_id, trace_id) = scope();
    let expected = identity(project_id, run_id, trace_id);
    let base = request(
        finding(project_id, run_id, trace_id, Vec::new(), None),
        expected,
        vec![ClaimEvidenceKind::Artifact],
        Vec::new(),
    );
    assert_eq!(base.schema_version(), 1);
    let no_proof = bound(PlanningEvidenceAdapter::bind(&base).unwrap());
    assert_eq!(
        no_proof.metrics(),
        &EvidenceBindingMetrics {
            no_proof: 1,
            ..Default::default()
        }
    );

    let roundtripped = serde_json::from_value::<
        agent_core::planning_evidence_binding::PlanningEvidenceBinding,
    >(serde_json::to_value(&no_proof).unwrap())
    .unwrap();
    assert_eq!(roundtripped, no_proof);

    let mut unsupported_result = serde_json::to_value(&no_proof).unwrap();
    unsupported_result
        .as_object_mut()
        .unwrap()
        .insert("schema_version".into(), serde_json::json!(99));
    assert!(
        serde_json::from_value::<agent_core::planning_evidence_binding::PlanningEvidenceBinding>(
            unsupported_result
        )
        .is_err()
    );

    let mut unknown = serde_json::to_value(&base).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("execute".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<
        agent_core::planning_evidence_binding::PlanningEvidenceBindingRequest,
    >(unknown)
    .is_err());

    let mut unsupported = serde_json::to_value(&base).unwrap();
    unsupported
        .as_object_mut()
        .unwrap()
        .insert("schema_version".into(), serde_json::json!(99));
    assert!(serde_json::from_value::<
        agent_core::planning_evidence_binding::PlanningEvidenceBindingRequest,
    >(unsupported)
    .is_err());
}
