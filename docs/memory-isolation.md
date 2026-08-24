# Memory isolation invariant

Memory is project-scoped at every boundary. A project identity is required for
repository reads/lists, vector index queries and destructive operations,
context selection, desktop commands, mutations and rollback.

## Rules

- A foreign `project_id` fails closed before an effect.
- A coincident memory/vector identifier never grants access across projects.
- Queries return only records from the requested project.
- Index archive/delete operations receive and validate the project identity.
- Context selection omits foreign candidates before creating `ContextEntry`.
- Mutation scope is checked before lifecycle or content changes.
- Errors and logs do not include foreign content.

The repository, selector, mutation service and vector index each have two-project
contract coverage. Desktop bridge coverage remains in the Tauri memory contract,
which receives `project_id` as part of every request and keeps SQLite behind the
managed application state.
