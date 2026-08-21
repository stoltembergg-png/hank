//! Tool schema definition, semantic validation, bounded payload validation,
//! and explicit version compatibility.

use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_SCHEMA_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_DEPTH: usize = 16;
const MAX_SCHEMA_PROPERTIES: usize = 128;
const MAX_METADATA_ENTRIES: usize = 64;
const MAX_TEXT_BYTES: usize = 4 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TIMEOUT_SECONDS: u64 = 86_400;
const MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

/// Tool schema describing input, output, capabilities, and constraints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSchema {
    /// Tool name (unique within registry).
    pub name: String,
    /// Semantic tool version.
    pub version: String,
    /// Human-readable description (untrusted data).
    pub description: Option<String>,
    /// Input JSON schema.
    #[schemars(with = "serde_json::Value")]
    pub input_schema: Value,
    /// Output JSON schema.
    #[schemars(with = "serde_json::Value")]
    pub output_schema: Value,
    /// Declared capabilities this tool provides/requires.
    pub capabilities: Vec<String>,
    /// Whether the tool has destructive side effects.
    pub destructive: bool,
    /// Required execution environment.
    pub environment: ToolEnvironment,
    /// Default timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum input payload size in bytes.
    pub max_input_bytes: usize,
    /// Maximum output payload size in bytes.
    pub max_output_bytes: usize,
    /// Additional non-sensitive metadata.
    pub metadata: BTreeMap<String, String>,
}

/// Execution environment requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolEnvironment {
    /// Runs in the host process (no sandbox).
    Host,
    /// Runs in a sandboxed process.
    Sandbox,
    /// Runs in a Python worker.
    Python,
    /// Runs in a remote/external process.
    Remote,
}

/// Policy controlling undeclared object fields in payload validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SchemaValidationPolicy {
    /// Reject fields omitted from `properties` unless the schema explicitly
    /// sets `additionalProperties` to true or to a validating schema.
    pub reject_unknown_fields: bool,
}

impl SchemaValidationPolicy {
    /// Strict, fail-closed validation policy.
    pub const fn strict() -> Self {
        Self {
            reject_unknown_fields: true,
        }
    }

    /// Permissive policy; an explicit `additionalProperties: false` still wins.
    pub const fn permissive() -> Self {
        Self {
            reject_unknown_fields: false,
        }
    }
}

impl Default for SchemaValidationPolicy {
    fn default() -> Self {
        Self::strict()
    }
}

/// Compatibility result for a requested tool version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SchemaCompatibility {
    /// Requested version equals the schema version.
    Exact,
    /// Requested version has the same major version.
    SameMajor,
    /// Requested version has a different major version.
    Incompatible,
}

impl ToolSchema {
    /// Validates the schema itself, including semantic version, shape,
    /// capabilities, limits, metadata, and bounded JSON Schema keywords.
    pub fn validate(&self) -> Result<(), ToolSchemaError> {
        if !valid_identifier(&self.name) {
            return Err(ToolSchemaError::MissingName);
        }
        parse_version(&self.version)?;
        if let Some(description) = &self.description {
            validate_text(description, MAX_TEXT_BYTES)
                .map_err(|_| ToolSchemaError::InvalidDescription)?;
        }
        if self.timeout_seconds == 0 || self.timeout_seconds > MAX_TIMEOUT_SECONDS {
            return Err(ToolSchemaError::InvalidTimeout);
        }
        if self.max_input_bytes == 0
            || self.max_output_bytes == 0
            || self.max_input_bytes > MAX_PAYLOAD_BYTES
            || self.max_output_bytes > MAX_PAYLOAD_BYTES
        {
            return Err(ToolSchemaError::InvalidPayloadLimit);
        }
        if self.capabilities.is_empty() {
            return Err(ToolSchemaError::InvalidCapability);
        }
        let mut capabilities = BTreeSet::new();
        for capability in &self.capabilities {
            if !valid_identifier(capability) || !capabilities.insert(capability.as_str()) {
                return if capabilities.contains(capability.as_str()) {
                    Err(ToolSchemaError::DuplicateCapability)
                } else {
                    Err(ToolSchemaError::InvalidCapability)
                };
            }
        }
        validate_metadata(&self.metadata)?;
        validate_schema_root(&self.input_schema, ToolSchemaError::InvalidInputSchema)?;
        validate_schema_root(&self.output_schema, ToolSchemaError::InvalidOutputSchema)?;
        Ok(())
    }

