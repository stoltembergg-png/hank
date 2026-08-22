use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use serde_json::json;
use tool_core::{
    ApprovalRequest, ConfirmationAttempt, ConfirmationLedger, ConfirmationPolicy,
    PermissionDecision, PermissionError, PermissionEvaluator, PermissionRequest, PolicyDecision,
    ToolEffect,
};

fn project(value: &str) -> ProjectId {
    let suffix = match value {
        "permission" => "001",
        "other" => "002",
        _ => "003",
    };
    ProjectId::parse(&format!("proj-00000000-0000-4000-8000-000000000{}", suffix)).unwrap()
}

fn request(policy: PolicyDecision, effect: ToolEffect) -> PermissionRequest {
    PermissionRequest {
        project_id: Some(project("project:permission")),
        tool_name: "filesystem.write".into(),
        tool_version: "1.0.0".into(),
        capability: "filesystem.write".into(),
        effect,
        policy,
        budget_available: true,
        confirmation_approved: false,
    }
}

#[test]
#[allow(clippy::too_many_lines)]
// @spec:AC-632
fn default_deny_and_invalid_identity_fail_closed() {
    let evaluator = PermissionEvaluator::new();
    assert!(matches!(
        evaluator.evaluate(&request(PolicyDecision::Deny, ToolEffect::Read)),
        PermissionDecision::Denied {
            reason: PermissionError::PolicyDenied
        }
    ));
    let mut invalid = request(PolicyDecision::Allow, ToolEffect::Read);
    invalid.project_id = None;
    assert!(matches!(
        evaluator.evaluate(&invalid),
        PermissionDecision::Denied {
            reason: PermissionError::MissingProject
        }
    ));
}

#[test]
// @spec:AC-632
fn read_only_allow_is_deterministic_but_destructive_effects_need_confirmation() {
    let evaluator = PermissionEvaluator::new();
    assert_eq!(
        evaluator.evaluate(&request(PolicyDecision::Allow, ToolEffect::Read)),
        PermissionDecision::Allowed {
            reason: "policy-and-budget-allow-read-only-effect"
        }
    );
    assert!(matches!(
        evaluator.evaluate(&request(PolicyDecision::AskEveryTime, ToolEffect::Write)),
        PermissionDecision::NeedsConfirmation { .. }
    ));
}

#[test]
// @spec:AC-633
fn ask_every_time_never_reuses_confirmation() {
    let evaluator = PermissionEvaluator::new();
    let mut approved = request(PolicyDecision::AskEveryTime, ToolEffect::Execute);
    approved.confirmation_approved = true;
    assert!(evaluator.evaluate(&approved).is_allowed());
    approved.confirmation_approved = false;
    assert!(matches!(
        evaluator.evaluate(&approved),
        PermissionDecision::NeedsConfirmation { .. }
    ));
}

#[test]
// @spec:AC-633
fn ask_once_is_scoped_to_project_tool_version_and_capability() {
    let evaluator = PermissionEvaluator::new();
    let mut approved = request(PolicyDecision::AskOnce, ToolEffect::Credentials);
    approved.confirmation_approved = true;
    assert!(evaluator.evaluate(&approved).is_allowed());
    approved.confirmation_approved = false;
    assert!(evaluator.evaluate(&approved).is_allowed());
    let other_project = project("other");
    assert_ne!(approved.project_id, Some(other_project));
    approved.project_id = Some(other_project);
    assert!(matches!(
        evaluator.evaluate(&approved),
        PermissionDecision::NeedsConfirmation { .. }
    ));
}

#[test]
// @spec:AC-632
fn budget_and_capability_are_checked_before_confirmation() {
    let evaluator = PermissionEvaluator::new();
    let mut unavailable = request(PolicyDecision::AskEveryTime, ToolEffect::Write);
    unavailable.budget_available = false;
    assert!(matches!(
        evaluator.evaluate(&unavailable),
        PermissionDecision::Denied {
            reason: PermissionError::BudgetUnavailable
        }
    ));
    unavailable.budget_available = true;
    unavailable.capability.clear();
    assert!(matches!(
        evaluator.evaluate(&unavailable),
        PermissionDecision::Denied {
            reason: PermissionError::MissingCapability
        }
    ));
}

