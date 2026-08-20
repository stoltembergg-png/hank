# Session storage contract

`agent_runtime::session_repo::SqliteSessionRepository` persists Session metadata behind the SQLite/service boundary. Message storage remains outside PR-080.

## Migration

Migration `0004_session_storage.sql` extends the existing sessions table without reusing migration version `0002` (already occupied by project folders). It adds schema version, correlation, participants, metadata, budget/trace references, failure reason and a project/created index. `run_migrations` remains idempotent.

## Repository operations

The repository supports:

- project/agent-FK-backed create;
- project-scoped get/list;
- bounded list pagination (requested limits clamp to 100; zero is invalid);
- optimistic versioned update using `updated_at`;
- lifecycle close through the Session entity;
- typed NotFound, ScopeMismatch, Conflict, Invalid, Serialization and Database errors.

Serialization stores only bounded Session metadata, participant records, references and lifecycle fields. No prompts, Message rows, credentials, provider payloads, UI data or cloud sync are written.

Stale updates affect zero rows and return `Conflict`; the persisted row remains unchanged. Foreign keys preserve project/agent ownership and cascading behavior defined by the existing schema.

## Tests

`crates/agent-runtime/tests/session_storage_contract.rs` covers:

- clean migration columns and idempotent migration;
- create/get/update/close/recovery;
- stale concurrent update rollback/no overwrite;
- cross-project scope and bounded listing;
- duplicate create and missing close typed errors.

## ONP mapping

- T-373 — Adicionar Session storage [concluida]