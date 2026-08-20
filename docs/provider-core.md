# Provider Core ModelProvider contract

`provider-core` is the provider-neutral contract boundary for future model adapters. It contains no HTTP client, SDK, network implementation, credential storage, or real provider claim.

## Workspace boundary

- New crate: `crates/provider-core`;
- `agent-core` and `agent-runtime` do not depend on concrete provider SDKs;
- The older runtime provider stub remains outside this migration card and is not used as the provider-core contract;
- The architecture fixture explicitly includes `provider-core` in the workspace package set.

## Opaque identifiers

- `ProviderId` and `ModelId` are bounded, validated identifiers;
- URLs, endpoint syntax, control characters, and secret-like markers are rejected;
- `CredentialRef` is an opaque `cred_...` reference only;
- Credential references reject key/token/password/secret-like values;
- `CredentialRef` Debug and Display never reveal its value.

## ModelProvider trait

The object-safe `ModelProvider` trait exposes:

- provider ID and contract version;
- bounded capability report;
- normalized completion request/response;
- streaming events with sequence and terminal marker;
- model listing;
- health hook;
- explicit cancellation token;
- bounded stream/backpressure configuration;
- typed unsupported, cancellation, backpressure, unavailable, and invalid-input errors.

All async operations use boxed futures/streams so adapters can be held behind `Box<dyn ModelProvider>` without a concrete SDK type.

## MockProvider

`MockProvider` is a deterministic contract fixture only. It returns local mock completion/stream/model-list/health results and never calls a network or real provider.

## Security and observability

- No secret/token/prompt payload is included in error text or credential debug output;
- Request IDs, model IDs, and provider IDs remain structured for correlation;
- Provider operation errors are typed and bounded;
- Cancellation and backpressure are explicit rather than silently dropped.

## Tests

`crates/provider-core/tests/provider_contract.rs` covers:

- object-safe trait compilation and MockProvider lifecycle;
- complete, stream, list-models, and health operations;
- cancellation;
- opaque identifier and credential-reference validation/redaction;
- request serialization and bounded stream/request inputs;
- typed unsupported operation errors.

The architecture fixture test also proves the workspace package list includes `provider-core`.

## ONP mapping

- T-349 — Definir trait `ModelProvider` provider-neutral [concluida]