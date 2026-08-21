# Session application service contract

`agent_runtime::session_service::SessionApplicationService` owns Session lifecycle and Agent turn orchestration behind the application boundary. Commands/UI do not access SQLite, Message storage, context, execution or providers directly.

## Lifecycle and authorization

`create` requires a Project/Agent-bound repository row and creates an Active Session. `open` and `close` require matching project/agent/session identity; close is terminal/idempotent. Cross-agent and closed-session operations fail before invocation.

## Send turn

`send_turn` validates Active Session and agent/session identity, checks cancellation, bounds concurrency, persists the user Message, creates an Execution, invokes an injected `TurnInvoker`, maps success/failure/cancel to explicit terminal state, and persists an Assistant Message only on success. Provider failures leave the user Message recoverable without a fabricated assistant response.

`ProviderApplicationInvoker` is the only concrete bridge and delegates to `ProviderApplicationService`; no provider adapter or UI type is imported. Tests use an invoker mock to keep the application contract provider-neutral.

Provider scope IDs are validated by the provider invocation boundary; domain `ProjectId` and provider `ProjectScopeId` remain distinct typed identities and are never compared as raw strings. Domain session authorization is enforced by `SqliteSessionRepository` and the service's project/agent/session checks.

## Tests

`crates/agent-runtime/tests/session_service_contract.rs` covers:

- create/open/close lifecycle and agent scope;
- successful send-turn with user/assistant persistence and terminal result;
- provider failure recovery with user-only persistence;
- cancellation before invocation;
- closed-session rejection without provider call.

## ONP mapping

- T-381 — Adicionar Session application service [concluida]