# QA agent profile

## Boundary

`agent-core::qa_profile` is a pure, provider-neutral contract for an authorized
QA executor. It validates a `QaTestPlan`, returns a bounded `QaPermit`, and
checks `QaTestResult`/`QaReport` identity and evidence. It does not execute a
process or access Git, filesystem, network, CI, providers, secrets, or raw logs.

## Allowlist

Plans use typed `QaCommand` values for repository-approved test families:
Cargo test/check/clippy/fmt, Node tests, the feature runner, and ONP verify.
`Shell` and `Arbitrary` values are representable only as rejected input. They
cannot become an executable capability or instruction.

Each permit is scoped to the active project/task/repository/worktree/branch and
policy revision. It carries bounded command count, timeout, attempt and output
limits. The permit has no APIs to disable checks, change expectations, or
authorize release.

## Evidence contract

A result records only a typed command, exact commit/tree SHA, status, attempt,
duration, output digest and optional artifact digest. Raw command output, logs,
prompts and secrets are intentionally absent. `QaReport` is complete only when
every planned command has a matching `Passed` result and an artifact digest.

`Failed` is valid evidence but is never success; it creates a failure handoff.
`Skipped`, `NoRun`, `Cancelled`, `TimedOut`, `Malformed` and `Stale` are
fail-closed and cannot release a gate. Wrong scope, policy, SHA/tree, duplicate
or missing results are rejected.

## Runtime boundary

A future adapter may execute only an accepted `QaPermit` using its own bounded
process, timeout, output-redaction and artifact policies. This PR intentionally
does not add that executor. Remote CI remains the authority for the exact PR
head and platform-specific execution.

## Rollback

Removing this module and its contracts removes only the QA domain boundary; it
does not alter migrations, existing gates, branches, or runtime behavior.
