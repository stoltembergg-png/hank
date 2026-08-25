# Workflow logs event catalog

`WorkflowLogStore` is a bounded in-process sink for structured workflow diagnostics.

Allowed event kinds are `start`, `transition`, `end`, `error`, and `recovery`; each event binds
`project_id`, `run_id`, `node_id`, `event_id`, severity, retention class and a monotonic timestamp.
Only `status`, `error_code`, `recovery_class`, `attempt` and `sequence` fields are retained.
Unknown fields and sensitive values are dropped before retention.

The store rejects duplicate event identities and out-of-order events per project/run, limits
retained events and export bytes, and exposes dropped/redacted counters. Queries require exact
project/run scope and bounded limits. Export never contains prompt content, URLs, paths, page
content, tokens, passwords or secrets.

This card intentionally does not add cloud telemetry, UI viewing, SQLite persistence or a
production audit sink. Sink failure is reported and does not authorize workflow execution.
