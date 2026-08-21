//! Contract tests for semantic ToolSchema validation and bounded payloads.

use serde_json::json;
use std::collections::BTreeMap;
use tool_core::schema::{
    SchemaCompatibility, SchemaValidationPolicy, ToolEnvironment, ToolSchema, ToolSchemaError,
};

fn schema_with(input_schema: serde_json::Value, output_schema: serde_json::Value) -> ToolSchema {
    ToolSchema {
        name: "filesystem_read".to_string(),
        version: "1.2.0".to_string(),
        description: Some("Read bounded project data".to_string()),
        input_schema,
        output_schema,
        capabilities: vec!["tool:filesystem:read".to_string()],
        destructive: false,
        environment: ToolEnvironment::Sandbox,
        timeout_seconds: 30,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        metadata: BTreeMap::new(),
    }
}

fn object_schema() -> ToolSchema {
    schema_with(
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "maxLength": 64},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100}
            },
            "required": ["path"],
            "additionalProperties": false
        }),
        json!({
            "type": "object",
            "properties": {"content": {"type": "string", "maxLength": 256}},
            "required": ["content"],
            "additionalProperties": false
        }),
    )
}

#[test]
// @spec:AC-610
fn valid_schema_passes_semantic_validation() {
    assert!(object_schema().validate().is_ok());
}

#[test]
// @spec:AC-610
fn malformed_version_and_unknown_schema_keyword_fail_closed() {
    let mut invalid_version = object_schema();
    invalid_version.version = "1.invalid".to_string();
    assert!(matches!(
        invalid_version.validate(),
        Err(ToolSchemaError::InvalidVersion)
    ));

    let mut unknown_keyword = object_schema();
    unknown_keyword.input_schema = json!({"type": "object", "x-executable": true});
    assert!(matches!(
        unknown_keyword.validate(),
        Err(ToolSchemaError::UnknownSchemaKeyword)
    ));
}

#[test]
// @spec:AC-610
fn malformed_shape_and_invalid_required_field_fail_closed() {
    let mut invalid_type = object_schema();
    invalid_type.input_schema = json!({"type": "not-a-json-type"});
    assert!(matches!(
        invalid_type.validate(),
        Err(ToolSchemaError::InvalidSchemaType)
    ));

    let mut invalid_required = object_schema();
    invalid_required.input_schema = json!({
        "type": "object",
        "properties": {"path": {"type": "string"}},
        "required": ["missing"]
    });
    assert!(matches!(
        invalid_required.validate(),
        Err(ToolSchemaError::InvalidRequiredField)
    ));
}

#[test]
// @spec:AC-611
fn input_and_output_payloads_validate_types_required_and_limits() {
    let schema = object_schema();
    assert!(
        schema
            .validate_input(
                &json!({"path": "src/lib.rs", "limit": 20}),
                SchemaValidationPolicy::strict()
            )
            .is_ok()
    );
    assert!(matches!(
        schema.validate_input(&json!({"limit": 20}), SchemaValidationPolicy::strict()),
        Err(ToolSchemaError::RequiredFieldMissing)
    ));
    assert!(matches!(
        schema.validate_input(&json!({"path": 42}), SchemaValidationPolicy::strict()),
        Err(ToolSchemaError::TypeMismatch)
    ));
    assert!(
        schema
            .validate_output(&json!({"content": "ok"}), SchemaValidationPolicy::strict())
            .is_ok()
    );
}

#[test]
// @spec:AC-611
fn payload_string_array_and_byte_limits_are_bounded() {
    let mut schema = object_schema();
    schema.max_input_bytes = 16;
    assert!(matches!(
        schema.validate_input(
            &json!({"path": "this path is too large"}),
            SchemaValidationPolicy::strict()
        ),
        Err(ToolSchemaError::PayloadTooLarge)
    ));

    let mut nested = object_schema();
    nested.input_schema = json!({
        "type": "array",
        "items": {"type": "integer"},
        "maxItems": 2
    });
    assert!(
        nested
            .validate_input(&json!([1, 2]), SchemaValidationPolicy::strict())
            .is_ok()
    );
    assert!(matches!(
        nested.validate_input(&json!([1, 2, 3]), SchemaValidationPolicy::strict()),
        Err(ToolSchemaError::ConstraintViolation)
    ));
}

