# Retry policy contract

`agent_runtime::retry` is a pure bounded decision policy. It does not sleep, retry adapters, execute tools, enqueue offline work or change policy based on provider/user response text.

## Matrix

Only RateLimited, Timeout, Outage and Quota are retryable for Completion/Stream operations. Authentication, InvalidRequest, Cancelled and Permanent failures are terminal. Tool and Destructive operations are always terminal even when the provider error is transient.

## Bounds and identity

`RetryPolicy` enforces positive max attempts/base delay, capped delay and bounded jitter configuration. Backoff is deterministic exponential with a maximum. `RetryContext` requires bounded request identity, positive token budget and cancellation token. Each retry gets a stable `request_id:attempt_N` identity; malformed IDs fail closed.

Attempt, token budget and cancellation are checked before producing Retry. Terminal reasons distinguish non-retryable, cancellation, attempt budget, token budget and side-effect veto. No raw provider body or user text is stored in decisions.

## Tests

`crates/agent-runtime/tests/retry_contract.rs` covers:

- transient/non-retryable error matrix;
- deterministic capped backoff and attempt identity;
- attempt/token/cancellation budgets;
- tool/destructive side-effect veto;
- bounded jitter and policy independence from text;
- malformed policy/context fail-closed behavior.

## ONP mapping

- T-380 — Adicionar retry policy [concluida]