    /// Validates an input payload against the input schema and policy.
    pub fn validate_input(
        &self,
        payload: &Value,
        policy: SchemaValidationPolicy,
    ) -> Result<(), ToolSchemaError> {
        self.validate()?;
        validate_payload_size(payload, self.max_input_bytes)?;
        validate_value(payload, &self.input_schema, policy, 0)
    }

    /// Validates an output payload against the output schema and policy.
    pub fn validate_output(
        &self,
        payload: &Value,
        policy: SchemaValidationPolicy,
    ) -> Result<(), ToolSchemaError> {
        self.validate()?;
        validate_payload_size(payload, self.max_output_bytes)?;
        validate_value(payload, &self.output_schema, policy, 0)
    }

    /// Returns the explicit compatibility relation for a requested version.
    pub fn compatibility_with(
        &self,
        requested_version: &str,
    ) -> Result<SchemaCompatibility, ToolSchemaError> {
        let schema_version = parse_version(&self.version)?;
        let requested = parse_version(requested_version)?;
        Ok(if requested == schema_version {
            SchemaCompatibility::Exact
        } else if requested.major == schema_version.major {
            SchemaCompatibility::SameMajor
        } else {
            SchemaCompatibility::Incompatible
        })
    }
}

fn parse_version(value: &str) -> Result<Version, ToolSchemaError> {
    if value.trim().is_empty() {
        return Err(ToolSchemaError::MissingVersion);
    }
    if !valid_identifier(value) {
        return Err(ToolSchemaError::InvalidVersion);
    }
    Version::parse(value).map_err(|_| ToolSchemaError::InvalidVersion)
}

fn validate_schema_root(value: &Value, root_error: ToolSchemaError) -> Result<(), ToolSchemaError> {
    if !value.is_object() {
        return Err(root_error);
    }
    let encoded = serde_json::to_vec(value).map_err(|_| ToolSchemaError::InvalidSchemaShape)?;
    if encoded.len() > MAX_SCHEMA_BYTES {
        return Err(ToolSchemaError::InvalidPayloadLimit);
    }
    validate_schema_node(value, 0)
}

fn validate_schema_node(value: &Value, depth: usize) -> Result<(), ToolSchemaError> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(ToolSchemaError::DepthExceeded);
    }
    let object = value
        .as_object()
        .ok_or(ToolSchemaError::InvalidSchemaShape)?;
    const ALLOWED: &[&str] = &[
        "type",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "enum",
        "const",
        "minLength",
        "maxLength",
        "minItems",
        "maxItems",
        "minimum",
        "maximum",
        "title",
        "description",
        "default",
        "examples",
    ];
    for key in object.keys() {
        if !ALLOWED.contains(&key.as_str()) {
            return Err(ToolSchemaError::UnknownSchemaKeyword);
        }
    }

    if let Some(type_value) = object.get("type") {
        let type_name = type_value
            .as_str()
            .ok_or(ToolSchemaError::InvalidSchemaType)?;
        if !matches!(
            type_name,
            "object" | "array" | "string" | "number" | "integer" | "boolean" | "null"
        ) {
            return Err(ToolSchemaError::InvalidSchemaType);
        }
    }
    if let Some(properties) = object.get("properties") {
        let properties = properties
            .as_object()
            .ok_or(ToolSchemaError::InvalidSchemaShape)?;
        if properties.len() > MAX_SCHEMA_PROPERTIES {
            return Err(ToolSchemaError::InvalidPayloadLimit);
        }
        for (name, property_schema) in properties {
            validate_field_name(name)?;
            validate_schema_node(property_schema, depth + 1)?;
        }
    }
    if let Some(required) = object.get("required") {
        let required = required
            .as_array()
            .ok_or(ToolSchemaError::InvalidRequiredField)?;
        let properties = object.get("properties").and_then(Value::as_object);
        let mut names = BTreeSet::new();
        for value in required {
            let name = value
                .as_str()
                .ok_or(ToolSchemaError::InvalidRequiredField)?;
            if !names.insert(name) || properties.is_none_or(|props| !props.contains_key(name)) {
                return Err(ToolSchemaError::InvalidRequiredField);
            }
        }
    }
    if let Some(additional) = object.get("additionalProperties") {
        match additional {
            Value::Bool(_) => {}
            Value::Object(_) => validate_schema_node(additional, depth + 1)?,
            _ => return Err(ToolSchemaError::InvalidSchemaShape),
        }
    }
    if let Some(items) = object.get("items") {
        validate_schema_node(items, depth + 1)?;
    }
    validate_numeric_constraint(object, "minLength")?;
    validate_numeric_constraint(object, "maxLength")?;
    validate_numeric_constraint(object, "minItems")?;
    validate_numeric_constraint(object, "maxItems")?;
    validate_numeric_constraint(object, "minimum")?;
    validate_numeric_constraint(object, "maximum")?;
    if let (Some(min), Some(max)) = (object.get("minLength"), object.get("maxLength"))
        && min.as_u64() > max.as_u64()
    {
        return Err(ToolSchemaError::ConstraintViolation);
    }
    if let (Some(min), Some(max)) = (object.get("minItems"), object.get("maxItems"))
        && min.as_u64() > max.as_u64()
    {
        return Err(ToolSchemaError::ConstraintViolation);
    }
    if let (Some(min), Some(max)) = (object.get("minimum"), object.get("maximum"))
        && min.as_f64() > max.as_f64()
    {
        return Err(ToolSchemaError::ConstraintViolation);
    }
    for key in ["title", "description"] {
        if let Some(value) = object.get(key) {
            let text = value.as_str().ok_or(ToolSchemaError::InvalidDescription)?;
            validate_text(text, MAX_TEXT_BYTES).map_err(|_| ToolSchemaError::InvalidDescription)?;
        }
    }
    if let Some(enum_values) = object.get("enum")
        && !enum_values.is_array()
    {
        return Err(ToolSchemaError::InvalidSchemaShape);
    }
    if let Some(examples) = object.get("examples")
        && !examples.is_array()
    {
        return Err(ToolSchemaError::InvalidSchemaShape);
    }
    Ok(())
}

