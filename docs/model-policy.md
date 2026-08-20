# Model policy contract

`ModelPolicy` is provider-neutral metadata for selecting a model through a future
provider registry. It contains abstract provider/model identifiers, bounded token and
context limits, temperature, declared modalities and an optional bounded fallback chain.

The policy never contains URLs, endpoints, API keys, tokens, passwords or SDK objects.
Unknown fields fail closed. Unsupported modalities are explicit through the capability
state and are never treated as supported implicitly. Fallback depth, numeric ranges,
parameter count and identifiers are bounded and deterministic.

This contract does not call providers, discover models, store credentials or execute
fallback. Those responsibilities belong to later Core Track cards.
