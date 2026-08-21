# Provider streaming runtime contract

`agent_runtime::streaming` consumes normalized provider stream events through `ProviderApplicationService::stream`; it never imports adapters or treats provider/user text as policy.

## Event contract

Events must have one contiguous sequence starting at zero. Duplicate, out-of-order or stale-generation events are rejected before mutating Execution/Message state. Text deltas become untrusted Message text parts; terminal events complete exactly once. A stream without terminal fails the execution/message explicitly as `stream_incomplete`; a second terminal is rejected.

Cancellation transitions both state holders to Cancelled. Payload validation rejects secret-like/oversized content. Attempt identity is preserved in `StreamOutcome`, and all provider failures are reduced to redacted `ProviderFailed`/message failure codes.

The pure `StreamEventConsumer::apply` path is deterministic and pre-validates ordering to guarantee no-op behavior for malformed first input. The async `stream` entry performs the provider application-service call, state transitions and normalized event consumption. Persistence remains behind the existing Message repository boundary for the next integration layer; no UI-thread or adapter access is introduced.

## Tests

`crates/agent-runtime/tests/streaming_contract.rs` covers:

- ordered deltas and exactly-one terminal;
- duplicate/out-of-order/stale generation no-overwrite;
- missing/multiple terminal events;
- cancellation and payload bounds;
- attempt identity and redacted diagnostics.

## ONP mapping

- T-378 — Adicionar provider streaming [concluida]