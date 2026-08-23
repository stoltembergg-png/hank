# Memory vector retrieval

`VectorIndex` is an optional local backend over typed embedding records. It
requires project/agent scope, model/version identity and active status.

Upsert is deterministic by record ID. Queries use bounded cosine similarity,
`k`, byte budget and deterministic ID tie-breaking. Dimension/model mismatches
fail closed. Archived records are removed from the active query set.

Rebuild validates the complete replacement against the current schema before
swapping the index. A failed rebuild leaves the previous index unchanged.

The backend does not select providers, embed raw content, share projects,
replace the memory repository or assemble final context.
