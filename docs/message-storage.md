# Message storage contract

`agent_runtime::message_repo::SqliteMessageRepository` persists ordered Message records under an existing Session. It is isolated from UI, provider calls, context assembly and secret storage.

## Migration and schema

Migration `0005_message_storage.sql` extends the existing messages table with schema version, provenance, status, correlation, generation, sequence and serialized parts, plus a unique `(session_id, generation, sequence)` index. Migration execution is idempotent and preserves the earlier Session migration.

## Repository contract

- `append(project, session, message)` validates explicit session binding and project ownership before insert;
- append enforces generation/sequence ordering and returns typed duplicate, stale or out-of-order errors;
- duplicate message IDs are idempotency conflicts;
- `get_by_id` and `list` join Session scope and never cross project/session boundaries;
- list limits are bounded to 100 records and ordered by generation/sequence;
- `update` changes stream/status metadata only when the expected current status matches;
- terminal updates are idempotent, stale updates do not overwrite stored state;
- partial Draft/Streaming messages remain recoverable after restart.

Only bounded Message metadata/parts/tool DTOs are serialized. No provider payload, credential, prompt log, arbitrary path or frontend storage is introduced.

## Tests

`crates/agent-runtime/tests/message_storage_contract.rs` covers:

- migration columns/idempotence;
- append/get/list and partial stream recovery;
- duplicate/stale/out-of-order rejection with no data loss;
- terminal update/idempotence/stale rollback;
- explicit session scope and bounded pagination.

## ONP mapping

- T-374 — Adicionar Message storage [concluida]