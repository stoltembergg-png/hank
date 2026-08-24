use agent_core::{
    AgentId, MemoryPolicy, MemoryPolicyAction, MemoryPolicyEntry, MemoryPolicyLayer,
    MemoryPolicyRequest, MemoryPolicyResolver, MemoryType, ProjectId,
};
use agent_protocol::AutonomyLevel;

fn policy(project_id: ProjectId, agent_id: AgentId, layer: MemoryPolicyLayer) -> MemoryPolicyEntry {
    MemoryPolicyEntry {
        layer,
        policy: MemoryPolicy {
            project_id,
            agent_id,
            version: 3,
            read: true,
            write: true,
            learn: false,
            allowed_types: vec![MemoryType::Semantic, MemoryType::Episodic],
            max_tokens: 100,
            max_cost_micros: 500,
            retention_days: 30,
            approval_mode: agent_core::MemoryApprovalMode::CandidateOnly,
            autonomy_level: AutonomyLevel::Assisted,
            allow_rollback: true,
            ..Default::default()
        },
    }
}

fn request(
    project_id: ProjectId,
    agent_id: AgentId,
    action: MemoryPolicyAction,
) -> MemoryPolicyRequest {
    MemoryPolicyRequest {
        project_id,
        agent_id,
        action,
        memory_type: MemoryType::Semantic,
        requested_tokens: 10,
        requested_cost_micros: 10,
    }
}

// @spec:AC-781
#[test]
fn missing_policy_and_foreign_identity_deny_by_default() {
    let project = ProjectId::new();
    let agent = AgentId::new();
    let entry = policy(project, agent, MemoryPolicyLayer::Project);
    assert!(
        !MemoryPolicyResolver::resolve(&request(project, agent, MemoryPolicyAction::Read), &[])
            .allowed
    );
    assert!(
        !MemoryPolicyResolver::resolve(
            &request(ProjectId::new(), agent, MemoryPolicyAction::Read),
            &[entry]
        )
        .allowed
    );
}

// @spec:AC-782
#[test]
fn hierarchy_is_fail_closed_when_security_or_project_denies() {
    let project = ProjectId::new();
    let agent = AgentId::new();
    let mut security = policy(project, agent, MemoryPolicyLayer::Security);
    security.policy.write = false;
    let project_policy = policy(project, agent, MemoryPolicyLayer::Project);
    let decision = MemoryPolicyResolver::resolve(
        &request(project, agent, MemoryPolicyAction::Write),
        &[project_policy, security],
    );
    assert!(!decision.allowed);
    assert_eq!(decision.layer, Some(MemoryPolicyLayer::Security));
}

// @spec:AC-783
#[test]
fn type_token_and_cost_bounds_are_enforced_without_model_override() {
    let project = ProjectId::new();
    let agent = AgentId::new();
    let entry = policy(project, agent, MemoryPolicyLayer::Agent);
    let mut oversized = request(project, agent, MemoryPolicyAction::Read);
    oversized.requested_tokens = 101;
    assert!(!MemoryPolicyResolver::resolve(&oversized, std::slice::from_ref(&entry)).allowed);
    let mut expensive = request(project, agent, MemoryPolicyAction::Read);
    expensive.requested_cost_micros = 501;
    assert!(!MemoryPolicyResolver::resolve(&expensive, &[entry]).allowed);
}

// @spec:AC-784
#[test]
fn policy_roundtrip_is_bounded_and_invalid_learning_fails() {
    let project = ProjectId::new();
    let agent = AgentId::new();
    let mut entry = policy(project, agent, MemoryPolicyLayer::Agent);
    entry.policy.learn = true;
    entry.policy.write = false;
    assert!(entry.policy.validate().is_err());
    let valid = policy(project, agent, MemoryPolicyLayer::Agent).policy;
    let encoded = serde_json::to_string(&valid).unwrap();
    let decoded: MemoryPolicy = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.version, valid.version);
    let mut value = serde_json::to_value(valid).unwrap();
    value["model_override"] = serde_json::json!("allow");
    assert!(serde_json::from_value::<MemoryPolicy>(value).is_err());
}
