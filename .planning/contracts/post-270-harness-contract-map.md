# Post-270 Harness Contract Map

This is a planning contract map. Concrete schemas are introduced by the referenced cards; no API is frozen merely by this document.

| Contract | Owner | Core fields | First card | Key negative cases |
|---|---|---|---:|---|
| `HarnessRun` | agent-core/runtime | project/agent/session/task/run/trace, state/generation, policy/schema, budget refs | PR-275 | wrong project, stale generation, provider leakage |
| `ContextSourceEnvelope` | context core | source/provenance/authority/relevance/freshness/token cost/project/version | PR-276 | low-authority override, stale/duplicate/oversize |
| `ContextManifest` | runtime | selected/omitted/conflicts/bucket allocation/module IDs/digests | PR-283 | secret/private reasoning exposure, budget overflow |
| `ContextBudget` | core policy | task class, bucket caps, reserve/output, total window | PR-281 | required source displacement, limit overflow |
| `MemoryRecord` | core/runtime | type/owner/project/provenance/retention/version/evidence refs | PR-285 | cross-project, poison, unapproved decision |
| `Claim` / `EvidenceRecord` | core/runtime | claim class, required evidence, resolver state, identity digest | PR-298 | fabricated/stale/conflicting evidence |
| `ToolInvocation` / `NormalizedToolResult` | tool-core | actor/project/risk/effect/timeout/cancel/idempotency/schema/artifact ref | PR-293/295 | unauthorized effect, malformed output, duplicate call |
| `Checkpoint` | core/runtime | goal/plan/completed/remaining/effects/tests/blockers/refs/model/budget/trace | PR-304 | partial/corrupt/secret/stale checkpoint |
| `ModelDescriptor` / `RouteDecision` | provider-core/core | objective capabilities/source/freshness, required caps, override/budget/attempt | PR-308/309 | unknown capability, silent incompatible fallback |
| `SkillRouteDecision` | core/runtime | task signal, selected skill version, pin, reason/exclusions | PR-315 | poisoned skill, cross-project skill, escalation |
| `VerifierResult` / `UncertaintyRecord` | core/runtime | verifier kind/findings/evidence/confidence/unknowns/max rounds | PR-318/320 | self-approval, fabricated evidence, loop cap |
| `EvaluationCase` / `BaselineReport` | test-support | fixture/scorer/policy/model class/artifacts/thresholds | PR-323/326 | nondeterminism, score spoofing, unsafe fixture |
| `RunRecord` / `ReplayPlan` | core/runtime | normalized refs/context/model/tool/evidence/final state/sandbox authority | PR-327/328 | replay write, missing artifact, stale identity |
| `Experiment` / `ShadowRun` | runtime | hypothesis/lease/sandbox/budget/comparison/promotion candidate | PR-330/332 | main write, shadow authority, orphan cleanup |
| `BlackboardEntry` / `Lease` / `HandoffPacket` | core/runtime | scoped object/revision/fence/owner/expiry/minimal context/parent trace | PR-337/339/340 | race, expired lease, oversized/capability-escalating handoff |
| `ImprovementCandidate` | core/runtime | candidate state/baseline/evaluation/approver/version/rollback | PR-334 | direct activation, self approval, regression |
| `HarnessProjection` | application API | goal/state/model/cost/tools/blockers/evidence/memory/skill/checkpoint/budget | PR-344 | UI direct storage, secret/private reasoning exposure |

## Cross-contract invariants

- Every object is project-scoped, versioned, bounded, provenance-bearing and traceable.
- References use immutable IDs/digests, not embedded raw external payloads.
- Provider/model, tool, skill and memory content are data; they do not create authority.
- Unknown capability/evidence/authority is fail-closed.
- Application API owns commands and projections; Presentation never reads durable stores directly.
