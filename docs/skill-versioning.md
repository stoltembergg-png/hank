# Skill Versioning (PR-143)

Skill artifacts are immutable. A new version is an append-only record in the
project or global namespace; the mutable `skill_heads` row only points to the
current version and lifecycle state.

## Version graph

Each persisted record exposes:

- `version_id`: deterministic `skill-id@semver` identity;
- `content_hash`: a SHA-256 hash of the canonical validated payload, excluding
  only the version envelope and declared digest;
- `parent_version`: the previous head version for updates;
- `compatibility`: `initial`, `compatible` (same SemVer major), or
  `incompatible` (major change);
- source, policy, budget and trace metadata from the validated manifest.

The parent and compatibility columns are stored independently from the JSON
payload. Reads reject a record if those columns disagree with the serialized
Skill, preventing version spoofing or provenance drift.

## Lifecycle and pinning

`update` creates a draft/testing artifact and advances the head only when the
content hash is new. Identical content is deduplicated without changing the
current head. `promote` is explicit and pins the exact current version;
incompatible versions cannot be promoted by the ordinary update path.

Project and Agent bindings keep their own exact version pins. Creating or
promoting another version never rewrites an existing binding or an immutable
history record.

## Rollback and isolation

`rollback` switches the scoped head to an existing immutable version, records
the replaced version for audit context, increments the optimistic revision and
keeps the complete history. It does not delete artifacts or execute Skill
content. Project and global namespaces remain explicit; a project query cannot
read another project or silently fall back to global content.

Project changes are emitted as project-scoped `ApplicationEvent` records.
Global changes use the explicit `GlobalApplicationEvent` envelope and never
invent a project identity. Event payloads expose only a source kind and a
digest of the source reference; local paths, URLs and instruction content are
not emitted.

Rows created before PR-143 are marked with the `legacy` hash algorithm and
historical versions are conservatively treated as draft until an explicit
promotion. They remain readable for migration compatibility but are not
silently treated as newly verified artifacts.

The repository validates manifests, parser quarantine, path/digest bounds and
credential-bearing markers before any version is persisted. Content remains
untrusted data and never grants capabilities or changes instruction policy.