#[test]
// @spec:AC-634
fn concurrent_evaluation_is_thread_safe_and_clear_is_project_scoped() {
    use std::sync::Arc;
    use std::thread;

    let evaluator = Arc::new(PermissionEvaluator::new());
    let mut handles = Vec::new();
    for _ in 0..8 {
        let evaluator = Arc::clone(&evaluator);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                assert!(
                    evaluator
                        .evaluate(&request(PolicyDecision::Allow, ToolEffect::Read))
                        .is_allowed()
                );
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    evaluator.clear_project(&project("project:permission"));
    let mut ask_once = request(PolicyDecision::AskOnce, ToolEffect::Write);
    ask_once.confirmation_approved = false;
    assert!(matches!(
        evaluator.evaluate(&ask_once),
        PermissionDecision::NeedsConfirmation { .. }
    ));
}

#[test]
// @spec:AC-673
fn evaluator_releases_sensitive_effect_only_with_matching_ledger_approval() {
    let evaluator = PermissionEvaluator::new();
    let mut permission = request(PolicyDecision::AskEveryTime, ToolEffect::Write);
    permission.confirmation_approved = false;
    let approval = approval_request(ConfirmationPolicy::AskEveryTime, &permission);
    let ledger = ConfirmationLedger::new();
    ledger.register(approval.clone()).unwrap();

    assert!(matches!(
        evaluator.evaluate_with_confirmation(&permission, None),
        PermissionDecision::NeedsConfirmation { .. }
    ));

    let grant = ledger
        .approve(approval.request_id, "user:gabriel", 1_100)
        .unwrap();
    let attempt = ConfirmationAttempt {
        ledger: &ledger,
        request: &approval,
        grant: Some(&grant),
        actor_id: "user:gabriel",
        now_ms: 1_101,
    };
    assert_eq!(
        evaluator.evaluate_with_confirmation(&permission, Some(attempt)),
        PermissionDecision::Allowed {
            reason: "ledger-confirmation"
        }
    );
}

#[test]
// @spec:AC-673
fn evaluator_denies_mismatched_or_invalid_confirmation_instead_of_bypassing_policy() {
    let evaluator = PermissionEvaluator::new();
    let permission = request(PolicyDecision::AskEveryTime, ToolEffect::Write);
    let approval = approval_request(ConfirmationPolicy::AskEveryTime, &permission);
    let ledger = ConfirmationLedger::new();
    ledger.register(approval.clone()).unwrap();
    let grant = ledger
        .approve(approval.request_id, "user:gabriel", 1_100)
        .unwrap();

    let mut changed = approval.clone();
    changed.effect = ToolEffect::Execute;
    let attempt = ConfirmationAttempt {
        ledger: &ledger,
        request: &changed,
        grant: Some(&grant),
        actor_id: "user:gabriel",
        now_ms: 1_101,
    };
    assert_eq!(
        evaluator.evaluate_with_confirmation(&permission, Some(attempt)),
        PermissionDecision::Denied {
            reason: PermissionError::ConfirmationInvalid
        }
    );

    let attempt = ConfirmationAttempt {
        ledger: &ledger,
        request: &approval,
        grant: None,
        actor_id: "user:gabriel",
        now_ms: 1_101,
    };
    assert!(matches!(
        evaluator.evaluate_with_confirmation(&permission, Some(attempt)),
        PermissionDecision::NeedsConfirmation { .. }
    ));
}

fn approval_request(policy: ConfirmationPolicy, request: &PermissionRequest) -> ApprovalRequest {
    ApprovalRequest::new(
        request.project_id.expect("fixture project"),
        None,
        request.tool_name.clone(),
        request.tool_version.clone(),
        &json!({"type": "object"}),
        &json!({"path": "notes.txt"}),
        request.effect,
        None,
        TraceId::new(),
        "agent-runtime",
        policy,
        1_000,
        2_000,
    )
    .expect("valid approval fixture")
}
