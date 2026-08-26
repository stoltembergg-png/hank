# Desktop notifications

PR-203 introduces a pure, bounded notification policy in `agent-runtime`.

The policy accepts only project-scoped automation signals, emits explicit
severity, redacts untrusted title/body text, suppresses duplicate event IDs,
applies a bounded per-window delivery count, and constructs only validated
`hank://runs/<project>/<run>` deep links.

This slice does not call OS notification APIs, expose raw prompts, implement
remote push, or perform automatic approval. Those operations require the
separate desktop permission boundary tracked by T-1292.
