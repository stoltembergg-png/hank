# Memory context selector

`MemorySelector` is a read-only, policy-first boundary between memory retrieval
and generic context assembly. It receives already-loaded candidates; it does not
open SQLite, select providers, write memory, activate records or execute text.

Selection order:

1. validate trace, budget and candidate bounds;
2. filter project scope and optional agent scope;
3. keep only `Approved` candidates;
4. reject policy/capability-denied, malformed, hostile or secret-like content;
5. rank deterministically by bounded importance, confidence and recency;
6. deduplicate by the canonical duplicate key;
7. consume only complete candidates within the token budget.

Selected memory is always emitted as `ContextEntry` with `untrusted: true` and
`tool_executable: false`. Omission reasons are metadata only and contain no raw
memory content. Missing trace, cancellation and invalid input fail closed; an
empty candidate list succeeds with an empty selection.
