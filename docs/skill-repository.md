# Skill Repository (PR-138)

`agent_runtime::SqliteSkillRepository` persists the declarative Skill model
without turning stored content into executable runtime state.

## Storage model

Migration `0007_skill_storage.sql` creates two tables, and migration
`0010_skill_version_provenance.sql` adds explicit version-graph metadata:

- `skill_versions` is append-only. It stores the project/global namespace,
  manifest version, content hash, serialized manifest/entity, serialized
  parser result, parent version and compatibility classification.
- `skill_heads` stores the mutable pointer for each Skill identity: current
  version, lifecycle status, pin/rollback metadata, and an optimistic revision.

The composite namespace is explicit:

- `project:<project-id>` requires a project identity on every query;
- `global` can only be queried with the global scope and no project identity.

There is no implicit project fallback or global import. A future loader must
create an explicit reference before using a global Skill in a project.

## Repository invariants

- Creating the same identity/version is rejected.
- Updates append a new immutable version and advance the head revision.
- An update with an already persisted content hash is idempotently deduplicated
  and does not move the head or create a second artifact.
- Each record exposes a deterministic `skill-id@semver` version ID and an
  immutable parent link. A major-version change is marked incompatible and
  cannot be activated through the ordinary update path.
- `promote` is the explicit operation that activates the current draft/testing
  version and pins the exact version; rollback only changes the mutable head.
- Updates, archive, and rollback use optimistic revision checks.
- History remains available through `list_versions` and `get_version`.
- Quarantined content cannot become active.
- Activation requires an already-approved manifest policy.
- Parsed instructions and artifacts remain data; the repository does not read
  files, resolve links, invoke tools, or execute scripts.
- Known credential-bearing markers are rejected before persistence, and SQL
  errors are converted to domain errors without returning payload contents.

The repository stores metadata, hashes, and bounded parsed content. It does
not emit instruction or artifact bodies to logs or traces.

## Scope boundary

This increment covers durable CRUD, version history, lifecycle head changes,
project/global isolation, and artifact references as persisted parser data.
Loading, explicit global import resolution, execution, UI integration, and
evolution evaluation belong to subsequent increments.
