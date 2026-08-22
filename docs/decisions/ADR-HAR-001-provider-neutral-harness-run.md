# ADR-HAR-001 — Provider-neutral Harness Run identity

- **Status:** proposed; activates only after PR-270 baseline PASS.
- **Decision:** a Harness Run is an agent/project-scoped durable aggregate independent of provider/model. Provider/model selection is an attempt attribute, not the Run identity.
- **Fields:** schema version, run/project/agent/session/task IDs, trace/correlation IDs, state/generation, parent run, policy/schema revisions, budget references, checkpoint/evidence/memory references, and terminal outcome.
- **Consequences:** model hot swap, fallback, replay and shadow can share a run lineage without pretending provider output is trusted state. Provider adapters remain outside `agent-core`.
- **Rejected:** provider-specific run types; storing provider SDK objects or raw prompt/completion in the run aggregate.
- **Proof required:** provider-neutral contract fixture, project isolation, stale generation and model-swap/replay negative tests.
- **Rollback:** disable new Run creation by policy; retain immutable records and use a versioned reader/compatibility path.
