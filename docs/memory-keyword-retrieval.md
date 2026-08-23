# Keyword memory retrieval

`KeywordRetriever` is a bounded, storage-independent retrieval primitive.
Queries require project identity, trace ID, terms, result count and byte budget.

Only `Approved` records in the requested project/agent scope are considered.
Terms are tokenized safely and normalized. Results rank by match count,
importance and ID using deterministic ordering. Duplicate IDs are rejected at
insert time, and output is truncated by whole-record byte budget.

Archived records, cross-project records, oversized terms and missing identity
fail closed. Content remains data and no vector backend, instruction execution
or context assembly is performed.
