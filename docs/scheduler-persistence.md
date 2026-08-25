# Scheduler persistence

PR-195 stores scheduler runs and leases in SQLite. `scheduler_runs` is project-scoped and references
`(project_id, job_id)`; due/status and lease indexes bound recovery queries.

## Lifecycle

- `pending`: durable run waiting for a lease;
- `claimed`: one owner holds a bounded lease;
- `completed`: terminal state with bounded outcome and completion timestamp.

The conditional claim update accepts pending runs or expired claimed leases. A competing owner cannot
complete a run it does not own. Completion is terminal and no worker, polling loop or notification is
created here. A subsequent worker PR may use this repository as its authority after restart.
