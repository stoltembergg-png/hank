# Tauri chat streaming event bridge

`apps/desktop/src-tauri/src/streaming.rs` transports normalized `agent_protocol::chat_stream::ChatStreamEvent` values to one authorized Webview window. The bridge is intentionally not a provider adapter, execution engine, storage reader or generic command handler.

## Contract

- Event name: `hank://chat/stream`.
- Envelope schema: version `1` with stream/command/caller/project/agent/session identity, generation and sequence.
- Payloads: `start`, bounded `delta`, bounded usage, typed finish, typed error code and typed cancellation reason.
- Errors carry categories only; provider payloads, prompt logs and credential material are not transported by the bridge.
- `ChatStreamValidator` rejects foreign identity, stale/future generation, duplicate/out-of-order sequence, missing/duplicate start and post-terminal events.
- `StreamBridge::publish` validates and enqueues atomically. A rejected event cannot advance the validator.
- Queue capacity is bounded. Non-terminal events return explicit backpressure when full. A terminal event may coalesce one queued delta to preserve terminal delivery; `start` is never discarded.
- `flush` emits in order through an injected `StreamEventSink`. Sink failure leaves the front event queued for retry.
- `TauriWindowSink` is the only Tauri-specific implementation and emits to the selected `WebviewWindow`; no arbitrary page-origin subscription or generic invoke command is added.

## Frontend contract

`frontend/src/contracts/chat-stream.ts` provides a pure `ChatStreamConsumer` for the future chat UI. It validates the same schema/identity/order/terminal invariants and returns typed rejection reasons without logging raw event content.

## Tests

- `crates/agent-protocol/tests/chat_stream_contract.rs`: identity isolation, stale/duplicate/out-of-order generation, malformed/oversized payloads, bounded queue and terminal preservation.
- `apps/desktop/src-tauri/src/streaming.rs`: named channel, sink failure retention, atomic validation/backpressure and terminal delivery.
- `frontend/tests/chat-stream-contract.test.ts`: ordered consumer, foreign/stale/duplicate/out-of-order rejection and malformed/oversized payload handling.

## Host limitation

The local Linux host does not provide `javascriptcoregtk-4.1`; Tauri compilation is therefore `NO_PROOF` locally. The remote Tauri required check remains authoritative and must pass on the exact PR head.

## ONP mapping

- T-383 — Adicionar Tauri streaming event bridge [concluida]