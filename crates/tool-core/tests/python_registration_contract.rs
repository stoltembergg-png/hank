use agent_protocol::ids::{ProjectId, TraceId};
use tool_core::registry::{
    RegistryError, ToolIdentity, ToolLifecycle, ToolLookupRequest, ToolOrigin, ToolRegistry,
    ToolScope,
};
use tool_core::{PythonToolRegistration, ToolEnvironment, ToolSchema};

fn declaration(project_id: ProjectId) -> PythonToolRegistration {
    PythonToolRegistration::new(
        ToolSchema {
            name: "python.demo".into(),
            version: "1.0.0".into(),
            description: Some("untrusted declaration".into()),
            input_schema: serde_json::json!({"type":"object"}),
            output_schema: serde_json::json!({"type":"object"}),
            capabilities: vec!["chat".into()],
            destructive: false,
            environment: ToolEnvironment::Python,
            timeout_seconds: 10,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            metadata: Default::default(),
        },
        "worker-1",
        project_id,
        TraceId::new(),
    )
}

#[test]
fn valid_python_declaration_registers_project_scoped_metadata_only() {
    let project = ProjectId::new();
    let registry = ToolRegistry::new();
    registry
        .register(declaration(project).into_request().unwrap())
        .unwrap();

    let descriptor = registry
        .descriptor(&ToolIdentity::project("python.demo", "1.0.0", project))
        .unwrap();
    assert_eq!(descriptor.environment, ToolEnvironment::Python);
    assert_eq!(descriptor.scope, ToolScope::Project(project));
    assert_eq!(descriptor.lifecycle, ToolLifecycle::Active);
}

#[test]
fn invalid_scope_environment_and_duplicate_are_rejected() {
    let project = ProjectId::new();
    let registry = ToolRegistry::new();
    let mut invalid = declaration(project);
    invalid.schema_mut().environment = ToolEnvironment::Host;
    assert!(matches!(
        invalid.into_request(),
        Err(tool_core::PythonRegistrationError::InvalidEnvironment)
    ));

    let request = declaration(project).into_request().unwrap();
    registry.register(request).unwrap();
    assert!(matches!(
        registry.register(declaration(project).into_request().unwrap()),
        Err(RegistryError::DuplicateIdentity { .. })
    ));
}

#[test]
fn project_origin_cannot_register_for_another_project() {
    let project = ProjectId::new();
    let other = ProjectId::new();
    let registration = declaration(project).with_origin(ToolOrigin::Project(other));
    assert!(matches!(
        registration.into_request(),
        Err(tool_core::PythonRegistrationError::UnauthorizedOrigin)
    ));
}

#[test]
fn registration_does_not_execute_or_grant_capability() {
    let project = ProjectId::new();
    let registry = ToolRegistry::new();
    registry
        .register(declaration(project).into_request().unwrap())
        .unwrap();
    let resolved = registry
        .resolve(&ToolLookupRequest::new(
            "python.demo".into(),
            "1.0.0".into(),
            project,
            Some("chat".into()),
            TraceId::new(),
        ))
        .unwrap();
    assert_eq!(resolved.schema().environment, ToolEnvironment::Python);
}
