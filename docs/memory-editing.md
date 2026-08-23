# Explicit memory editing boundary

The memory mutation service is a backend-only, project/version-scoped boundary.
Every operation carries actor ID, project ID, trace ID, operation ID,
`memory.write` capability and an explicit policy decision.

Supported operations in this slice:

- update bounded content, summary and importance;
- approve;
- reject;
- archive;
- restore.

The repository update uses a parametrized project/id/version predicate. A stale
version, foreign project, denied policy/capability, invalid context or invalid
content fails without mutation. Lifecycle transitions increment the version.

Tauri commands, human confirmation, persistent audit records and UI mutation
controls remain a later boundary in PR-133; the service is not exposed as a
free-form frontend write path.