#[test]
// @spec:AC-612
fn strict_and_permissive_unknown_field_policy_is_explicit() {
    let schema = object_schema();
    let payload = json!({"path": "src/lib.rs", "unknown": true});
    assert!(matches!(
        schema.validate_input(&payload, SchemaValidationPolicy::strict()),
        Err(ToolSchemaError::UnknownInputField)
    ));
    assert!(matches!(
        schema.validate_input(&payload, SchemaValidationPolicy::permissive()),
        Err(ToolSchemaError::UnknownInputField)
    ));

    let mut permissive_schema = object_schema();
    permissive_schema.input_schema = json!({
        "type": "object",
        "properties": {"path": {"type": "string"}},
        "additionalProperties": true
    });
    assert!(
        permissive_schema
            .validate_input(&payload, SchemaValidationPolicy::strict())
            .is_ok()
    );
}

#[test]
// @spec:AC-613
fn version_compatibility_has_explicit_exact_same_major_and_incompatible_states() {
    let schema = object_schema();
    assert_eq!(
        schema.compatibility_with("1.2.0").unwrap(),
        SchemaCompatibility::Exact
    );
    assert_eq!(
        schema.compatibility_with("1.9.0").unwrap(),
        SchemaCompatibility::SameMajor
    );
    assert_eq!(
        schema.compatibility_with("2.0.0").unwrap(),
        SchemaCompatibility::Incompatible
    );
    assert!(matches!(
        schema.compatibility_with("not-semver"),
        Err(ToolSchemaError::InvalidVersion)
    ));
}

#[test]
// @spec:AC-614
fn capabilities_metadata_and_sensitive_schema_names_are_rejected() {
    let mut duplicate_capability = object_schema();
    duplicate_capability
        .capabilities
        .push("tool:filesystem:read".to_string());
    assert!(matches!(
        duplicate_capability.validate(),
        Err(ToolSchemaError::DuplicateCapability)
    ));

    let mut sensitive_name = object_schema();
    sensitive_name.input_schema = json!({
        "type": "object",
        "properties": {"api_token": {"type": "string"}}
    });
    assert!(matches!(
        sensitive_name.validate(),
        Err(ToolSchemaError::SensitiveFieldName)
    ));

    let mut bad_metadata = object_schema();
    bad_metadata
        .metadata
        .insert("trace".to_string(), "\u{0001}".to_string());
    assert!(matches!(
        bad_metadata.validate(),
        Err(ToolSchemaError::InvalidMetadata)
    ));
}

#[test]
// @spec:AC-614
fn descriptions_and_schema_strings_reject_controls_and_traversal() {
    let mut control = object_schema();
    control.description = Some("unsafe\u{0007}".to_string());
    assert!(matches!(
        control.validate(),
        Err(ToolSchemaError::InvalidDescription)
    ));

    let mut traversal = object_schema();
    traversal.input_schema = json!({
        "type": "object",
        "properties": {"../secret": {"type": "string"}}
    });
    assert!(matches!(
        traversal.validate(),
        Err(ToolSchemaError::InvalidFieldName)
    ));
}

#[test]
// @spec:AC-615
fn schema_contract_does_not_return_raw_payload_in_errors() {
    let schema = object_schema();
    let result = schema.validate_input(
        &json!({
            "path": "src/lib.rs",
            "secret": "api_key=redacted"
        }),
        SchemaValidationPolicy::strict(),
    );
    let error = result.unwrap_err();
    let display = error.to_string();
    assert!(!display.contains("api_key"));
    assert!(!display.contains("secret:"));
}
