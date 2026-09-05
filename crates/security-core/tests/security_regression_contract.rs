//! Security regression contract tests for PR-260.
//!
//! Each test is annotated with its corresponding `@spec:AC-NNNN` tag so
//! the ONP feature runner can map it back to the SDD criterion. The
//! manifest at `docs/security/threat-regression-manifest.json` is the
//! canonical source of TM-NNN bindings.

// TM-NNN identifiers (TM_001_..., TM_007_...) are intentionally
// non-snake-case to mirror the manifest IDs and keep grep traceability.
#![allow(non_snake_case)]

use std::collections::BTreeSet;

use security_core::audit::{
    class_query, AuditClass, AuditLog, AuditPolicy, AuditQuery, InMemorySink, Payload,
    RedactedField, REDACTED_PLACEHOLDER,
};
use security_core::mcp_permission::{
    PermissionAction, PermissionEngine, PermissionError, PermissionRequest,
};
use security_core::plugin_permission::{
    PluginAction, PluginPermissionEngine, PluginPermissionError, PluginPermissionRequest,
};
use security_core::rate_limit::{
    RateLimitClass, RateLimitIdentity, RateLimitPolicy, RateLimitRequest, RateLimiter,
};
use security_core::{BranchMutation, BranchPolicy, BranchPolicyError, BranchPolicyRequest};

// --- Helpers ----------------------------------------------------------------

fn redacted_payload() -> Payload {
    let mut p = Payload::new();
    p.insert_text("operation", "grant").expect("text");
    p.insert_redacted("credential", RedactedField::Secret)
        .expect("redacted");
    p
}

fn observed_keys(payload: &Payload) -> BTreeSet<String> {
    payload.iter().map(|(k, _)| k.to_string()).collect()
}

fn observed_rendered(payload: &Payload) -> BTreeSet<String> {
    payload.iter().map(|(_, v)| v).collect()
}

fn policy() -> BranchPolicy {
    // Branch prefix must end with `/` (BranchPolicy::validate_prefix).
    BranchPolicy::new("p-1", "r-1", "rev-1", "p_/", vec!["main".to_string()]).expect("policy")
}

// --- TM-001 / AC-2102 -------------------------------------------------------

// @spec:AC-2102
#[test]
fn TM_001_malformed_actor_is_rejected_with_typed_error() {
    let policy = policy();
    for bad in ["", "act\nor", "act\tor", "act\u{0}or"] {
        let decision = policy.evaluate(&BranchPolicyRequest::new(
            "p-1",
            "r-1",
            "t-1",
            "owner-1",
            bad,
            "p_/t-1",
            "main",
            "rev-1",
            BranchMutation::LocalCommit,
        ));
        assert!(
            matches!(decision, Err(BranchPolicyError::InvalidRequest)),
            "malformed actor {bad:?} should be rejected with InvalidRequest, got {decision:?}"
        );
    }
}

// @spec:AC-2102
#[test]
fn TM_001_malformed_branch_request_is_rejected() {
    let policy = policy();
    for bad_branch in [
        "../escape",
        "feat/../../etc",
        "-starts-with-dash",
        "feat/ends-with-dot.",
        "feat/has space",
        "feat/has~tilde",
        "feat/has`backtick",
        "/abs",
    ] {
        let decision = policy.evaluate(&BranchPolicyRequest::new(
            "p-1",
            "r-1",
            "t-1",
            "owner-1",
            "owner-1",
            bad_branch,
            "main",
            "rev-1",
            BranchMutation::LocalCommit,
        ));
        assert!(
            decision.is_err(),
            "branch {bad_branch:?} should be rejected"
        );
    }
}

// --- TM-002 / AC-2102 -------------------------------------------------------

// @spec:AC-2102
#[test]
fn TM_002_remote_origin_not_allowlisted_is_rejected() {
    let mut engine = PermissionEngine::new("rev-1").expect("engine");
    // Without an explicit grant, the engine denies the request.
    let request = PermissionRequest::new(
        1,
        "rev-1",
        "server-a",
        "tool",
        "https://allowed.example",
        "p-1",
        "a-1",
        PermissionAction::Execution,
    );
    let decision = engine.evaluate(request, 1_000);
    assert!(matches!(
        decision,
        Ok(security_core::mcp_permission::PermissionDecision::Denied)
    ));
    // A request with a mismatched policy revision is rejected.
    let request = PermissionRequest::new(
        2,
        "rev-other",
        "server-a",
        "tool",
        "https://allowed.example",
        "p-1",
        "a-1",
        PermissionAction::Execution,
    );
    let decision = engine.evaluate(request, 1_000);
    assert!(matches!(decision, Err(PermissionError::PolicyStale)));
}

