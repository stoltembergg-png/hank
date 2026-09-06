# Provider compatibility contract

PR-265 validates provider adapters against the provider-neutral capability,
request, response, stream, fallback, and redaction contracts.

The manifest is a bounded, offline release input. Fixtures are synthetic and
contain no provider credentials, live endpoints, network calls, or availability
claims. `supported`, `bounded`, and `expected-fail` are explicit classifications;
unknown capabilities and contract mismatches fail closed.

The matrix executes the provider-core compatibility contract plus each existing
adapter contract for OpenAI, Anthropic, Gemini, OpenRouter, Ollama, and the
OpenAI-compatible boundary.
