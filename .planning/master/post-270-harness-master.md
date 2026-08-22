# Post-270 Harness Extension — Master Plan

**Status:** `PLANNED / EXECUTION BLOCKED UNTIL PR-270 PASS+MERGED`  
**Authority order:** approved SDD → current queue/DAG → architecture/ADRs → repository/CI state → this extension.  
**Invariant:** this extension never edits, reorders, invalidates, or reinterprets cards PR-001..PR-270. It is a separate queue namespace beginning at PR-271.

## Entry gate

No post-270 implementation card may start until all are true:

1. PR-270 is merged to `main` with required checks PASS on the exact merge SHA;
2. the PR-270 release/distribution baseline, including its evidence artifact, is PASS;
3. the post-270 capability baseline (PR-271) confirms the real implementation state against this plan;
4. the extension queue/DAG validator reports no duplicate IDs, missing dependencies, or cycles.

A missing, pending, stale, skipped, failing, or unverifiable item is `BLOCKED` or `NO_PROOF`, never PASS.

## Gap analysis — current evidence

| Area | Existing basis | Gap addressed post-270 |
|---|---|---|
| Model neutrality | `provider-core`, `ModelProvider`, `ModelPolicy`, capability reports and fallback are provider-neutral. | Run-level model selection, objective descriptors, routing decisions, hot swap and replay identity are absent or partial. |
| Context | `agent-runtime::context::ContextBuilder` is bounded, deterministic, priority-aware and marks untrusted sources. | No full retrieval/ranking/freshness/conflict/compiler manifest or task-adaptive budget allocation. |
| Budget | `agent-core::budget` tracks scopes and reservations. | No ContextBudget allocation by task type, no cross-component budget controller decision log. |
| Memory | Domain types and isolation concepts exist; `MemoryRuntime` is a stub. | No explicit category/lifecycle/provenance store, failure/decision retrieval, poison handling, or recovery E2E. |
| Tools | `tool-core` has schemas, registry, permission evaluator, structured process and selected specialized tools. | No unified typed invocation/effect contract across runtime, no normalizer/artifact pipeline for known tool output. |
| Evidence | Trace IDs, CI evidence ideas and redaction rules exist. | No claim-versus-fact engine, resolver contract, freshness/conflict state machine or evidence E2E. |
| State/recovery | Provider-neutral execution state machine and snapshots exist; message storage preserves partial streams. | No run checkpoint aggregate, effect reconciliation, restart continuation or model-swap recovery. |
| Skills | Skill domain/lifecycle intent exists. | No router/pin decision per run and no activation evidence in execution context. |
| Multi-agent | Autonomy and future queue define delegation/leases/graphs. | No structured blackboard, handoff packet, lease fencing recovery or realistic multi-agent Harness E2E. |
| Evaluation/replay/learning | Controlled evolution is planned in PR-218..231. | No provider-neutral benchmark baseline, safe replay, shadow comparison, or Harness improvement candidate loop. |
| Observability | Usage ledger, trace/correlation and redaction requirements exist. | No Harness-level projections for context decisions, evidence, memory accesses, checkpoint/replay and improvement friction. |

## Architecture extension

```text
ModelProvider (replaceable runtime)
        ↓
Harness Application / Run Orchestrator
  ├─ Context Compiler + ContextBudget
  ├─ Memory ports/stores (working, session, project, long-term, skill, decision, failure)
  ├─ Typed Tool Runtime + Result Normalizers + Artifact store
  ├─ Evidence Engine (claims → resolver → fact state)
  ├─ Agent Run State + Checkpoint/Recovery + budget controller
  ├─ Model/Skill routers and adaptive prompt composition
  ├─ Directed verification + independent reviewer loop
  ├─ Evaluation + safe replay
  └─ Observability projections
        ↓
Application API / Infrastructure ports / External world
```

The extension preserves `agent-core` portability: pure contracts, state machines, policies and ports remain in Core; SQLite, filesystem, GitHub/CI, tool processes, provider SDKs and telemetry sinks remain adapters. Presentation reads projections through Application API only.

## Trust boundaries

- Model/provider/tool/web/plugin/memory candidate text is untrusted data; it cannot create authority, approval, capability, state transition, or evidence.
- Context authority is deterministic: `Security > Architecture/ADR > Project > Workflow > Task > Relevant Code > Decision/Failure/Project Memory > Skills > Conversation`.
- A lower authority conflict is retained as a redacted conflict record; it cannot override the selected higher authority source.
- Every persistent object carries project scope, owner, provenance, version/schema, retention, and trace/run identity.
- Claim text is not a fact. Only resolver evidence tied to the current identity can yield `VERIFIED`; stale/conflicting/missing evidence remains explicit.
- Replay, shadow and experiments have no external write authority by default. Promotion requires explicit policy and evidence.