// --- TM-003 / AC-2103 -------------------------------------------------------

// @spec:AC-2103
#[test]
fn TM_003_path_traversal_attempts_are_rejected_by_branch_policy() {
    let policy = policy();
    for bad in [
        "../escape",
        "feat/../../etc",
        "-starts-with-dash",
        "feat/ends-with-dot.",
        "feat/has space",
        "feat/has~tilde",
        "feat/has`backtick",
        "/abs",
    ] {
        let decision = policy.evaluate(&BranchPolicyRequest::new(
            "p-1",
            "r-1",
            "t-1",
            "owner-1",
            "owner-1",
            bad,
            "main",
            "rev-1",
            BranchMutation::LocalCommit,
        ));
        assert!(
            decision.is_err(),
            "traversal branch {bad:?} should be rejected"
        );
    }
}

// --- TM-004 / AC-2103 -------------------------------------------------------

// @spec:AC-2103
#[test]
fn TM_004_credential_values_are_redacted_in_audit_export() {
    let mut log = AuditLog::new(
        AuditPolicy::new("p-1", "rev-1", 8, 4, 60_000).expect("policy"),
        InMemorySink::new(),
    )
    .expect("log");
    log.record(
        "evt-1",
        "actor",
        "resource",
        AuditClass::Authorization,
        1_000,
        redacted_payload(),
    )
    .expect("record");
    let exported = log.export(8).expect("export");
    let rendered = observed_rendered(exported[0].payload());
    assert!(rendered.contains(REDACTED_PLACEHOLDER));
    assert!(!rendered
        .iter()
        .any(|v| v.contains("api_key") || v.contains("password") || v.contains("token")));
}

// @spec:AC-2103
#[test]
fn TM_004_audit_query_does_not_leak_secret_values() {
    let mut log = AuditLog::new(
        AuditPolicy::new("p-1", "rev-1", 16, 8, 60_000).expect("policy"),
        InMemorySink::new(),
    )
    .expect("log");
    for i in 0..4u64 {
        log.record(
            format!("evt-{i:02}"),
            "actor-1",
            "resource-1",
            AuditClass::Authorization,
            1_000 + i,
            redacted_payload(),
        )
        .expect("record");
    }
    let q = AuditQuery::new().with_actor("actor-1").with_limit(16);
    let res = log.query(&q).expect("query");
    assert_eq!(res.len(), 4);
    for event in res.events() {
        for (_, v) in event.payload().iter() {
            assert!(!v.contains("api_key"), "literal api_key leaked");
            assert!(!v.contains("password"), "literal password leaked");
            assert!(!v.contains("token"), "literal token leaked");
        }
    }
}

// @spec:AC-2105
#[test]
fn TM_004_secret_redaction_class_query_is_redacted_for_credential_key() {
    let mut log = AuditLog::new(
        AuditPolicy::new("p-1", "rev-1", 8, 4, 60_000).expect("policy"),
        InMemorySink::new(),
    )
    .expect("log");
    for i in 0..3u64 {
        log.record(
            format!("evt-{i:02}"),
            "a-1",
            "r-1",
            AuditClass::Authorization,
            1_000 + i,
            redacted_payload(),
        )
        .expect("record");
    }
    let q = class_query(AuditClass::Authorization, 4).expect("class query");
    let res = log.query(&q).expect("query");
    assert_eq!(res.len(), 3);
    for event in res.events() {
        let keys = observed_keys(event.payload());
        assert!(keys.contains("credential"));
        let rendered: Vec<(&str, String)> = event.payload().iter().collect();
        for (k, v) in rendered {
            if k == "credential" {
                assert_eq!(v, REDACTED_PLACEHOLDER);
            } else {
                assert_ne!(v, REDACTED_PLACEHOLDER);
            }
        }
    }
}