fn validate_numeric_constraint(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<(), ToolSchemaError> {
    if let Some(value) = object.get(key) {
        let valid = if matches!(key, "minimum" | "maximum") {
            value.is_number()
        } else {
            value.as_u64().is_some()
        };
        if !valid {
            return Err(ToolSchemaError::ConstraintViolation);
        }
    }
    Ok(())
}

fn validate_value(
    value: &Value,
    schema: &Value,
    policy: SchemaValidationPolicy,
    depth: usize,
) -> Result<(), ToolSchemaError> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(ToolSchemaError::DepthExceeded);
    }
    let object = schema
        .as_object()
        .ok_or(ToolSchemaError::InvalidSchemaShape)?;
    if let Some(expected) = object.get("type").and_then(Value::as_str) {
        let matches = match expected {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => false,
        };
        if !matches {
            return Err(ToolSchemaError::TypeMismatch);
        }
    }
    if let Some(enum_values) = object.get("enum").and_then(Value::as_array)
        && !enum_values.iter().any(|candidate| candidate == value)
    {
        return Err(ToolSchemaError::ConstraintViolation);
    }
    if let Some(constant) = object.get("const")
        && constant != value
    {
        return Err(ToolSchemaError::ConstraintViolation);
    }
    if let Some(text) = value.as_str() {
        if let Some(min) = object.get("minLength").and_then(Value::as_u64)
            && text.chars().count() < min as usize
        {
            return Err(ToolSchemaError::ConstraintViolation);
        }
        if let Some(max) = object.get("maxLength").and_then(Value::as_u64)
            && text.chars().count() > max as usize
        {
            return Err(ToolSchemaError::ConstraintViolation);
        }
    }
    if let Some(number) = value.as_f64() {
        if let Some(min) = object.get("minimum").and_then(Value::as_f64)
            && number < min
        {
            return Err(ToolSchemaError::ConstraintViolation);
        }
        if let Some(max) = object.get("maximum").and_then(Value::as_f64)
            && number > max
        {
            return Err(ToolSchemaError::ConstraintViolation);
        }
    }
    if let Some(array) = value.as_array() {
        if let Some(min) = object.get("minItems").and_then(Value::as_u64)
            && array.len() < min as usize
        {
            return Err(ToolSchemaError::ConstraintViolation);
        }
        if let Some(max) = object.get("maxItems").and_then(Value::as_u64)
            && array.len() > max as usize
        {
            return Err(ToolSchemaError::ConstraintViolation);
        }
        if let Some(items) = object.get("items") {
            for item in array {
                validate_value(item, items, policy, depth + 1)?;
            }
        }
    }
    if let Some(map) = value.as_object() {
        let properties = object.get("properties").and_then(Value::as_object);
        for required in object
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let name = required
                .as_str()
                .ok_or(ToolSchemaError::InvalidRequiredField)?;
            if !map.contains_key(name) {
                return Err(ToolSchemaError::RequiredFieldMissing);
            }
        }
        for (name, child) in map {
            if let Some(child_schema) = properties.and_then(|props| props.get(name)) {
                validate_value(child, child_schema, policy, depth + 1)?;
                continue;
            }
            match object.get("additionalProperties") {
                Some(Value::Bool(false)) => return Err(ToolSchemaError::UnknownInputField),
                Some(Value::Bool(true)) => {}
                Some(additional_schema @ Value::Object(_)) => {
                    validate_value(child, additional_schema, policy, depth + 1)?;
                }
                None if policy.reject_unknown_fields => {
                    return Err(ToolSchemaError::UnknownInputField);
                }
                None => {}
                _ => return Err(ToolSchemaError::InvalidSchemaShape),
            }
        }
    }
    Ok(())
}

