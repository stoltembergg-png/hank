# Agent CRUD services contract

`AgentService` is an application service that composes Agent repository, Project repository
and event bus to expose CRUD use cases for Agents.

It enforces:
- Project existence on create/get/list/update/archive;
- Domain validation on every mutation;
- Optimistic concurrency via `updated_at` version;
- Explicit confirmation for archive;
- Event publication after successful persistence;
- No direct SQLite/adapter access in callers.

## Operations

### Create
Input: `CreateAgentInput { project_id, name, description?, policy, correlation_id? }`
Output: `AgentOutput { agent, event_id?, correlation_id? }`
Validates project exists, creates Agent with `Active` status and default Personality, emits `AgentCreated`.

### Get
Input: `project_id, agent_id`
Output: `Option<Agent>`
Project-scoped lookup; returns `None` if not found in that project.

### List
Input: `project_id, limit?, offset?`
Output: `Vec<Agent>`
Paginated, bounded to max 100 per page; validates project exists first.

### Update
Input: `UpdateAgentInput { project_id, agent_id, name?, description?, status?, personality?, policy?, expected_version, correlation_id? }`
Output: `AgentOutput`
Optimistic version check on `expected_version`; applies partial updates; validates each field; emits `AgentUpdated`.

### Archive
Input: `ArchiveAgentInput { project_id, agent_id, expected_version, confirmation, correlation_id? }`
Output: `AgentOutput`
Requires `confirmation == "confirm archive"`; sets status to `Inactive` (terminal); emits `AgentArchived`. Idempotent for already inactive.

## Events

All mutations emit `ApplicationEvent` with:
- `schema_version: 1`
- `event_id: EventId`
- `event_type: AgentCreated | AgentUpdated | AgentArchived`
- `project_id`
- `aggregate_id: agent.id.to_string()`
- `agent_id: Some(agent.id)`
- `sequence: 1 | 2 | 3`
- `payload: JSON string`

## Error modes

- `NotFound` — project or agent not found
- `Validation` — field bounds, forbidden content, name limits
- `ConcurrencyConflict` — stale `expected_version`
- `InvalidStateTransition` — invalid status change (e.g., archive inactive)
- `Duplicate` — unique constraint violation on save
- `InvariantViolation` — storage/serialization failures

## Tests

Integration tests cover:
- create persists and emits event
- create without project fails
- get returns exact agent
- list paginates correctly
- update with correct version succeeds
- update with stale version fails with ConcurrencyConflict
- archive without confirmation fails
- archive with confirmation succeeds
- archive inactive agent fails with InvalidStateTransition

## ONP mapping

- T-338 — Map legacy Project list UI
- T-339 — Map legacy Project create UI
- T-340 — Map legacy Project detail UI