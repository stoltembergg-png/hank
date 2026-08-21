# Agent execution state machine

`agent_runtime::execution` provides the bounded provider-neutral execution state machine for one Agent turn. It is intentionally separate from adapters, UI, tool execution, retry policy and memory writes.

## States and transitions

```text
Preparing -> Running -> Streaming -> Completed
       \        \          \-> Failed
        \        \-> Failed
         \-> Cancelled
```

`Completed`, `Failed` and `Cancelled` are terminal and cannot be overwritten. Provider invocation identity can be recorded once. Stream start is legal only from Running; completion is legal from Running or Streaming; cancellation wins a race when applied first. Illegal transitions return typed errors without mutating state.

## Fences and budgets

Each execution carries bounded execution/session/agent/correlation identity and a positive generation. A different generation is stale. Token and cost counters are bounded; an exceeded budget transitions the active execution to redacted `budget_exceeded` failure. `ExecutionConcurrency` uses an atomic bounded permit with RAII release.

`ExecutionSnapshot` preserves state, identity, invocation id, failure code, usage counters and budgets. Restore rejects invalid budget/failure combinations and never reopens a terminal execution.

## Provider boundary

`ExecutionCoordinator::complete` consumes `ProviderApplicationService` and `InvocationRequest`, never an adapter. It fences generation/cancellation, records invocation identity, maps success to Completed, cancellation to Cancelled and all provider failures to a neutral redacted ProviderFailed result. No raw provider error or content is stored in execution state.

## Tests

`crates/agent-runtime/tests/execution_contract.rs` covers:

- success and exactly-one terminal state;
- illegal transitions and duplicate invocation;
- streaming/error/generation fencing;
- cancellation race;
- token/cost budget and redacted debug;
- bounded concurrency and permit release;
- snapshot recovery and terminality.

## ONP mapping

- T-377 — Adicionar Agent execution state machine [concluida]