fn validate_metadata(metadata: &BTreeMap<String, String>) -> Result<(), ToolSchemaError> {
    if metadata.len() > MAX_METADATA_ENTRIES {
        return Err(ToolSchemaError::InvalidMetadata);
    }
    for (key, value) in metadata {
        if !valid_identifier(key)
            || value.len() > MAX_TEXT_BYTES
            || value.chars().any(char::is_control)
            || is_sensitive_name(key)
        {
            return Err(ToolSchemaError::InvalidMetadata);
        }
    }
    Ok(())
}

fn validate_field_name(name: &str) -> Result<(), ToolSchemaError> {
    if !valid_identifier(name) {
        return Err(ToolSchemaError::InvalidFieldName);
    }
    if is_sensitive_name(name) {
        return Err(ToolSchemaError::SensitiveFieldName);
    }
    Ok(())
}

fn validate_text(text: &str, max_bytes: usize) -> Result<(), ToolSchemaError> {
    if text.len() > max_bytes || text.chars().any(char::is_control) {
        return Err(ToolSchemaError::InvalidDescription);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && !value.chars().any(char::is_control)
        && !value.contains("..")
        && !value.contains('/')
        && !value.contains('\\')
}

fn is_sensitive_name(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "password",
        "secret",
        "token",
        "api_key",
        "client_secret",
        "credential",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

/// Errors during schema validation.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum ToolSchemaError {
    #[error("schema name is required")]
    MissingName,
    #[error("schema version is required or invalid")]
    InvalidVersion,
    #[error("schema version is required")]
    MissingVersion,
    #[error("timeout must be > 0 and bounded")]
    InvalidTimeout,
    #[error("payload limits must be > 0 and bounded")]
    InvalidPayloadLimit,
    #[error("input_schema must be a JSON object")]
    InvalidInputSchema,
    #[error("output_schema must be a JSON object")]
    InvalidOutputSchema,
    #[error("schema shape is invalid")]
    InvalidSchemaShape,
    #[error("schema keyword is not allowlisted")]
    UnknownSchemaKeyword,
    #[error("schema type is invalid")]
    InvalidSchemaType,
    #[error("required field declaration is invalid")]
    InvalidRequiredField,
    #[error("payload exceeds its declared limit")]
    PayloadTooLarge,
    #[error("schema constraint is invalid or violated")]
    ConstraintViolation,
    #[error("payload contains an unknown field")]
    UnknownInputField,
    #[error("payload type does not match schema")]
    TypeMismatch,
    #[error("required payload field is missing")]
    RequiredFieldMissing,
    #[error("capability declaration is invalid")]
    InvalidCapability,
    #[error("capability declaration is duplicated")]
    DuplicateCapability,
    #[error("schema field name is invalid")]
    InvalidFieldName,
    #[error("schema field name is sensitive")]
    SensitiveFieldName,
    #[error("schema metadata is invalid")]
    InvalidMetadata,
    #[error("schema description is invalid")]
    InvalidDescription,
    #[error("schema nesting exceeds the bounded depth")]
    DepthExceeded,
}

fn validate_payload_size(payload: &Value, limit: usize) -> Result<(), ToolSchemaError> {
    let bytes = serde_json::to_vec(payload).map_err(|_| ToolSchemaError::InvalidSchemaShape)?;
    if bytes.len() > limit {
        Err(ToolSchemaError::PayloadTooLarge)
    } else {
        Ok(())
    }
}
