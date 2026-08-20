# Session entity contract

`agent_core::session::Session` is a storage/provider/UI-independent domain entity bound to exactly one Project and owner Agent.

## Identity and metadata

Each session carries a schema version, typed session/project/agent IDs, bounded correlation ID, lifecycle status, participant metadata, optional budget reference, optional trace reference, bounded metadata map, timestamps and optional terminal failure reason. It never stores prompts, credential values, tokens, endpoints or provider types.

Participants must belong to the session project, have bounded labels, and are deduplicated with a maximum count. Metadata keys and serialized values are bounded, immutable by key, and reject secret-like markers. Budget references are opaque `budget_` identifiers and trace references use typed protocol IDs.

## Lifecycle

Valid transitions are:

```text
Created -> Active -> Closing -> Closed
Created -> Failed
Active  -> Failed
Closing -> Failed
```

`Closed` and `Failed` are terminal. Closing is idempotent; all other invalid transitions fail without mutating state. Participants, metadata and references can be added only before Closing.

## Tests

`crates/agent-core/tests/session_contract.rs` covers:

- project/agent/correlation binding and bounds;
- deterministic lifecycle and idempotent close;
- invalid transition immutability;
- project-scoped bounded participant list;
- opaque budget/trace references and no prompt storage;
- schema/lifecycle serde roundtrip;
- bounded metadata and lifecycle mutation guard.

## ONP mapping

- T-371 — Adicionar Session entity [concluida]