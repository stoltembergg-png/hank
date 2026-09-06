# Workflow recovery test contract

PR-263 exercises the existing bounded workflow recovery store with an isolated
SQLite fixture. It verifies lease fencing, bounded expired-run recovery,
unknown-effect classification, idempotent active-lease handling, and typed
invalid-input rejection. The suite never starts providers, tools, processes,
or external capabilities.
