//! Contract tests for the bounded, project-isolated ToolRegistry.

use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use async_trait::async_trait;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use tool_core::registry::{
    RegistryError, ToolIdentity, ToolLifecycle, ToolLookupRequest, ToolOrigin,
    ToolRegistrationRequest, ToolRegistry, ToolScope,
};
use tool_core::response::{ToolOutcome, ToolResponse};
use tool_core::schema::{ToolEnvironment, ToolSchema};
use tool_core::trait_def::Tool;
use tool_core::{ToolError, ToolRequest};

struct MockTool {
    schema: ToolSchema,
    executions: Arc<AtomicUsize>,
}

impl MockTool {
    fn new(name: &str, version: &str, capability: &str) -> Arc<Self> {
        Arc::new(Self {
            schema: ToolSchema {
                name: name.to_string(),
                version: version.to_string(),
                description: Some("untrusted test description".to_string()),
                input_schema: json!({"type": "object", "properties": {}, "additionalProperties": true}),
                output_schema: json!({"type": "object", "properties": {}, "additionalProperties": true}),
                capabilities: vec![capability.to_string()],
                destructive: false,
                environment: ToolEnvironment::Sandbox,
                timeout_seconds: 30,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                metadata: BTreeMap::new(),
            },
            executions: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn invalid(name: &str) -> Arc<Self> {
        let tool = Self::new(name, "1.0.0", "tool:test:read");
        let mut schema = tool.schema.clone();
        schema.version.clear();
        Arc::new(Self {
            schema,
            executions: tool.executions.clone(),
        })
    }
}

#[async_trait]
impl Tool for MockTool {
    fn schema(&self) -> &'static ToolSchema {
        Box::leak(Box::new(self.schema.clone()))
    }

    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        Ok(ToolResponse {
            operation_key: request.operation_key,
            tool_name: request.tool_name,
            tool_version: request.tool_version,
            outcome: ToolOutcome::Success,
            payload: json!({"ok": true}),
            trace_id: request.context.trace_id,
            duration_ms: 1,
            metadata: BTreeMap::new(),
        })
    }
}

fn register_global(registry: &ToolRegistry, tool: Arc<MockTool>) -> ToolIdentity {
    let identity = ToolIdentity::global(tool.schema.name.clone(), tool.schema.version.clone());
    registry
        .register(ToolRegistrationRequest::new(
            tool,
            ToolOrigin::Builtin,
            ToolScope::Global,
            TraceId::new(),
        ))
        .unwrap();
    identity
}

fn lookup(
    identity: &ToolIdentity,
    project_id: ProjectId,
    capability: Option<&str>,
) -> ToolLookupRequest {
    ToolLookupRequest::new(
        identity.name.clone(),
        identity.version.clone(),
        project_id,
        capability.map(str::to_string),
        TraceId::new(),
    )
}

#[test]
// @spec:AC-616
fn valid_registration_indexes_without_executing_handler() {
    let registry = ToolRegistry::new();
    let tool = MockTool::new("read", "1.0.0", "tool:test:read");
    let executions = tool.executions.clone();
    let identity = register_global(&registry, tool);
    let descriptor = registry.descriptor(&identity).unwrap();
    assert_eq!(descriptor.lifecycle, ToolLifecycle::Active);
    assert_eq!(descriptor.name, "read");
    assert_eq!(executions.load(Ordering::SeqCst), 0);
}

#[test]
// @spec:AC-617
fn duplicate_invalid_origin_and_capacity_fail_without_mutation() {
    let registry = ToolRegistry::with_capacity(1).unwrap();
    let first = MockTool::new("read", "1.0.0", "tool:test:read");
    let identity = register_global(&registry, first);
    let duplicate = MockTool::new("read", "1.0.0", "tool:test:read");
    assert!(matches!(
        registry.register(ToolRegistrationRequest::new(
            duplicate,
            ToolOrigin::Builtin,
            ToolScope::Global,
            TraceId::new(),
        )),
        Err(RegistryError::DuplicateIdentity { .. })
    ));

    let project_a = ProjectId::new();
    let project_b = ProjectId::new();
    let mismatched = MockTool::new("other", "1.0.0", "tool:test:read");
    assert!(matches!(
        registry.register(ToolRegistrationRequest::new(
            mismatched,
            ToolOrigin::Project(project_a),
            ToolScope::Project(project_b),
            TraceId::new(),
        )),
        Err(RegistryError::UnauthorizedOrigin)
    ));

    let invalid = MockTool::invalid("invalid");
    assert!(matches!(
        registry.register(ToolRegistrationRequest::new(
            invalid,
            ToolOrigin::Builtin,
            ToolScope::Global,
            TraceId::new(),
        )),
        Err(RegistryError::SchemaInvalid(_))
    ));
    assert_eq!(registry.descriptor(&identity).unwrap().name, "read");
}

#[test]
// @spec:AC-618
fn lookup_is_project_isolated_with_project_precedence_and_deterministic_listing() {
    let registry = ToolRegistry::new();
    let project_a = ProjectId::new();
    let project_b = ProjectId::new();
    let global = MockTool::new("read", "1.0.0", "tool:test:read");
    let global_identity = register_global(&registry, global);
    let project_tool = MockTool::new("read", "1.0.0", "tool:test:project");
    let project_identity = ToolIdentity::project("read", "1.0.0", project_a);
    registry
        .register(ToolRegistrationRequest::new(
            project_tool,
            ToolOrigin::Project(project_a),
            ToolScope::Project(project_a),
            TraceId::new(),
        ))
        .unwrap();

    let resolved_a = registry
        .resolve(&lookup(
            &global_identity,
            project_a,
            Some("tool:test:project"),
        ))
        .unwrap();
    assert_eq!(resolved_a.schema().name, "read");
    assert!(
        registry
            .resolve(&lookup(
                &project_identity,
                project_b,
                Some("tool:test:project"),
            ))
            .is_err()
    );
    assert!(
        registry
            .list_visible(&project_a)
            .unwrap()
            .iter()
            .any(|item| item.scope == ToolScope::Project(project_a))
    );
    let visible_b = registry.list_visible(&project_b).unwrap();
    assert!(
        visible_b
            .iter()
            .all(|item| item.scope != ToolScope::Project(project_a))
    );
}

#[test]
// @spec:AC-618
fn capability_mismatch_and_missing_lookup_are_typed() {
    let registry = ToolRegistry::new();
    let tool = MockTool::new("read", "1.0.0", "tool:test:read");
    let identity = register_global(&registry, tool);
    let project = ProjectId::new();
    assert!(matches!(
        registry.resolve(&lookup(&identity, project, Some("tool:test:write"))),
        Err(RegistryError::CapabilityMismatch)
    ));
    let missing = ToolIdentity::global("missing", "1.0.0");
    assert!(matches!(
        registry.resolve(&lookup(&missing, project, None)),
        Err(RegistryError::NotFound { .. })
    ));
}

#[test]
// @spec:AC-619
fn lifecycle_disables_resolution_but_keeps_metadata() {
    let registry = ToolRegistry::new();
    let tool = MockTool::new("read", "1.0.0", "tool:test:read");
    let identity = register_global(&registry, tool);
    let project = ProjectId::new();
    registry
        .set_lifecycle(&identity, ToolLifecycle::Disabled)
        .unwrap();
    assert!(matches!(
        registry.resolve(&lookup(&identity, project, None)),
        Err(RegistryError::NotActive { .. })
    ));
    assert_eq!(
        registry.descriptor(&identity).unwrap().lifecycle,
        ToolLifecycle::Disabled
    );
    registry
        .set_lifecycle(&identity, ToolLifecycle::Retired)
        .unwrap();
    assert!(matches!(
        registry.resolve(&lookup(&identity, project, None)),
        Err(RegistryError::NotActive { .. })
    ));
}

#[test]
// @spec:AC-620
fn unregister_and_restore_roundtrip_is_bounded() {
    let registry = ToolRegistry::new();
    let tool = MockTool::new("read", "1.0.0", "tool:test:read");
    let identity = register_global(&registry, tool);
    let removed = registry.unregister(&identity).unwrap();
    assert!(registry.descriptor(&identity).is_err());
    registry.restore(removed).unwrap();
    assert_eq!(
        registry.descriptor(&identity).unwrap().lifecycle,
        ToolLifecycle::Active
    );
    assert!(matches!(
        registry.unregister(&ToolIdentity::global("missing", "1.0.0")),
        Err(RegistryError::NotFound { .. })
    ));
}

#[test]
// @spec:AC-621
fn seal_blocks_mutations_but_allows_reads() {
    let registry = ToolRegistry::new();
    let tool = MockTool::new("read", "1.0.0", "tool:test:read");
    let identity = register_global(&registry, tool);
    registry.seal().unwrap();
    assert!(registry.is_sealed());
    assert!(registry.descriptor(&identity).is_ok());
    assert!(matches!(
        registry.set_lifecycle(&identity, ToolLifecycle::Disabled),
        Err(RegistryError::Sealed)
    ));
    let second = MockTool::new("second", "1.0.0", "tool:test:read");
    assert!(matches!(
        registry.register(ToolRegistrationRequest::new(
            second,
            ToolOrigin::Builtin,
            ToolScope::Global,
            TraceId::new(),
        )),
        Err(RegistryError::Sealed)
    ));
}

#[test]
// @spec:AC-621
fn concurrent_registration_and_reads_are_thread_safe() {
    let registry = Arc::new(ToolRegistry::new());
    let mut handles = Vec::new();
    for index in 0..8 {
        let registry = registry.clone();
        handles.push(thread::spawn(move || {
            let project = ProjectId::new();
            let name = format!("read-{index}");
            let tool = MockTool::new(&name, "1.0.0", "tool:test:read");
            registry
                .register(ToolRegistrationRequest::new(
                    tool,
                    ToolOrigin::Project(project),
                    ToolScope::Project(project),
                    TraceId::new(),
                ))
                .unwrap();
            assert_eq!(registry.list_visible(&project).unwrap().len(), 1);
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(registry.len().unwrap(), 8);
}

#[test]
// @spec:AC-622
fn registry_never_executes_handler_or_trusts_description_metadata() {
    let registry = ToolRegistry::new();
    let tool = MockTool::new("read", "1.0.0", "tool:test:read");
    let executions = tool.executions.clone();
    let identity = register_global(&registry, tool);
    let _ = registry.descriptor(&identity).unwrap();
    let _ = registry.list_all().unwrap();
    assert_eq!(executions.load(Ordering::SeqCst), 0);
}

#[test]
// @spec:AC-623
fn deterministic_list_and_visible_capability_filter_are_bounded() {
    let registry = ToolRegistry::new();
    let a = MockTool::new("a", "1.0.0", "tool:test:read");
    let b = MockTool::new("b", "1.0.0", "tool:test:write");
    register_global(&registry, b);
    register_global(&registry, a);
    let all = registry.list_all().unwrap();
    assert_eq!(
        all.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    let visible = registry
        .list_by_capability(&ProjectId::new(), "tool:test:read")
        .unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].name, "a");
}
