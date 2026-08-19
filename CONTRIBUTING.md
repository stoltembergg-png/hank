# Contributing to Hank

## Before implementation

1. Select a card whose predecessors are merged and whose base SHA is current.
2. State scope, non-goals, dependencies, tests, security impact and rollback.
3. Use an isolated branch/worktree and keep one active implementation increment.

## Development loop

Use RED → GREEN → REFACTOR:

- add or update a focused failing test/fixture;
- implement the smallest compatible change;
- run the applicable local gates;
- inspect the exact diff and generated artifacts;
- do not weaken a failing gate to obtain success.

## Pull requests

PRs are small, traceable to a queue card and use the repository title/commit policies.
The PR must document acceptance criteria, tests, CI state, risks and rollback. A PR
is not merge-ready while a required check is pending, failed, skipped unexpectedly,
or tied to a stale SHA.

## Blockers and reporting

Classify blockers objectively. Continue safe independent diagnosis and preparation
for soft blockers. Preserve `FAILED`, `BLOCKED`, `NO_PROOF` and stale-evidence states
until real evidence changes them. Never invent credentials, requirements, decisions,
external results or integration evidence.

## Definition of Done

A change is complete only when its scope is implemented, tests and required CI pass on
the exact head SHA, security/observability/docs impacts are addressed, rollback is
clear, and the queue/traceability state is updated when the change changes it.
