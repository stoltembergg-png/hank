# Message entity contract

`agent_core::session::Message` represents bounded chat content and stream provenance without persistence, prompt assembly, provider execution or UI rendering.

## Identity and provenance

Every message carries schema version, typed Session ID, role, independent provenance, correlation ID, generation and sequence. User/provider/tool content is marked `untrusted`; role alone never changes instruction precedence. Secret-like markers, control characters and oversized content are rejected before storage in the entity.

Content is represented by bounded `MessagePart` values. Tool execution payloads remain external; the entity retains only existing tool-call/result DTOs and does not execute them.

## State and ordering

Message states are `Draft`, `Streaming`, `Complete`, `Failed` and `Cancelled`. Complete/failed/cancelled are terminal; complete is idempotent and late mutations fail. `MessageOrdering` binds a session and generation, rejects cross-session messages, stale/future generations, duplicate sequences and out-of-order events, and stops accepting after a terminal message.

## Tests

`crates/agent-core/tests/message_contract.rs` covers:

- provenance/untrusted marking and parts bounds;
- provider/tool provenance without role precedence escalation;
- state transitions and terminal idempotence;
- invalid transition immutability;
- cross-session, stale-generation, duplicate and out-of-order rejection;
- terminal ordering;
- serde unknown-role rejection and provenance roundtrip;
- correlation/part metadata bounds and redaction.

## ONP mapping

- T-372 — Adicionar Message entity [concluida]