# Skill UI (PR-144)

The project detail screen exposes Skills through the typed desktop API. The
frontend never reads SQLite, resolves a source reference, executes a Skill, or
activates a version on the client.

## Scope and states

- **Do projeto** shows only project-scoped Skills whose `project_id` matches the
  selected project.
- **Globais** shows global source records. A record without an enabled project
  binding is rendered as unavailable until the import is explicit.
- Each card shows lifecycle state, displayed and pinned versions, compatibility,
  bounded version history, binding revision, approval, capability declarations,
  budget, trace ID and a redacted source digest.
- Search is bounded to the loaded page (maximum 50 records). It does not widen
  the project scope or bypass API authorization.

## Mutations and safety

Rollback is available only for an enabled project binding. The UI requires a
confirmation and sends `project_id`, `skill_id`, `actor_id`, `approval_id`,
`trace_id`, `expected_revision`, `capability: skill.rollback` and
`confirmed: true` through the API client. The client does not provide a browser
fallback for rollback.

Descriptions are rendered as plain text, with bounded length; no HTML or Skill
content is interpreted. API responses with a different project or scope are
rejected before rendering. When the desktop bridge does not provide the Skills
service, the UI reports it as unavailable instead of fabricating records.

The bridge commands consumed by this surface are `list_skills` and
`rollback_skill`; their governed desktop implementations remain the source of
authorization, persistence, audit and lifecycle decisions.
