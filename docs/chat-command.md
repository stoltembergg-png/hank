# Typed chat command contract

PR-089 adds a versioned typed ChatCommand boundary in `agent-protocol` plus a thin runtime router. It does not add Tauri generic invoke, direct SQLite/provider access, UI rendering or secret transport.

## Protocol envelope

`ChatCommand` carries schema version, command/caller identity, typed Project/Agent/Session IDs, bounded user text, generation and cancellation ID. It rejects empty/control/oversized/secret-like content and unknown schema state fail-closed.

`ChatCommandRegistry` bounds accepted commands, deduplicates command IDs, rejects stale generations per Session and exposes only status/diagnostic metadata. Typed IDs remain typed in the envelope; registry internal keys use canonical strings without changing public ID ordering traits.

## Runtime boundary

`ChatCommandRouter` validates/deduplicates through the protocol registry before invoking an injected `ChatCommandDispatcher`. Duplicate/stale commands never reach the dispatcher. The router returns a typed command/session/generation handle and has no generic invoke, storage, provider or adapter dependency.

## Tests

`crates/agent-protocol/tests/chat_command_contract.rs` covers valid versioned identity, duplicate/stale generation, malformed/oversized/secret-like input and bounded registry capacity/cancellation metadata.

## ONP mapping

- T-382 — Adicionar typed chat command [concluida]