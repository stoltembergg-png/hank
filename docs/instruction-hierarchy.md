# Instruction hierarchy contract

The default order is deterministic and descending:

```text
system → security → project → agent → workflow → skill → conversation → user
```

Each source appears at most once. Security is immutable/non-overridable; lower layers
cannot replace it. Layer names and the aggregate size budget are bounded. Unknown
fields, duplicate sources, zero precedence and invalid budgets fail closed.

`ordered_layers()` returns a precedence-sorted copy and does not execute or render
instructions. Provider adapters, LLM calls and context assembly remain outside this
contract. Provenance is represented by the typed source enum; raw instruction content
is intentionally not accepted here.
