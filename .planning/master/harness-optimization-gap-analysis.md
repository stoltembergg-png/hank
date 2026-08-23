# Harness Optimization Gap Analysis

## Scope

This is a planning-only audit of the current repository and the existing post-270 plans. It is not implementation evidence and does not promote any planned capability to PASS.

## Existing / partial / new capability map

| Requested capability | Existing evidence | Classification | Extension decision |
|---|---|---|---|
| Native Skill Registry | `agent-core/src/skill.rs` has a permissive manifest/lifecycle seed; `agent-runtime/src/skill_runtime.rs` is a stub. | PARTIAL | Extend via structured first-party contract, immutable registry snapshot and lifecycle pointer; do not replace the seed without baseline evidence. |
| Policy / Skill / Tool separation | Existing tool evaluator/registry and autonomy policy; skill seed contains capabilities. | PARTIAL | Formalize three-way boundary and negative enforcement; no duplicate permission evaluator. |
| Skill Router | Contract map plans `SkillRouteDecision` at PR-315. | PLANNED | Add event/state router contract, deterministic mappings, run pins and E2E after PR-315 baseline. |
| Official skill set | No first-party official profiles evidenced in Hank runtime. | ABSENT | Add exactly ten V1 profiles in two bounded cards, not a general marketplace. |
| Adversarial planning | Historical Hyperplan review exists only as planning evidence. | PARTIAL/PLANNED | Add bounded reviewer contracts, finding normalization and reconciliation; do not depend on HyperPlan runtime. |
| Native eval suite | Existing PR-323/326 plan EvaluationCase/BaselineReport. | PLANNED | Add benchmark corpus, metric schema, baseline/candidate/holdout protocol and evidence-fabrication evals. |
| External evaluator | Better Harness review is documentary; no runtime adapter evidenced. | ABSENT | Add optional adapter/importer after native eval baseline; unavailable adapter is `NO_PROOF`. |
| Skill benchmarking | No skill-version comparison contract evidenced. | ABSENT | Add comparison protocol with comparable-environment requirement and regression thresholds. |
| Improvement candidates | Existing PR-334 plans ImprovementCandidate. | PLANNED | Extend with friction/frequency evidence and required eval refs; no autonomous patching. |
| Shadow execution | ADR-HAR-004 and PR-332 plan no-write shadow. | PLANNED | Add exact no-effect enforcement and side-by-side comparison contract. |
| Promotion/rollback | ADR-HAR-005 requires baseline, approval, activation and rollback. | PLANNED | Specify lifecycle states, atomic pointers and run pin compatibility. |
| Meta-Harness | Existing experiment plan/ADR only. | PLANNED | Place only after evaluation + shadow/promotion evidence; no production mutation route. |
| Observability | Trace/usage/redaction metadata exists in plans. | PARTIAL | Add optimization projections/metrics, never chain-of-thought. |
| E2E coverage | Test Platform PR-356..376 plans E2E, fixtures, safety corpus and release validation. | PLANNED | New E2Es depend on PR-376 and reuse its fixture/virtual-tool boundaries. |

## Hard gaps

0. **Authority reconciliation is missing.** PR-377 must produce a canonical ADR→card→contract→gate map because the master plan's proposed ADR titles/ranges and the currently present ADR-HAR documents are planning artifacts with potentially divergent names. No downstream card may infer a canonical contract from title similarity alone.

1. No structured first-party skill contract binds schemas, allowed tools, policy, budget, max rounds, tests/evals and rollback as one immutable digest.
2. No event-to-skill router is evidenced as deterministic and run-pinned.
3. No bounded adversarial planning pipeline has severity, dedupe, disagreement and `HUMAN_REQUIRED` contracts.
4. No native benchmark corpus or controlled baseline/candidate/holdout comparison is evidenced.
5. No optional external harness importer binds findings to evaluator version, SHA/tree and internal evidence state.
6. No evidence-based friction frequency signal is separated from candidate activation.
7. No shadow zero-side-effect E2E or atomic promotion/rollback proof exists.

## Non-duplication constraints

- Reuse planned `SkillRouteDecision`, `VerifierResult`, `EvaluationCase`, `BaselineReport`, `ShadowRun` and `ImprovementCandidate`; this extension adds required fields/protocols only after baseline compatibility audit.
- Reuse existing `ProjectId`, `RunId`, `TraceId`, budget/evaluator, tool registry, checkpoint and evidence identities.
- Do not introduce an external Better Harness or HyperPlan runtime dependency.
- Do not create a second policy engine, a second tool runtime, raw prompt store, automatic activation path, or direct production meta-mutation path.
