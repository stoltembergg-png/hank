# Cancellation boundary contract

`agent_runtime::cancellation` is the provider-neutral cancellation boundary for an Agent turn. It owns bounded execution-id to `CancellationToken` registration and synchronizes cancellation with the Execution and Message state machines.

## Registry

`CancellationRegistry` has a fixed capacity, validates bounded non-control execution IDs, rejects duplicate registration, exposes idempotent cancel/unregister and uses a mutex-protected map safe for concurrent callers. `CancellationHandle` exposes only cancellation state/token; the custom Debug output contains IDs and capacity, never payloads.

## Turn cancellation

`cancel_turn` cancels the token first, then transitions active Execution and Message to terminal Cancelled exactly once. Repeated cancellation returns `AlreadyCancelled`; a turn already Completed/Failed returns `AlreadyTerminal` and is never overwritten. The boundary does not kill processes, call provider-specific APIs, retry or mutate UI state.

## Tests

`crates/agent-runtime/tests/cancellation_contract.rs` covers:

- register/cancel idempotence/unregister;
- capacity and identity bounds;
- synchronized Execution/Message cancellation;
- completion-wins race and no terminal overwrite;
- redacted diagnostics;
- concurrent registry operations.

## ONP mapping

- T-379 — Adicionar cancellation boundary [concluida]