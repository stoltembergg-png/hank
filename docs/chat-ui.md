# Single-agent chat UI contract

`frontend/src/chat/ChatPage.tsx` is the single-agent chat shell for one typed Session scope. It receives a `ChatTransport` dependency instead of importing storage, provider adapters, or Tauri internals directly.

## Scope and transport

The page sends a bounded typed command containing schema version, caller/project/agent/session identity, command/stream IDs, generation, cancellation ID and user text. `ChatTransport.subscribe` delivers unknown event values from the bridge; `ChatStreamConsumer` validates identity, generation, sequence and terminality before UI state changes.

## UI state

- `idle`: composer is ready;
- `sending`: command accepted by transport;
- `streaming`: ordered deltas append to one assistant message;
- `cancelling`: cancel request is in flight;
- `completed`/`cancelled`: exactly one terminal state is rendered;
- `error`: only a generic redacted message is shown, with retry for the last user text.

Foreign/stale/duplicate/out-of-order events are ignored without changing the active assistant message. The rendered message list is bounded to 200 entries. No provider error payload, credential, endpoint, prompt log or arbitrary HTML is rendered.

## Accessibility

The page exposes a named `main`, labeled textarea, actionable Send/Cancel/Retry buttons, a live message list and a status region. Controls are keyboard-actionable and disabled while the turn is busy.

## Tests

`frontend/tests/chat-page.test.tsx` covers send scope, ordered stream rendering, foreign/stale isolation, cancellation, redacted error/retry, blank composer rejection and accessible controls.

## ONP mapping

- T-384 — Adicionar single-agent chat UI [concluida]