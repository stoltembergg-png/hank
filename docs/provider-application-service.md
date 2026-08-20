# Provider application/invocation service contract

`agent_runtime::provider_service` is the only provider-neutral application entry point for runtime complete/stream calls. It depends on `provider-core` contracts and does not import concrete adapter crates.

## Request boundary

`InvocationRequest` requires a validated `NormalizedRequest`, project-scoped `CredentialAccount`, `CredentialAccessContext`, and optional normalized fallback candidates. The service resolves opaque credentials, checks the registry descriptor and capability requirement, then invokes only the selected `ModelProvider` port.

Provider response/stream types are converted to runtime DTOs:

- `InvocationResult` contains attempt identity, attempt number, provider/model identity, text, finish reason and usage;
- `InvocationStreamEvent` contains attempt identity, sequence, text and terminal flag.

No adapter type, credential value, raw provider payload, endpoint, SDK or storage type crosses the boundary.

## Fallback and safety

Provider failures are classified into the fallback policy matrix. The service executes only the policy's bounded decision: retry one eligible candidate with a new `request:attempt_N` identity or return an explicit terminal fallback error. Candidate scope/capability/health/budget constraints are rechecked by `provider-core::fallback`. Authentication, invalid request, unsupported and cancellation errors do not silently retry.

Complete and stream paths check cancellation before provider resolution and before consuming stream events. Streams are collected only until one terminal event; incomplete streams become a bounded outage decision. Credential resolution occurs for every selected account, preventing unauthorized fallback across accounts/projects.

## Tests

`crates/agent-runtime/tests/provider_application_contract.rs` covers:

- provider-neutral complete DTO and redaction;
- credential/capability rejection before provider invocation;
- cancellation;
- stream terminal event and attempt identity;
- unavailable provider fallback to one eligible alternative.

## ONP mapping

- T-370 — Adicionar provider application/invocation service [concluida]