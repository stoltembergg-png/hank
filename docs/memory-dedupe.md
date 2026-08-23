# Memory deduplication

The pre-persistence dedupe index uses deterministic normalization and a scoped
canonical key. Scope includes project, optional agent and memory kind.

- normalized equivalent content → `Duplicate` with existing ID;
- same scoped key with different content → `Conflict` for review;
- no scoped match → `New`;
- another project never matches;
- duplicate identity and oversized input fail closed;
- rollback removes only the specified index entry.

No semantic/vector similarity or silent merge is performed. Existing content and
provenance are never overwritten by a dedupe decision.
