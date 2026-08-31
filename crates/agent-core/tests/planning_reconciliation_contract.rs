use agent_core::planning_reconciliation::{
    ConflictKind, Disposition, EvidenceRef, EvidenceStatus, FinalPlanStatus, FindingSeverity,
    PlanningReconciliation, ReconciliationError, ReconciliationOutcome, ReconciliationRequest,
    ReviewerFinding, ReviewerKind, PLANNING_RECONCILIATION_SCHEMA_VERSION,
};
use agent_core::{ProjectId, RunId, TraceId};

fn scope() -> (ProjectId, RunId, TraceId) {
    (ProjectId::new(), RunId::new(), TraceId::new())
}

fn evidence(project_id: ProjectId, run_id: RunId, trace_id: TraceId, id: &str) -> EvidenceRef {
    EvidenceRef::new(
        project_id,
        run_id,
        trace_id,
        id,
        "a".repeat(64),
        EvidenceStatus::Verified,
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn finding(
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    id: &str,
    reviewer_id: &str,
    severity: FindingSeverity,
    disposition: Option<Disposition>,
    evidence_refs: Vec<EvidenceRef>,
) -> ReviewerFinding {
    ReviewerFinding::new(
        project_id,
        run_id,
        trace_id,
        id,
        reviewer_id,
        ReviewerKind::Security,
        severity,
        "security",
        "tool execution contract",
        "execution may bypass the permission engine",
        "b".repeat(64),
        "The finding is backed by the reviewer evidence.",
    )
    .unwrap()
    .with_evidence(evidence_refs)
    .with_disposition(disposition)
}

fn request(
    project_id: ProjectId,
    run_id: RunId,
    trace_id: TraceId,
    findings: Vec<ReviewerFinding>,
) -> ReconciliationRequest {
    ReconciliationRequest::new(
        project_id,
        run_id,
        trace_id,
        "idem-plan-1",
        "c".repeat(64),
        "policy-r1",
        "planner-1",
        "judge-1",
        1,
        findings,
    )
    .unwrap()
}

fn plan(outcome: ReconciliationOutcome) -> agent_core::planning_reconciliation::FinalPlan {
    match outcome {
        ReconciliationOutcome::FinalPlan(value) => *value,
        ReconciliationOutcome::Cancelled { .. } => panic!("expected a final plan"),
    }
}

#[test]
// @spec:AC-1403
fn disposition_matrix_is_bounded_and_high_findings_need_verified_evidence() {
    let (project_id, run_id, trace_id) = scope();
    let low = finding(
        project_id,
        run_id,
        trace_id,
        "low-1",
        "reviewer-low",
        FindingSeverity::Low,
        None,
        Vec::new(),
    );
    let mut medium = finding(
        project_id,
        run_id,
        trace_id,
        "medium-1",
        "reviewer-medium",
        FindingSeverity::Medium,
        Some(Disposition::Mitigate),
        Vec::new(),
    );
    let mut high = finding(
        project_id,
        run_id,
        trace_id,
        "high-1",
        "reviewer-high",
        FindingSeverity::High,
        Some(Disposition::Accept),
        vec![evidence(project_id, run_id, trace_id, "evidence-high")],
    );
    let mut critical_without_evidence = finding(
        project_id,
        run_id,
        trace_id,
        "critical-1",
        "reviewer-critical",
        FindingSeverity::Critical,
        Some(Disposition::Reject),
        Vec::new(),
    );
    medium.affected_contract = "medium finding contract".into();
    high.affected_contract = "high finding contract".into();
    critical_without_evidence.affected_contract = "critical finding contract".into();

    let result = plan(
        PlanningReconciliation::reconcile(&request(
            project_id,
            run_id,
            trace_id,
            vec![low, medium, high, critical_without_evidence],
        ))
        .unwrap(),
    );

    assert_eq!(result.status, FinalPlanStatus::HumanRequired);
    assert_eq!(result.findings.len(), 4);
    assert_eq!(
        result.decision_for("low-1").unwrap().disposition,
        Disposition::Defer
    );
    assert_eq!(
        result.decision_for("medium-1").unwrap().disposition,
        Disposition::Mitigate
    );
    assert_eq!(
        result.decision_for("high-1").unwrap().disposition,
        Disposition::Accept
    );
    assert_eq!(
        result.decision_for("critical-1").unwrap().disposition,
        Disposition::HumanRequired
    );
    assert_eq!(result.metrics.human_required, 1);
    assert!(!result.can_execute());
    assert!(!result.can_approve());
    assert!(!result.can_merge());

    let mut unverified = finding(
        project_id,
        run_id,
        trace_id,
        "high-unverified",
        "reviewer-unverified",
        FindingSeverity::High,
        Some(Disposition::Mitigate),
        vec![evidence(
            project_id,
            run_id,
            trace_id,
            "evidence-unverified",
        )],
    );
    unverified.evidence[0].status = EvidenceStatus::Unverified;
    let unverified_plan = plan(
        PlanningReconciliation::reconcile(&request(project_id, run_id, trace_id, vec![unverified]))
            .unwrap(),
    );
    assert_eq!(
        unverified_plan
            .decision_for("high-unverified")
            .unwrap()
            .disposition,
        Disposition::HumanRequired
    );
}

#[test]
// @spec:AC-1404
fn duplicate_findings_keep_all_provenance_and_conflicting_dispositions_escalate() {
    let (project_id, run_id, trace_id) = scope();
    let first = finding(
        project_id,
        run_id,
        trace_id,
        "finding-a",
        "reviewer-a",
        FindingSeverity::Medium,
        Some(Disposition::Accept),
        vec![evidence(project_id, run_id, trace_id, "evidence-1")],
    );
    let second = finding(
        project_id,
        run_id,
        trace_id,
        "finding-b",
        "reviewer-b",
        FindingSeverity::Medium,
        Some(Disposition::Accept),
        vec![evidence(project_id, run_id, trace_id, "evidence-1")],
    );
    let first_request = request(
        project_id,
        run_id,
        trace_id,
        vec![first.clone(), second.clone()],
    );
    let first_plan = plan(PlanningReconciliation::reconcile(&first_request).unwrap());

    assert_eq!(first_plan.findings.len(), 2);
    assert_eq!(first_plan.decisions.len(), 1);
    assert_eq!(
        first_plan.decisions[0].finding_ids,
        ["finding-a", "finding-b"]
    );
    assert_eq!(first_plan.metrics.deduplicated_findings, 1);
    assert_eq!(first_plan.metrics.reviewer_disagreements, 0);

    let conflicting = second.with_disposition(Some(Disposition::Reject));
    let conflicting_plan = plan(
        PlanningReconciliation::reconcile(&request(
            project_id,
            run_id,
            trace_id,
            vec![first, conflicting],
        ))
        .unwrap(),
    );
    assert_eq!(conflicting_plan.status, FinalPlanStatus::HumanRequired);
    assert_eq!(conflicting_plan.metrics.reviewer_disagreements, 1);
    assert_eq!(
        conflicting_plan.disagreements[0].kind,
        ConflictKind::ReviewerDisagreement
    );
    assert_eq!(conflicting_plan.findings.len(), 2);
}

#[test]
// @spec:AC-1405
fn unresolved_policy_product_conflict_is_human_required() {
    let (project_id, run_id, trace_id) = scope();
    let conflict = finding(
        project_id,
        run_id,
        trace_id,
        "policy-product-1",
        "reviewer-policy",
        FindingSeverity::Low,
        Some(Disposition::Accept),
        Vec::new(),
    )
    .with_conflict(Some(ConflictKind::PolicyProduct));

    let result = plan(
        PlanningReconciliation::reconcile(&request(project_id, run_id, trace_id, vec![conflict]))
            .unwrap(),
    );

    assert_eq!(result.status, FinalPlanStatus::HumanRequired);
    assert_eq!(result.metrics.policy_product_conflicts, 1);
    assert_eq!(
        result.decision_for("policy-product-1").unwrap().disposition,
        Disposition::HumanRequired
    );
}

#[test]
// @spec:AC-1406
fn self_approval_and_round_overflow_fail_closed_without_dropping_findings() {
    let (project_id, run_id, trace_id) = scope();
    let self_approval_finding = finding(
        project_id,
        run_id,
        trace_id,
        "finding-1",
        "reviewer-1",
        FindingSeverity::Info,
        None,
        Vec::new(),
    );
    let mut self_approval_request =
        request(project_id, run_id, trace_id, vec![self_approval_finding]);
    self_approval_request.findings[0].reviewer_id = "planner-1".into();
    let error = PlanningReconciliation::reconcile(&self_approval_request).unwrap_err();
    assert_eq!(error, ReconciliationError::SelfApproval);

    let valid = finding(
        project_id,
        run_id,
        trace_id,
        "finding-2",
        "reviewer-1",
        FindingSeverity::Info,
        None,
        Vec::new(),
    );
    let mut overflow = request(project_id, run_id, trace_id, vec![valid]);
    overflow.round = 3;
    assert_eq!(
        PlanningReconciliation::reconcile(&overflow).unwrap_err(),
        ReconciliationError::RoundOverflow
    );

    let mut sensitive = request(
        project_id,
        run_id,
        trace_id,
        vec![finding(
            project_id,
            run_id,
            trace_id,
            "finding-sensitive",
            "reviewer-1",
            FindingSeverity::Info,
            None,
            Vec::new(),
        )],
    );
    sensitive.findings[0].rationale = "token=do-not-store".into();
    assert_eq!(
        PlanningReconciliation::reconcile(&sensitive).unwrap_err(),
        ReconciliationError::InvalidFinding
    );
}

#[test]
// @spec:AC-1407
fn final_plan_schema_is_versioned_unknown_fields_fail_closed_and_identity_is_preserved() {
    let (project_id, run_id, trace_id) = scope();
    let value = finding(
        project_id,
        run_id,
        trace_id,
        "finding-1",
        "reviewer-1",
        FindingSeverity::Info,
        None,
        Vec::new(),
    );
    let result = plan(
        PlanningReconciliation::reconcile(&request(project_id, run_id, trace_id, vec![value]))
            .unwrap(),
    );
    assert_eq!(
        result.schema_version,
        PLANNING_RECONCILIATION_SCHEMA_VERSION
    );
    assert_eq!(result.project_id, project_id);
    assert_eq!(result.run_id, run_id);
    assert_eq!(result.trace_id, trace_id);

    let encoded = serde_json::to_value(&result).unwrap();
    let decoded: agent_core::planning_reconciliation::FinalPlan =
        serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(decoded, result);
    let mut unknown = encoded.as_object().unwrap().clone();
    unknown.insert("approve".into(), serde_json::json!(true));
    assert!(
        serde_json::from_value::<agent_core::planning_reconciliation::FinalPlan>(
            serde_json::Value::Object(unknown)
        )
        .is_err()
    );
}

#[test]
// @spec:AC-1408
fn fake_pipeline_is_idempotent_cancellable_and_reopens_from_immutable_plan() {
    let (project_id, run_id, trace_id) = scope();
    let findings = vec![
        finding(
            project_id,
            run_id,
            trace_id,
            "architecture-1",
            "architecture-reviewer",
            FindingSeverity::Low,
            Some(Disposition::Accept),
            Vec::new(),
        ),
        finding(
            project_id,
            run_id,
            trace_id,
            "test-1",
            "test-reviewer",
            FindingSeverity::Medium,
            Some(Disposition::Mitigate),
            Vec::new(),
        ),
    ];
    let input = request(project_id, run_id, trace_id, findings);
    let first = plan(PlanningReconciliation::reconcile(&input).unwrap());
    let second = plan(PlanningReconciliation::reconcile(&input).unwrap());
    assert_eq!(first.fingerprint, second.fingerprint);
    assert_eq!(first, second);

    let mut cancelled = input.clone();
    cancelled.cancelled = true;
    assert!(matches!(
        PlanningReconciliation::reconcile(&cancelled).unwrap(),
        ReconciliationOutcome::Cancelled { .. }
    ));

    let reopened = first.reopen().unwrap();
    assert_eq!(reopened.findings.len(), first.findings.len());
    assert_eq!(
        reopened.reopened_from.as_deref(),
        Some(first.fingerprint.as_str())
    );
    let reopened_request = reopened.clone().into_request().unwrap();
    let reopened_plan = plan(PlanningReconciliation::reconcile(&reopened_request).unwrap());
    assert_eq!(first, reopened_plan);
    assert!(!first.can_execute());

    let mut reordered = input.clone();
    reordered.findings.reverse();
    let reordered_plan = plan(PlanningReconciliation::reconcile(&reordered).unwrap());
    assert_eq!(first, reordered_plan);
}
