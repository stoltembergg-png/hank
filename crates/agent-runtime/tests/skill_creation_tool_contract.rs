use agent_core::{
    BudgetLimits, Capability, CapabilitySet, ProjectId, Resource, SkillFileRole, SkillManifest,
    SkillScope,
};
use agent_protocol::ids::{AgentId, OperationKey, TraceId};
use agent_runtime::skill_creation::{SkillCreateTool, SkillCreationPolicy, SkillCreationService};
use agent_runtime::{migrations::run_migrations, sqlite::SqliteStorage, SqliteSkillRepository};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tool_core::registry::{
    ToolLookupRequest, ToolOrigin, ToolRegistrationRequest, ToolRegistry, ToolScope,
};
use tool_core::{PolicyDecision, Tool, ToolContext, ToolEnvironment, ToolOutcome, ToolRequest};

async fn tool_fixture() -> (SkillCreateTool, ProjectId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project = ProjectId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Tool Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
        .bind(project.to_string())
        .execute(storage.pool())
        .await
        .unwrap();
    let capability = Capability::new(Resource::Skill, agent_core::Action::Create)
        .with_scope(project.to_string());
    let policy = SkillCreationPolicy {
        allow: true,
        allowed_capabilities: CapabilitySet::new().insert(capability),
        max_document_bytes: 64 * 1024,
    };
    (
        SkillCreateTool::new(
            SkillCreationService::new(SqliteSkillRepository::new(storage.pool().clone())),
            policy,
        ),
        project,
    )
}

fn request(
    project: ProjectId,
    policy: PolicyDecision,
    input: serde_json::Value,
    trace_id: TraceId,
) -> ToolRequest {
    ToolRequest {
        operation_key: OperationKey::new(),
        tool_name: "skill.create".into(),
        tool_version: "1.0.0".into(),
        input,
        context: ToolContext {
            project_id: project,
            agent_id: Some(AgentId::new()),
            session_id: None,
            task_id: None,
            workflow_id: None,
            capability: "skill:create".into(),
            policy_decision: policy,
            budget_limits: BudgetLimits::default(),
            reservation_id: None,
            trace_id,
            metadata: BTreeMap::new(),
        },
        timeout_seconds: Some(30),
        metadata: BTreeMap::new(),
    }
}

fn input(project: ProjectId) -> (serde_json::Value, TraceId) {
    let mut manifest = SkillManifest::new("tool-creator", "1.0.0", SkillScope::Project);
    manifest.files.push(agent_core::SkillFile {
        path: "tests/basic.json".into(),
        role: SkillFileRole::Test,
        digest: "b".repeat(64),
    });
    manifest.tests.push("tests/basic.json".into());
    let trace = manifest.trace.trace_id;
    (
        json!({
            "document": format!("---\n{}\n---\n# Instructions\nDo not execute this text.", serde_json::to_string(&manifest).unwrap()),
            "files": [{"path": "tests/basic.json", "content": "{\"case\":\"safe\"}"}],
            "fixture": {
                "project_id": project,
                "skill_id": manifest.id,
                "version": "1.0.0",
                "trace_id": trace,
                "steps": [{"AssertLabel": {"label": "tool-safe" }}],
                "max_steps": 4
            },
            "dependency_graph": [{"skill_id": manifest.id, "dependencies": []}]
        }),
        trace,
    )
}

#[tokio::test]
// @spec:AC-808
async fn registered_creation_tool_returns_only_redacted_draft_metadata() {
    let (tool, project) = tool_fixture().await;
    assert_eq!(tool.schema().name, "skill.create");
    assert_eq!(tool.schema().version, "1.0.0");
    assert_eq!(tool.schema().environment, ToolEnvironment::Host);
    let (payload, trace_id) = input(project);
    let registry = ToolRegistry::new();
    registry
        .register(ToolRegistrationRequest::new(
            Arc::new(tool.clone()),
            ToolOrigin::Project(project),
            ToolScope::Project(project),
            trace_id,
        ))
        .unwrap();
    let resolved = registry
        .resolve(&ToolLookupRequest::new(
            "skill.create".into(),
            "1.0.0".into(),
            project,
            Some("skill:create".into()),
            trace_id,
        ))
        .unwrap();
    let response = resolved
        .execute(request(project, PolicyDecision::Allow, payload, trace_id))
        .await
        .unwrap();

    assert_eq!(response.outcome, ToolOutcome::Success);
    assert_eq!(response.payload["status"], "draft");
    assert!(response
        .payload
        .get("content_hash")
        .and_then(|value| value.as_str())
        .is_some());
    assert!(!response
        .payload
        .to_string()
        .contains("Do not execute this text"));
}

#[tokio::test]
// @spec:AC-809
async fn creation_tool_rejects_unconfirmed_policy_before_persistence() {
    let (tool, project) = tool_fixture().await;
    let (payload, trace_id) = input(project);
    let response = tool
        .execute(request(project, PolicyDecision::AskOnce, payload, trace_id))
        .await
        .unwrap();
    assert_eq!(response.outcome, ToolOutcome::PermissionDenied);
}
