# Tool schema contract (PR-097)

`tool_core::ToolSchema` is the provider-neutral declaration consumed before a handler or registry resolution. It describes the tool identity/version, input/output JSON Schema, capabilities, destructive flag, environment, timeout, payload limits and bounded metadata.

## Semantic schema validation

`ToolSchema::validate()` is fail-closed and deterministic:

- tool name and semantic version are bounded and reject controls/path traversal;
- timeout and input/output byte limits are finite;
- capabilities are required, bounded, unique and free of control/traversal data;
- descriptions, titles and metadata are bounded; metadata rejects control characters and sensitive names;
- input/output schemas must be bounded JSON objects with an allowlisted keyword set;
- nested schema depth/properties and required declarations are bounded;
- unsupported or malformed schema keywords/types fail before a handler can run.

Descriptions, examples and metadata are untrusted data. They do not change capability, environment, destructive state or policy.

## Payload validation

`validate_input` and `validate_output` first validate the schema and payload byte bound, then recursively validate types, required fields, enum/const, string and array limits, numeric limits, nested objects and depth.

`SchemaValidationPolicy::strict()` rejects undeclared object fields unless the schema explicitly allows `additionalProperties`. `permissive()` allows undeclared fields only when the schema does not explicitly prohibit them; `additionalProperties: false` always wins.

Errors are categorized without embedding raw payloads: `TypeMismatch`, `RequiredFieldMissing`, `UnknownInputField`, `PayloadTooLarge`, `ConstraintViolation`, and bounded schema errors.

## Version compatibility

`compatibility_with` parses semantic versions and returns:

- `Exact` for the same version;
- `SameMajor` for a different version with the same major;
- `Incompatible` for a different major;
- `InvalidVersion` for malformed requests.

Registry lookup and handler policy may choose exact-only behavior later; this PR only defines the normalized compatibility result.

## Security and non-goals

This PR does not execute tools, resolve registries, evaluate permissions, access filesystem/network, or implement handlers. It rejects sensitive field names such as password/token/secret/api_key/credential in declared properties and metadata. It does not infer trust from descriptions or examples.

## Tests

`crates/tool-core/tests/schema_contract.rs` contains 10 contract tests covering valid/malformed schemas, semantic versions, recursive shape, payload bounds, required/type/array constraints, strict/permissive unknown fields, compatibility, sensitive metadata and error redaction. Existing `trait_contract.rs` remains green with all 34 tests.