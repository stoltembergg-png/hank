# Memory embeddings interface

The memory core exposes an embedding request/response contract without a
provider SDK or vector store. Requests carry project identity, trace, model,
model version, dimensions, bounded references, budget and cancellation.

`MockEmbeddingProvider` is deterministic and offline. It returns vectors with
the requested dimension and preserves model/version/trace identity. Invalid
project/model/dimension/batch/reference/budget/cancellation states fail closed.

Only references are transported; raw memory content is not part of this
contract. Future providers must preserve the same scope, cost, cancellation and
privacy boundaries.
