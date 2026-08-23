# Memory candidates

`MemoryCandidateExtractor` is a data-only producer. It validates a candidate
request and returns `Pending` data with project/session identity, source message,
taxonomy kind, provenance and bounded confidence.

It never writes to the memory repository, never activates a memory and never
turns content into trusted instructions. Missing identity/source, invalid
bounds, instruction-hierarchy claims and secret-like content fail closed.

Approval, importance assignment and persistence are separate operations owned by
later policy/repository increments.
