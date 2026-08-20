# Normalized Request contract

`provider_core::request::NormalizedRequest` is the provider-neutral request envelope consumed before adapters. It does not build context, persist sessions, transport over a network, or execute tools.

## Envelope

The request carries:

- schema version;
- request and correlation IDs;
- mandatory project and agent scope plus optional session scope;
- opaque provider/model IDs;
- ordered messages with explicit roles;
- requested modalities and capability requirements;
- bounded tool metadata/fingerprints (metadata only, never execution);
- token/cost budget;
- cancellation ID/deadline metadata;
- bounded temperature option.

## Validation

`validate()` rejects:

- missing/control/oversized identity or cancellation IDs;
- empty or oversized message sets/content;
- empty modalities or malformed capability requirements;
- invalid tool IDs/fingerprints and secret-like metadata;
- invalid token/cost/temperature/deadline limits.

Total message content is bounded to 2 MiB, individual messages to 1 MiB, messages to 128, tools to 64, token budget to 1,000,000, and cost metadata to 1,000,000,000 micro-units.

## Capability gate

`validate_against_capabilities()` delegates to the capability schema before an adapter. Unsupported and unknown modalities/features, and insufficient limits, fail with typed `CapabilityError`; unknown is never promoted to supported.

## Redaction and observability

`redacted_summary()` returns request/correlation/project/agent/session IDs and bounded counts/sizes only. It never includes message text, tool payload, credential references, or raw prompt content.

## Tests

`crates/provider-core/tests/request_contract.rs` covers:

- deterministic serde roundtrip and redacted summary;
- mandatory identity/cancellation and bounds;
- message/tool/budget/numeric fail-closed validation;
- capability compatibility rejection before adapter;
- empty message/modality rejection.

## ONP mapping

- T-351 — Definir normalized request provider-neutral [concluida]