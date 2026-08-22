# Post-270 Harness Adversarial Planning Review

**Method:** architecture/SDD/queue contract audit plus independent role-based review of simplicity, integration, evidence, architecture and lateral alternatives. No finding below is treated as runtime proof.

## Findings retained

| Attack | Risk | Planning correction | Evidence gate |
|---|---|---|---|
| Premature mega-Harness | One framework could absorb provider, context, tools, memory and UI into a coupled runtime. | Contracts and ports precede adapters; V1 is split into context, memory, evidence, recovery, router, skill/verifier and evaluation milestones. | Architecture graph/forbidden imports and contract tests per card. |
| Model-specific leakage | Provider/model identity could leak into Agent/Run persistence. | ADR-HAR-001 makes model selection an attempt attribute; provider-core remains the capability boundary. | Model swap and incompatible capability negative tests. |
| Context escalation | Conversation/memory/skill/tool text could override security or ADRs. | Fixed authority order, conflict records, untrusted provenance and ContextBudget minima. | Injection/conflict/stale E2E and redacted manifests. |
| Memory poisoning | A model-proposed correction could become durable authority. | Typed candidate lifecycle, provenance, quarantine and evidence requirement; decision memory is restricted to approved sources. | Poison/cross-project/fabricated-evidence negative suite. |
| Evidence spoofing | Agent prose, stale CI, or foreign SHA could be called verified. | Claim/fact separation, resolver ports, exact identity checks and explicit stale/conflicting states. | Resolver/reconciliation matrix and tools+evidence E2E. |
| Duplicate external effects | Crash/recovery/replay could repeat a mutation. | Idempotency/effect ledger, reconciliation before resume and no-write replay/shadow defaults. | Crash-before/after, replay-write and unknown-effect negatives. |
| Autoapproval | Reviewer/model/shadow could approve executor output. | Distinct reviewer identity, advisory verifier outputs, approval policy and max rounds. | Self-review/double-submit/expiry tests. |
| Multi-agent loop/collision | Chat-only delegation can deadlock or overwrite work. | Structured blackboard, leases/fencing, minimal handoff, depth/budget/cycle policy. | Lease race/crash/takeover and multi-agent E2E. |
| Budget bypass | More context/fallback/review rounds can silently exceed cost. | Shared budget controller reserves mandatory verification and terminates optional work first. | Budget exhaustion E2E and mandatory-security negative test. |
| Nonreproducible improvement | Learning/shadow/experiments might optimize from anecdotes. | Evaluation suite/baselines precede candidate activation; V2 candidates are proposal-only with rollback. | Benchmark baseline/regression and activation rollback proof. |
| Observability leakage | Debugger/traces might expose prompts, secrets or private reasoning. | Metadata/digest-only projections and redaction tests; no chain-of-thought field. | Secret/golden redaction and scope tests. |

## Rejected alternatives

1. A generic `shell(command)`-centred Harness was rejected: it violates the specialized typed-tool requirement and expands injection surface.
2. A single generic vector-memory bucket was rejected: it cannot represent decision/failure authority, lifecycle or provenance.
3. V2 features before V1 evaluation/recovery were rejected: they amplify unproven state and side-effect risks.
4. A benchmark score embedded in model capabilities was rejected: objective sourced capability and internal evaluation are intentionally separate.
5. A single post-270 monolithic PR was rejected: the extension queue uses 75 evidence-bound cards with contract-first dependencies.

## Open conditions that cannot be planned away

- PR-270 has no current GitHub PR/evidence identity, so implementation entry remains `BLOCKED`.
- Current runtime capabilities must be re-audited at the actual PR-270 merge SHA; this avoids assuming planned PRs became sufficient implementations.
- OS/process/sandbox claims require cross-platform evidence at the card that claims support.

## Conclusion

The 75-card decomposition is defensible only with the entry gate, immutable authority/provenance, resolver-based evidence, no-write replay/shadow, independent review, and V1-before-V2 barrier. Removing any of those controls reopens a critical adversarial finding.
