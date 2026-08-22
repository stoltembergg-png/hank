# ADR-HAR-002 — Context authority, conflict and budget

- **Status:** proposed; activates only after PR-270 baseline PASS.
- **Decision:** Context Compiler selects bounded, provenance-bearing sources in fixed authority order: Security → Architecture/ADR → Project → Workflow → Task → Relevant Code → Decision/Failure/Project Memory → Skills → Conversation.
- **Conflict rule:** lower-authority conflicting content never replaces higher-authority content. The compiler emits a bounded conflict/omission record with source IDs, authority values and redacted digest.
- **Budget rule:** ContextBudget reserves output capacity and assigns bounded buckets by task class. It does not fill the entire model window or infer missing authority.
- **Consequences:** context selection is explainable through a manifest and Context Debugger projection; provider/model token accounting remains an adapter concern.
- **Rejected:** concatenating all sources, prompt-text authority, implicit memory priority, or silently dropping conflict metadata.
- **Proof required:** authority conflict, injection, freshness, duplicate, budget-overflow and task-class allocation tests.
- **Rollback:** use the prior compiler policy revision; retain manifests/evidence without reusing stale context.
