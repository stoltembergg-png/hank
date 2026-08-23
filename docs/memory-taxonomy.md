# Memory taxonomy

`MemoryKind` is a storage-independent, versioned classification for future
candidate extraction. Version 1 recognizes exactly:

- `fact`
- `preference`
- `decision`
- `lesson`
- `project_context`
- `technical_context`
- `failure`
- `successful_pattern`

Each kind has explicit retention and minimum-importance hints. Hints do not
approve, activate or retrieve a memory.

Unknown kinds fail closed. Content that claims a `system`/`developer` role,
tries to override previous instructions, or resembles a secret is rejected as
untrusted data. Provenance remains a separate field and is never promoted to
instruction authority by classification.

The taxonomy is independent of SQLite, retrieval, embeddings and UI. Version 1
wire values are stable; future migrations must preserve provenance and provide
an explicit rollback path.
