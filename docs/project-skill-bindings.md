# Project skill bindings

Project skill bindings are the explicit, auditable boundary between a
persisted Skill and the loader. A Skill is never selected merely because it
exists in SQLite or on disk.

## Binding rules

- A bind request carries project, actor, Skill, scope, capability, policy,
  trace and an optional approval identifier.
- The required mutation capability is `skill:configure:<project-id>` and it
  must also be present in the project policy.
- Project-scoped Skills must belong to the requested project.
- Global Skills require an explicit `project-import:*` reference. There is no
  implicit global fallback.
- Only an active Skill head can be enabled. Skill content is still loaded by
  `SkillLoader` as untrusted data; scripts are never executed by binding.
- A repeated bind for the same scope, version and import is idempotent.
- Scope changes are rejected; the old binding must be disabled or rolled back
  before a different source can be assigned.

## Disable and rollback

Disable and rollback are versioned, optimistic mutations. Rollback is a safe
unbind: it removes the active project reference without rewriting immutable
Skill history. Skill repository rollback remains a separate operation and can
be followed by a new explicit bind.

Every changed mutation emits `SkillBindingChanged` with action, project/Skill
identity, version, actor, approval, trace and revision. Payloads contain no
Skill instructions or artifact contents.

## Storage and isolation

`project_skill_bindings` has one current assignment per `(project_id,
skill_id)`, cascades on project deletion, stores only bounded metadata and
keeps a previous version for audit context. Loading through
`ProjectSkillService::load_bound` requires an enabled binding and passes its
exact version and global import reference to the bounded loader.
