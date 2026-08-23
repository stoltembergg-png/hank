# Memory repository

`SqliteMemoryRepository` persists validated Memory entities with a mandatory
project scope.

All reads use both `project_id` and `memory_id`; a different project returns no
row. Active listing excludes archived rows and uses deterministic ordering.

Creation validates the domain entity and uses parameterized SQL. Duplicate IDs
fail without replacing the existing row. Archive uses optimistic version
matching; a stale expected version returns a concurrency conflict without
mutation.

The migration adds a foreign key to projects and a project/status index. This
repository slice does not implement retrieval, embeddings, ranking, extraction,
model auto-write or cross-project sharing.