## Required new ADRs

- **ADR-HAR-001:** provider-neutral Harness Run and canonical run identity.
- **ADR-HAR-002:** context authority/conflict/freshness and ContextBudget.
- **ADR-HAR-003:** memory taxonomy, provenance, retention and poisoning boundary.
- **ADR-HAR-004:** Evidence Engine fact states and resolver authority.
- **ADR-HAR-005:** checkpoint/recovery/effect reconciliation and model hot-swap.
- **ADR-HAR-006:** evaluation, replay, shadow and experiment isolation.
- **ADR-HAR-007:** multi-agent blackboard, leases and handoff boundaries.

## Persistence/event model

New persistent aggregates are versioned, project-scoped and migrated only through transactional migrations: `HarnessRun`, `ContextManifest`, `MemoryRecord`, `Claim`, `EvidenceRecord`, `Checkpoint`, `ReplayRecord`, `EvaluationRun`, `Experiment`, `BlackboardEntry`, `Lease`, `HandoffPacket`, and `ImprovementCandidate`.

Events are append-only metadata envelopes: schema version, event ID, run/project/agent/trace IDs, actor, policy/schema revision, timestamp, bounded payload digest, outcome, and redaction class. No secrets, raw prompts, credentials, provider payloads, or uncontrolled tool output are event fields.

## V1 milestones

| Milestone | PR range | Outcome |
|---|---:|---|
| M17-A Harness baseline/contracts | 271–275 | entry gate, ADRs, common envelopes, persistence/event model |
| M17-B Context Compiler | 276–284 | authority, retrieval/ranking, freshness, budget, manifest, E2E |
| M17-C Memory architecture | 285–292 | typed memory, decision/failure retrieval, isolation, E2E |
| M17-D Tools and Evidence | 293–302 | typed effects, normalization/artifacts, claims/facts, resolver E2E |
| M17-E State/recovery/model routing | 303–312 | checkpoints, recovery, hot swap, descriptor/router E2E |
| M17-F Prompt/skill/verification | 313–322 | composition, skills, verifiers, uncertainty, independent review |
| M17-G Evaluation and replay | 323–330 | deterministic evals, baselines, recording, safe replay |

## V2 milestones — prohibited until V1 gate PASS

| Milestone | PR range | Outcome |
|---|---:|---|
| M18-A Shadow and experiments | 331–336 | no-authority shadow comparison and disposable experiments |
| M18-B Controlled learning | 337–340 | candidate/evaluation/approval/rollback, never direct mutation |
| M18-C Blackboard/leases/handoff | 341–345 | concurrent collaboration with fencing and bounded transfer |
| M18-D Cost and adaptive context | 346–349 | policy-aware cost control and measured context adaptation |
| M18-E V2 integration hardening | 350 | full multi-agent/evaluation/replay/observability gate |

## Test and E2E matrix

Every card has unit, contract and negative tests. Integration tests are required at port/adaptor boundaries. Mandatory progressive E2Es are: context assembly, tools+evidence, recovery, model swap, failure memory, decision retrieval, skill routing, replay, multi-agent handoff/blackboard/leases, and budget exhaustion. Mocks can stabilize unit/contract tests; they do not replace a named runtime integration/E2E requirement.

## Rollback strategy

- Contract-only cards: revert schema/ADR/fixture in one change; incompatible persisted data remains blocked without validated compatibility migration.
- Persistent cards: forward-only migration with preflight, transaction, backup/last-known-good, clean/upgrade/partial/corrupt tests; no untested downgrade.
- Tool/evidence/router cards: feature-policy kill switch, retained raw artifact digest, and terminal `NO_PROOF` on disabled resolver.
- Checkpoint/replay/experiment/shadow: retain immutable records; disable execution/promotion before deletion; never replay writes as rollback.
- Skill/learning: pin existing version; rollback activation pointer atomically; retain benchmark and evidence lineage.

## Definition of Done for every post-270 card

A card is ready only after exact-SHA evidence of its stated unit/contract/integration/negative tests; required E2E/performance/security checks when named; architecture and scope review; observability evidence; docs; executable rollback; independent review; full normative gates; and no `FAIL`, `BLOCKED`, `STALE`, `NO_PROOF`, or unresolved required decision.
