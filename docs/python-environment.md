# Python environment manifest

PR-119 stores Python environment intent per project. This slice does not
install packages or mutate a global interpreter.

## Manifest

A manifest contains a schema version, project/environment identity, Python
version, HTTPS source allowlist, and package requirements pinned by version and
SHA-256. Package order is normalized and duplicate identities are rejected.
Names cannot contain traversal, separators or control characters.

## Lock and atomicity

`PythonEnvironmentManager::prepare` creates the project-local environment
directory and an exclusive `environment.lock`. The manifest is written through a
temporary file and renamed atomically. A held lock fails closed; it never
silently overwrites another preparation.

## Rollback

Updating an existing manifest moves the previous version to
`environment.previous.json`. `rollback` restores that validated previous
manifest. No package code is executed during prepare/load/rollback.

Future installation work must add an explicit source, capability, budget and
approval policy; it must not be inferred from this manifest slice.
