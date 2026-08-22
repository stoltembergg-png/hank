# Post-270 Harness Gap Analysis

## Method

This analysis compares the requested Harness scope against the current repository/SDD, not the intended titles of PR-001..PR-270. A queue title, stub, mock, or planned card is not treated as implementation evidence.

## Evidence-backed capability map

| Capability | Current evidence | Assessment | Post-270 action |
|---|---|---|---|
| Provider neutrality | `provider-core` capability/response/fallback contracts; `agent-runtime` invocation service; `docs/model-capabilities.md`, `docs/provider-application-service.md`. | Partial but usable basis. | Extend descriptors/router/hot-swap; do not replace `ModelProvider`. |
| Context selection | `crates/agent-runtime/src/context.rs`, `basic.rs`, `docs/context-builder.md`. | Deterministic bounded selector exists. | Add retrieval, ranking, authority conflict/freshness, task-aware ContextBudget, manifest/debugger. |
| Budget | `crates/agent-core/src/budget.rs`; execution usage accounting. | Scope limits/reservations exist. | Add allocation policy and cross-component budget decisions, not a parallel ledger. |
| Tool safety | `tool-core` trait/schema/registry/permission/process/git/http; `ToolContext`. | Strong component basis. | Add invocation/effect envelope and normalizers/artifacts; no generic shell-first layer. |
| Execution state | `agent-runtime/src/execution/mod.rs`, `docs/execution-state-machine.md`. | Turn state/snapshot exists. | Extend to durable Harness Run/checkpoint/recovery; preserve existing execution API. |
| Message persistence | `message_repo`, `docs/message-storage.md`. | Session/partial stream persistence exists. | Reuse migration/repository conventions for checkpoint/evidence records. |
| Memory | `agent-core/src/memory.rs`; `agent-runtime/src/memory.rs` is an initial stub. | Domain seed only. | Implement typed lifecycle/store/retrieval/provenance, decision and failure memory. |
| Skills | `agent-core/src/skill.rs`; runtime stub. | Lifecycle seed only. | Add routing, run pinning and controlled candidate activation; do not duplicate skill storage. |
| Multi-agent | SDD/invariants define delegation/lease intentions; current queue has PR-155..172. | Must be audited after PR-270, not presumed complete. | Add blackboard/handoff/lease extensions only when baseline confirms gaps. |
| Evidence | trace/correlation, CI contracts and redaction policy exist; no claim resolver. | Absent as a first-class engine. | Add formal claim/evidence/fact state and resolvers. |
| Evaluation/replay/learning | controlled-evolution plans exist; no complete provider-neutral harness evaluator/replay evidence found. | Absent/partial. | V1 evaluation/replay first; V2 candidate/shadow/learning afterward. |
| Observability | usage ledger, traces and redaction requirements exist. | Partial. | Add read models and friction candidates, never raw hidden reasoning. |

## Non-duplication rules

1. Reuse `ProjectId`, `TraceId`, existing budget identities, provider capability states, tool permission decisions, execution snapshots and migration conventions.
2. PR-271 must turn this document into a measured capability matrix at the PR-270 merge SHA. A capability shown sufficient is skipped; only a hardening/adapter card remains when an explicit invariant demands it.
3. No post-270 card creates a second provider registry, permission engine, generic memory bucket, tool executor, or raw prompt store.
4. Existing PR-001..PR-270 delivery claims are revalidated only through their formal baseline/evidence; the extension does not rewrite their cards.

## Functional requirements

- A model is a replaceable runtime attempt; the Agent/Harness Run owns identity, state, policies, budgets, context, tools, evidence, checkpoint and memory references.
- Context selection must be authority-aware, freshness-aware, bounded, explainable and conflict-recording.
- Memory, claim/evidence, checkpoint, replay, lease and handoff records must be project-isolated, provenance-bearing and versioned.
- Tool results are structured summaries plus raw evidence artifacts, never unbounded model context.
- Every external/world claim has an explicit evidence state rather than model-text trust.
- V1 provides an end-to-end reliable run; V2 may compare and improve only through policy/evaluation/rollback.

## Non-functional requirements

- Core/provider neutrality, fail-closed capability and policy handling, bounded inputs/outputs, cancellation/idempotency, deterministic ordering and schema compatibility.
- Performance budgets: context compile/retrieval/checkpoint/router decisions measured and gated by benchmark fixtures before a regression is accepted.
- Cross-platform evidence required whenever OS/process/worktree/desktop behavior is claimed.
- Observability stores metadata/digests and redacted classifications; it never stores secrets, credentials, raw prompts or private reasoning.
