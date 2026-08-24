# Skill Loader (PR-139)

`agent_runtime::SkillLoader` is the read-only boundary between persisted
Skills and context/tool consumers. It resolves the current repository head,
then applies identity, scope, lifecycle, capability, policy, budget, and
content checks before returning data.

## Load semantics

- A project load requires `project_id`, `agent_id`, a non-nil `trace_id`, and
  a scoped `skill:read` capability approved by the request policy.
- Only `active` Skills are loaded by default. `testing` requires the explicit
  `allow_testing` policy flag. Draft, blocked, deprecated, and archived Skills
  are denied.
- An explicit version must equal the current repository head. Updates and
  rollback therefore produce a new revision/key and cannot return stale
  current state.
- `requested_paths` is a bounded relative-path allow-list. An empty list
  means all declared data; undeclared files, traversal, absolute paths, and
  malformed persisted artifacts fail closed.
- External links are never fetched. They require an explicit policy flag and
  remain metadata only; internal links are checked against declared files.

## Dependencies and cache

Manifest dependencies resolve in the same explicit namespace. The loader
enforces bounded dependency count/depth and rejects cycles. Optional
dependencies may be omitted only when absent or version-incompatible; policy,
quarantine, content, and structural errors remain fatal.

The cache is bounded and keyed by project, agent, scope, Skill, version,
repository revision, import reference, selected paths, and external-link
policy. Version/rollback changes cannot hit an old current-state key, and
`invalidate` allows update services to evict old entries immediately.

## Trust and non-execution invariant

Instruction sections are returned separately from artifact data. Scripts,
templates, references, and tests are returned as bounded `SkillArtifact` data
with no executable handle. The loader does not read files, use the network,
install dependencies, mutate runtime state, or execute scripts.
