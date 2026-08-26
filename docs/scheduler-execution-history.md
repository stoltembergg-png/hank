# Scheduler execution history

PR-202 adds a bounded query boundary to the existing `scheduler_runs` persistence.

## Query contract

`SchedulerPersistence::list_history` requires a project scope and accepts optional job,
status and due-time filters. Results are ordered by `due_at_ms ASC, run_id ASC` and the
page size is limited to 100 rows. The DTO intentionally excludes lease owner and any
prompt/provider payload.

## Retention contract

`prune_completed` deletes only `completed` rows for the requested project, with a strict
cutoff and a maximum batch of 100. It returns the number of deleted rows and cannot
mutate another project.

The existing claim, lease, completion and missed-outcome APIs remain unchanged. No worker
loop or notification mechanism is introduced by this slice.
