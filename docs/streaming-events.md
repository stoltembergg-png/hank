# Streaming Events contract

`provider_core::stream` defines the provider-neutral streaming event contract. It does not implement provider transport, Tauri events, chat UI, reconnect, or tool execution.

## Event envelope

Every `StreamEvent` carries:

- schema version;
- stream, request, and correlation IDs;
- non-zero generation;
- contiguous sequence number;
- one typed payload.

Payloads are:

- `start` with opaque provider/model IDs;
- `delta` with bounded output part;
- `tool_request` with bounded tool ID, capability fingerprint, and optional context metadata;
- `usage`;
- `finish` with normalized finish reason;
- `error` with normalized redacted provider error;
- `cancel` with bounded reason;
- explicit `unknown`, rejected fail-closed by validation.

Tool requests are metadata only. This contract has no execution method or provider/tool invocation path.

## Ordering and generation

`StreamValidator` requires:

1. exactly one start event at sequence 0;
2. one contiguous sequence per accepted event;
3. the configured generation on every event;
4. no duplicate, gap, stale, or future-generation event;
5. exactly one terminal event (`finish`, `error`, or `cancel`);
6. no data or second terminal event after terminality.

Violations return typed errors. No event is silently dropped.

## Backpressure and bounds

`StreamBuffer` has a caller-selected bounded capacity up to 1024 events. A full buffer returns `Backpressure`, including for terminal events, rather than silently dropping data.

Payloads and metadata are bounded. Secret-like values (`api_key`, authorization headers, passwords, secrets, tokens, bearer values) are rejected before acceptance.

## Tests

`crates/provider-core/tests/stream_contract.rs` covers:

- ordered start/delta/usage/finish lifecycle;
- duplicate, out-of-order, stale, and future generation rejection;
- start requirement and exactly-one terminal behavior;
- bounded buffer and terminal preservation;
- bounded delta and tool metadata-only behavior;
- schema, identity, unknown/secret metadata fail-closed validation.

## ONP mapping

- T-353 — Definir eventos de streaming provider-neutral [concluida]