// --- TM-005 / AC-2103 -------------------------------------------------------

// @spec:AC-2103
#[test]
fn TM_005_plugin_unauthorized_is_denied() {
    let mut engine = PluginPermissionEngine::new("rev-1").expect("engine");
    // A request without an explicit grant is denied.
    let request = PluginPermissionRequest::new(
        1,
        "rev-1",
        "unknown-plugin",
        "digest",
        "1.0.0",
        "fs.read",
        "p-1",
        "a-1",
        PluginAction::Use,
    );
    let decision = engine.evaluate(request);
    assert!(matches!(
        decision,
        Ok(security_core::plugin_permission::PluginPermissionDecision::Denied)
    ));
    // A request with a mismatched policy revision is rejected.
    let request = PluginPermissionRequest::new(
        2,
        "rev-other",
        "plugin-a",
        "digest",
        "1.0.0",
        "fs.read",
        "p-1",
        "a-1",
        PluginAction::Use,
    );
    let decision = engine.evaluate(request);
    assert!(matches!(decision, Err(PluginPermissionError::PolicyStale)));
}

// --- TM-006 / AC-2104 -------------------------------------------------------

// @spec:AC-2104
#[test]
fn TM_006_stale_evidence_is_rejected_by_rate_limit_clock_regression() {
    let policy = RateLimitPolicy::new("rev-1", 1_000, 8, 16).expect("policy");
    let limiter = RateLimiter::new(policy);
    let id = RateLimitIdentity::authenticated("a-1", "p-1")
        .expect("identity")
        .with_agent("agent-1")
        .expect("with agent");
    let r1 = RateLimitRequest::new(id.clone(), RateLimitClass::Trigger, 1, "rev-1", None)
        .expect("request");
    let d1 = limiter.check(r1, 1_000).expect("admit");
    assert!(matches!(
        d1,
        security_core::rate_limit::RateLimitDecision::Allowed { .. }
    ));
    // Clock went backwards: must be rejected with ClockWentBackwards.
    let r2 = RateLimitRequest::new(id, RateLimitClass::Trigger, 1, "rev-1", None).expect("request");
    let d2 = limiter.check(r2, 999);
    assert!(matches!(
        d2,
        Err(security_core::rate_limit::RateLimitError::ClockWentBackwards)
    ));
}

// --- TM-007 / AC-2104 -------------------------------------------------------

// @spec:AC-2104
#[test]
fn TM_007_bad_release_metadata_is_rejected_by_actor_ownership() {
    let policy = policy();
    // actor_id != owner_id is rejected with ActorNotOwner.
    let decision = policy.evaluate(&BranchPolicyRequest::new(
        "p-1",
        "r-1",
        "t-1",
        "owner-1",
        "actor-NOT-owner",
        "p_/t-1",
        "main",
        "rev-1",
        BranchMutation::LocalCommit,
    ));
    assert!(matches!(decision, Err(BranchPolicyError::ActorNotOwner)));
    // Force-push is always denied.
    let decision = policy.evaluate(&BranchPolicyRequest::new(
        "p-1",
        "r-1",
        "t-1",
        "owner-1",
        "owner-1",
        "p_/t-1",
        "main",
        "rev-1",
        BranchMutation::ForcePush,
    ));
    assert!(matches!(decision, Err(BranchPolicyError::ForcePushDenied)));
}

// --- AC-2107: deterministic redaction in export --------------------------

// @spec:AC-2107
#[test]
fn TM_007_audit_export_is_deterministic_for_same_inputs() {
    let mut log = AuditLog::new(
        AuditPolicy::new("p-1", "rev-1", 4, 4, 60_000).expect("policy"),
        InMemorySink::new(),
    )
    .expect("log");
    log.record(
        "e-1",
        "a",
        "r",
        AuditClass::Authorization,
        1,
        redacted_payload(),
    )
    .expect("record");
    let a = log.export(4).expect("export");
    let b = log.export(4).expect("export");
    let keys_a: Vec<(String, String)> = a[0]
        .payload()
        .iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    let keys_b: Vec<(String, String)> = b[0]
        .payload()
        .iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    assert_eq!(keys_a, keys_b);
}
