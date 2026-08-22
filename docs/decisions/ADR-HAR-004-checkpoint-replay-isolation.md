# ADR-HAR-004 — Checkpoint, replay and experimental isolation

- **Status:** proposed; activates only after PR-270 baseline PASS.
- **Decision:** checkpoints persist run goal/state, completed and remaining work, changed-file digests, tool idempotency/effect state, tests, blockers, decisions, memory/evidence references, model/provider attempt, budgets and trace.
- **Recovery:** restart validates checkpoint schema, project scope, policy revision and effect reconciliation before continuing. Completed effects are never blindly repeated.
- **Replay/shadow/experiment:** default to read-only or isolated worktree/sandbox. External writes require a new explicit approval/fingerprint; replay never inherits authority from the original run.
- **Consequences:** benchmark/replay comparisons are reproducible and do not claim real-world effects from mock-only execution.
- **Rejected:** serializing raw secrets/prompts, resuming from free-form model text, replaying destructive tool calls, or mutable checkpoint histories.
- **Proof required:** torn/corrupt checkpoint, crash-before/after effect, model swap, replay write denial, experiment cleanup and SHA/policy mismatch tests.
- **Rollback:** stop scheduling/resume, preserve immutable checkpoint/evidence, and use versioned compatibility readers.
